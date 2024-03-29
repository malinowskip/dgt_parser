mod cli;
mod functions;
mod handlers;
mod tmx_parser;
mod types;

use anyhow::{bail, Result};
use clap::Parser;
use cli::Commands;
use functions::{
    coerce_lang_codes, for_each_tmx_file_in_zip, for_each_zip, read_utf16_file_to_string,
};
use rusqlite;
use std::io::Write;
use std::path::{Path, PathBuf};

use tmx_parser::{parse_tmx, Tmx};
use types::RequestedLangs;

fn main() -> Result<()> {
    let cli = cli::Cli::parse();

    // Total count of TMX documents across the ZIP files in the input
    // directory.
    let total_tmx_files = count_tmx_files(&cli.input_dir)?;

    // Reported back to the user.
    let mut tmx_files_parsed = 0;

    // Allows the user to restrict which languages are included in the output.
    //
    // By default, the output will contain texts in all languages. If language
    // codes are specified, only the specified languages will be included in the
    // output.
    let requested_langs: RequestedLangs = match cli.langs {
        None => RequestedLangs::Unlimited,
        Some(langs) => match cli.require_each_lang {
            true => RequestedLangs::Each(coerce_lang_codes(langs)),
            false => RequestedLangs::Some(coerce_lang_codes(langs)),
        },
    };

    // Saves each translation unit received into the handler’s dedicated output
    // format.
    let mut handler = init_handler(cli.command, requested_langs.clone())?;

    // Keep track of the number of TMX documents parsed and report progress to
    // the user.
    let mut incr_count_and_report_progress = || -> Result<()> {
        tmx_files_parsed += 1;
        let percentage: f32 = (tmx_files_parsed as f32 / total_tmx_files as f32) * 100 as f32;
        print!(
            "\rParsing {} out of {} documents ({:.0}%).",
            tmx_files_parsed, total_tmx_files, percentage
        );
        std::io::stdout().flush()?;

        Ok(())
    };

    for_each_zip(&cli.input_dir, &mut |mut zip_archive| {
        for_each_tmx_file_in_zip(&mut zip_archive, &mut |mut file| {
            incr_count_and_report_progress()?;
            let tmx_contents = read_utf16_file_to_string(&mut file)?;
            let Tmx { body, header: _ } = parse_tmx(tmx_contents)?;
            for (i, tu) in body.translation_units.into_iter().enumerate() {
                if let RequestedLangs::Some(_) = &requested_langs {
                    if !tu.contains_any_lang(&requested_langs) {
                        continue;
                    }
                }
                if let RequestedLangs::Each(_) = &requested_langs {
                    if !tu.contains_each_lang(&requested_langs) {
                        continue;
                    }
                }
                handler.handle(tu, i as u32);
            }

            Ok(())
        })?;

        Ok(())
    })?;

    Ok(())
}

fn init_handler(
    cli_command: Commands,
    requested_langs: RequestedLangs,
) -> Result<Box<dyn types::TranslationUnitHandler>> {
    let handler: Box<dyn types::TranslationUnitHandler> = match cli_command {
        Commands::Sqlite { output_file } => {
            if Path::exists(&PathBuf::from(&output_file)) {
                bail!("Error: {} already exists.", &output_file);
            }
            let conn = rusqlite::Connection::open(output_file)?;
            let handler = Box::new(handlers::sqlite_db::Handler::new(conn, requested_langs));
            handler
        }
    };

    Ok(handler)
}

/// Determine the total number of TMX files across all ZIP archives in the
/// target directory.
fn count_tmx_files(path: &PathBuf) -> Result<u32> {
    let mut counter = 0;
    for_each_zip(path, &mut |zip_archive| {
        let file_names = zip_archive.file_names();
        for file_name in file_names {
            if file_name.ends_with(".tmx") {
                counter += 1;
            }
        }

        Ok(())
    })?;

    Ok(counter)
}
