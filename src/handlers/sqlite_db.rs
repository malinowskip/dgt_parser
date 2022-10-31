use anyhow::{bail, Result};
use regex::Regex;
use rusqlite::{params, params_from_iter, Connection, ParamsFromIter};
use std::collections::HashMap;

use crate::tmx_parser::TranslationUnit;
use crate::types::{RequestedLangs, TranslationUnitHandler};

/// How many translation units to insert in one batch.
const TRANSACTION_SIZE: usize = 20_000;

pub struct Handler {
    /// SQLite connection.
    conn: Connection,

    /// Keeps track of language columns that are already in the database to
    /// determine if a new column should be added.
    language_columns_in_db: Vec<String>,

    /// Keeps track of document IDs (names) that are already in the database to
    /// determine if a new document should be added.
    docs_in_db: HashMap<String, u32>,

    /// Current batch of translation unit insert queries, which will be executed
    /// in the next transaction.
    queries: Vec<(String, ParamsFromIter<Vec<String>>)>,

    /// Config value provided by the user. Determines if a text in a given
    /// language should be included in the output or skipped.
    requested_langs: RequestedLangs,

    /// Used to validate language codes (used a database columns).
    valid_lang_codes: Vec<String>,
}

impl TranslationUnitHandler for Handler {
    fn handle(&mut self, translation_unit: TranslationUnit, sequential_number_in_doc: u32) {
        self.handle_translation_unit(translation_unit, sequential_number_in_doc)
            .unwrap();
    }
}

impl Handler {
    pub fn new(conn: rusqlite::Connection, requested_langs: RequestedLangs) -> Handler {
        let handler = Handler {
            conn,
            language_columns_in_db: Vec::new(),
            queries: Vec::new(),
            docs_in_db: HashMap::new(),
            requested_langs,
            valid_lang_codes: Vec::new(),
        };
        handler.setup();
        handler
    }

    fn setup(&self) -> () {
        self.drop_table_if_exists();
        self.set_up_schema();
    }

    fn drop_table_if_exists(&self) -> () {
        let query = format!("DROP TABLE IF EXISTS translation_units");
        self.conn.execute(&query, []).unwrap();
    }

    fn set_up_schema(&self) -> () {
        let queries = vec![
            format!(
                "
            CREATE TABLE IF NOT EXISTS translation_units (
                id INTEGER PRIMARY KEY,
                document_id INTEGER,
                sequential_number NUMBER
            )"
            ),
            format!(
                "
            CREATE TABLE IF NOT EXISTS documents (
                    id INTEGER PRIMARY KEY,
                    name TEXT
            )"
            ),
        ];

        for query in queries {
            self.conn
                .execute(&query, [])
                .expect("error setting up dgt table");
        }
    }

    fn add_lang_column(&mut self, column: &String) -> Result<()> {
        let query = format!("ALTER TABLE translation_units ADD COLUMN {}", &column);
        self.conn
            .execute(&query, [])
            .expect("Failed to add new column to database.");
        self.language_columns_in_db.push(column.clone());

        Ok(())
    }

    fn handle_translation_unit(
        &mut self,
        tu: TranslationUnit,
        sequential_number_in_doc: u32,
    ) -> Result<()> {
        self.insert_document(&tu)?;
        let query = self.create_translation_unit_insert_query(&tu, sequential_number_in_doc)?;
        self.queries.push(query);
        if self.queries.len() > TRANSACTION_SIZE {
            self.commit_translation_units()?;
        }

        Ok(())
    }

    fn create_translation_unit_insert_query(
        &mut self,
        tu: &TranslationUnit,
        sequential_number_in_doc: u32,
    ) -> Result<(String, ParamsFromIter<Vec<String>>)> {
        let doc_name = match tu.doc_name() {
            Some(doc) => doc.to_string(),
            None => bail!("Error: no document ID provided for the translation segment."),
        };

        #[derive(Clone)]
        enum StringOrNumberValue {
            StringValue(String),
            NumberValue(u32),
        }

        #[derive(Clone)]
        struct InsertMap {
            column: String,
            value: StringOrNumberValue,
        }

        let mut insert_map: Vec<InsertMap> = Vec::new();

        for el in &tu.segments {
            if !self.lang_is_eligible(&el.lang) {
                continue;
            }

            let lang_code = self.lang_code_to_db_column(&el.lang)?;

            if !&self.language_columns_in_db.contains(&lang_code) {
                self.add_lang_column(&lang_code)?;
            }

            insert_map.push(InsertMap {
                column: lang_code,
                value: StringOrNumberValue::StringValue(el.content.clone()),
            });
        }

        insert_map.push(InsertMap {
            column: String::from("sequential_number"),
            value: StringOrNumberValue::NumberValue(sequential_number_in_doc),
        });

        insert_map.push(InsertMap {
            column: String::from("document_id"),
            value: StringOrNumberValue::NumberValue(*self.docs_in_db.get(&doc_name).unwrap()),
        });

        let columns: Vec<String> = insert_map
            .clone()
            .iter()
            .map(|el: &InsertMap| el.column.clone())
            .collect();

        let values: Vec<String> = insert_map
            .iter()
            .map(|el: &InsertMap| match &el.value {
                StringOrNumberValue::StringValue(v) => format!("{}", v),
                StringOrNumberValue::NumberValue(v) => format!("{}", v),
            })
            .collect();

        // e.g.: `INSERT INTO translation_units (en_gb,pl_01) VALUES (?,?);`
        let query = format!(
            "INSERT INTO translation_units ({}) VALUES ({});",
            columns.join(","),
            repeat_vars(*&values.len())
        );
        let params = params_from_iter(values);

        Ok((query, params))
    }

    /// Take the current batch of queries and commit them into the database.
    fn commit_translation_units(&mut self) -> Result<()> {
        let tx = self.conn.transaction()?;
        for query in &self.queries {
            tx.execute(&query.0, query.1.clone())?;
        }
        tx.commit()?;
        self.queries.clear();

        Ok(())
    }

    /// If the translation unit is the child of a document that doesn’t yet
    /// exist in the database, insert the document into the database.
    fn insert_document(&mut self, translation_unit: &TranslationUnit) -> Result<()> {
        if let Some(doc_name) = translation_unit.doc_name() {
            if let None = self.docs_in_db.get(&doc_name) {
                let mut query = self
                    .conn
                    .prepare("INSERT INTO documents (name) VALUES (?)")?;
                query.execute(params![&doc_name])?;
                let id: u32 = self.conn.query_row(
                    "SELECT id FROM documents WHERE name = ?",
                    params![doc_name],
                    |row| Ok(row.get(0)),
                )??;

                self.docs_in_db.insert(doc_name.clone(), id);
            };
        }

        Ok(())
    }

    /// Determine if the text in a language should be included in the output.
    fn lang_is_eligible(&mut self, lang_code: &String) -> bool {
        match &self.requested_langs {
            RequestedLangs::Unlimited => true,
            RequestedLangs::Each(langs) | RequestedLangs::Some(langs) => langs.contains(lang_code),
        }
    }

    /// Convert the language code according to the following pattern so that it
    /// can be used as a column name in the database:
    ///
    /// - `EN-GB` => `en_gb`
    /// - `PL-01` => `pl_01`
    fn lang_code_to_db_column(&mut self, lang_code: &str) -> Result<String> {
        let lang_code = lang_code.to_ascii_lowercase().replace("-", "_");
        if self.valid_lang_codes.contains(&lang_code) {
            return Ok(lang_code);
        } else {
            let lang_code_regex = Regex::new(r"^\w{2}(-|_)(\w|\d){2}$")?;
            if lang_code_regex.is_match(&lang_code) {
                self.valid_lang_codes.push(lang_code.clone());
                Ok(lang_code)
            } else {
                bail!("Error: invalid language code: {}.", lang_code);
            }
        }
    }
}

impl Drop for Handler {
    fn drop(&mut self) {
        self.commit_translation_units().unwrap();
    }
}

/// Helper function to return a comma-separated sequence of `?`. See
/// [Source](https://docs.rs/rusqlite/latest/rusqlite/struct.ParamsFromIter.html#realistic-use-case)
///
/// - `repeat_vars(0) => panic!(...)`
/// - `repeat_vars(1) => "?"`
/// - `repeat_vars(2) => "?,?"`
/// - `repeat_vars(3) => "?,?,?"`
/// - ...
///
fn repeat_vars(count: usize) -> String {
    assert_ne!(count, 0);
    let mut s = "?,".repeat(count);
    // Remove trailing comma
    s.pop();
    s
}

#[cfg(test)]
mod test {
    use std::path::PathBuf;

    use anyhow::Result;

    use crate::{
        functions::{for_each_tmx_file_in_zip, for_each_zip, read_utf16_file_to_string},
        tmx_parser::{parse_tmx, Tmx},
        types::TranslationUnitHandler,
    };

    use super::Handler;

    fn setup() -> Handler {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        let langs = crate::types::RequestedLangs::Unlimited;
        let mut handler = Handler::new(conn, langs);
        let input_dir = PathBuf::from("./test_data/zipped");
        let mut parsed_translation_units = 0;
        let mut parsed_tmx_files = 0;
        for_each_zip(&input_dir, &mut |mut zip_archive| {
            for_each_tmx_file_in_zip(&mut zip_archive, &mut |mut tmx_file| {
                parsed_tmx_files += 1;
                let tmx_contents = read_utf16_file_to_string(&mut tmx_file)?;
                let Tmx { body, header: _ } = parse_tmx(tmx_contents)?;
                for (i, tu) in body.translation_units.into_iter().enumerate() {
                    parsed_translation_units += 1;
                    handler.handle(tu, i as u32);
                }
                Ok(())
            })?;

            Ok(())
        })
        .unwrap();
        handler.commit_translation_units().unwrap();

        handler
    }

    fn query_number(handler: &mut Handler, query: &str) -> Result<u32> {
        let result = handler
            .conn
            .query_row(query, [], |row| {
                let value: u32 = row.get(0).unwrap();
                Ok(value)
            })
            .unwrap();

        Ok(result)
    }

    #[test]
    fn correct_number_of_translation_units_parsed() -> Result<()> {
        let mut handler = setup();
        let query = "select count(*) from translation_units";
        let translation_unit_count = query_number(&mut handler, query)?;

        assert_eq!(translation_unit_count, 462);

        Ok(())
    }

    #[test]
    fn number_of_docs_and_translation_units_per_doc_checks_out() -> Result<()> {
        let mut handler = setup();

        let documents = [
            ("22019D0557", 20),
            ("22019D0558", 22),
            ("22019D0559", 21),
            ("22019D0391", 25),
            ("22019D0437", 143),
            ("22019D0438", 212),
            ("22019D0556", 19),
        ];

        for (name, expected_count) in documents {
            let query = format!(
                "
                select 
                   count(*)
                from
                    translation_units tu
                join documents d on tu.document_id = d.id
                where d.name = '{}'
                ",
                name
            );
            let actual_count = query_number(&mut handler, &query)?;
            assert_eq!(actual_count, expected_count);
        }

        Ok(())
    }

    #[test]
    fn english_text_of_each_translation_unit_is_identical_to_tmx() {
        let mut english_texts: Vec<String> = Vec::new();
        let input_dir = PathBuf::from("./test_data/zipped");
        for_each_zip(&input_dir, &mut |mut zip_archive| {
            for_each_tmx_file_in_zip(&mut zip_archive, &mut |mut tmx_file| {
                let tmx_contents = read_utf16_file_to_string(&mut tmx_file)?;
                let Tmx { body, header: _ } = parse_tmx(tmx_contents)?;
                for (_i, tu) in body.translation_units.into_iter().enumerate() {
                    for segment in tu.segments {
                        if segment.lang == "EN-GB" {
                            english_texts.push(segment.content);
                        }
                    }
                }
                Ok(())
            })?;

            Ok(())
        })
        .unwrap();

        assert_eq!(english_texts.len(), 462);
        let handler = setup();
        let mut query = handler
            .conn
            .prepare("select en_gb from translation_units")
            .unwrap();
        let english_texts_in_db: Vec<String> = query
            .query_map([], |row| {
                let s: String = row.get(0).unwrap();
                Ok(s)
            })
            .unwrap()
            .map(|el| el.unwrap())
            .collect();

        for (i, text) in english_texts.into_iter().enumerate() {
            assert_eq!(text, english_texts_in_db.get(i).unwrap().to_string());
        }
    }
}
