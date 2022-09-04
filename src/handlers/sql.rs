use std::io::Write as FileWrite;
use std::{
    fmt::Write,
    fs::File,
    path::{Path, PathBuf},
};

use anyhow::{bail, Result};

use crate::{tmx_parser::TranslationUnit, types::TranslationUnitHandler};

/// How many translation units to insert in one batch.
const INSERT_SIZE: usize = 20_000;

pub struct Handler {
    incoming_batch: Vec<(TranslationUnit, u32)>,
    langs_in_batch: Vec<String>,
    lang_columns: Vec<String>,
    inserts_file: std::fs::File,
}

impl TranslationUnitHandler for Handler {
    fn handle(&mut self, translation_unit: TranslationUnit, sequential_number_in_doc: u32) {
        for seg in &translation_unit.segments {
            if !&self.langs_in_batch.contains(&seg.lang) {
                self.langs_in_batch.push(seg.lang.clone())
            }
        }
        self.incoming_batch
            .push((translation_unit, sequential_number_in_doc));
        if self.incoming_batch.len() == INSERT_SIZE {
            self.commit_batch().unwrap();
        }
    }
}

impl Handler {
    pub fn new(file_path: &str) -> Result<Self, anyhow::Error> {
        if Path::exists(&PathBuf::from(file_path)) {
            bail!("The target file ({}) already exists!", file_path);
        }

        let mut inserts_file = File::create(file_path)?;

        write!(
            inserts_file,
            "CREATE TABLE translation_units (
                id SERIAL PRIMARY KEY,
                sequential_number INTEGER,
                document_id VARCHAR(255)
            );\n"
        )?;

        let handler = Handler {
            incoming_batch: Vec::default(),
            langs_in_batch: Vec::default(),
            lang_columns: Vec::new(),
            inserts_file,
        };

        Ok(handler)
    }

    fn commit_batch(&mut self) -> Result<()> {
        for lang in &self.langs_in_batch {
            if !&self.lang_columns.contains(lang) {
                write!(
                    self.inserts_file,
                    "ALTER TABLE translation_units ADD COLUMN {} TEXT;\n",
                    lang_code_to_db_column(lang)
                )?;
                self.lang_columns.push(lang.clone());
            }
        }

        // en_gb,pl_01,de_de
        let langs_list: String = self
            .langs_in_batch
            .iter()
            .map(|lang_code| lang_code_to_db_column(lang_code))
            .collect::<Vec<String>>()
            .join(",");

        write!(
            self.inserts_file,
            "INSERT INTO translation_units (sequential_number, document_id, {}) VALUES ",
            langs_list
        )?;

        let mut counter = 0;
        for (tu, seq) in &self.incoming_batch {
            let mut stmt = String::new();
            if counter != 0 {
                stmt.write_char(',')?;
            }
            stmt.write_str("\n")?;
            stmt.write_char('(')?;
            stmt.write_str(&format!("{}, '{}'", seq, tu.doc_name().unwrap()))?;
            for lang in &self.langs_in_batch {
                match tu.get_lang(lang) {
                    Some(tuv) => {
                        let escaped = tuv.content.replace("'", "''");
                        stmt.write_str(&format!(",'{}'", escaped))?;
                    }
                    None => {
                        stmt.write_str(",NULL")?;
                    }
                }
            }
            stmt.write_char(')')?;
            write!(self.inserts_file, "{}", stmt)?;
            counter += 1;
        }

        write!(self.inserts_file, ";\n")?;
        self.incoming_batch.clear();
        self.langs_in_batch.clear();

        Ok(())
    }
}

fn lang_code_to_db_column(lang_code: &str) -> String {
    return lang_code.to_ascii_lowercase().replace("-", "_");
}

impl Drop for Handler {
    fn drop(&mut self) {
        self.commit_batch().unwrap();
    }
}
