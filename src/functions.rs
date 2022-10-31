use std::fs::File;
use std::io::BufReader;
use std::io::Read;
use std::path::PathBuf;
use zip::read::ZipFile;
use zip::ZipArchive;

use anyhow::{bail, Result};

pub fn parse_utf16_string(input: Vec<u8>) -> Result<String> {
    let (result, malformed_sequences_present) =
        encoding_rs::UTF_16LE.decode_with_bom_removal(&input);
    if malformed_sequences_present {
        bail!("Error decoding input");
    }
    Ok(result.to_string())
}

pub fn read_utf16_file_to_string<T>(file: &mut T) -> Result<String>
where
    T: Read,
{
    let mut buffer: Vec<u8> = Vec::new();
    file.read_to_end(&mut buffer)?;
    let tmx_contents = parse_utf16_string(buffer)?;
    Ok(tmx_contents)
}

/// - `en` => `EN-GB`
/// - `pl` => `PL-01`
/// - `Asdf` => `Asdf`
pub fn coerce_lang_codes(input: Vec<String>) -> Vec<String> {
    input
        .iter()
        .map(|lang_code| coerce_lang_code(lang_code))
        .collect()
}

fn coerce_lang_code(input: &String) -> String {
    match input.to_ascii_lowercase().as_str() {
        "en" => String::from("EN-GB"),
        "pl" => String::from("PL-01"),
        "de" => String::from("DE-DE"),
        "da" => String::from("DA-01"),
        "el" => String::from("EL-01"),
        "es" => String::from("ES-ES"),
        "fi" => String::from("FI-01"),
        "fr" => String::from("FR-FR"),
        "it" => String::from("IT-IT"),
        "nl" => String::from("NL-NL"),
        "pt" => String::from("PT-PT"),
        "sv" => String::from("SV-SE"),
        "lv" => String::from("LV-01"),
        "cs" => String::from("CS-01"),
        "et" => String::from("ET-01"),
        "hu" => String::from("HU-01"),
        "sl" => String::from("SL-01"),
        "lt" => String::from("LT-01"),
        "mt" => String::from("MT-01"),
        "sk" => String::from("SK-01"),
        "ro" => String::from("RO-RO"),
        "bg" => String::from("BG-01"),
        "hr" => String::from("HR-HR"),
        "ga" => String::from("GA-IE"),
        _ => String::from(input),
    }
}

#[test]
fn coercion_leaves_unrecognized_string_intact() {
    assert_eq!(coerce_lang_code(&"en".to_string()), "EN-GB".to_string());
    assert_eq!(coerce_lang_code(&"Hello".to_string()), "Hello".to_string());
}

/// Perform an operation on every ZIP file in the input directory.
pub fn for_each_zip<F>(input_dir: &PathBuf, callback: &mut F) -> Result<()>
where
    F: FnMut(ZipArchive<BufReader<File>>) -> Result<()>,
{
    let zip_files = std::fs::read_dir(input_dir)?;
    for zip_file in zip_files {
        if let Ok(zip_file) = zip_file {
            let f = File::open(zip_file.path())?;
            let reader = BufReader::new(f);
            let zip_archive = zip::ZipArchive::new(reader);
            if let Ok(zip_archive) = zip_archive {
                callback(zip_archive)?;
            }
        }
    }
    Ok(())
}

/// Perform an operation on every TMX file in a ZIP archive.
pub fn for_each_tmx_file_in_zip<F>(
    zip_archive: &mut ZipArchive<BufReader<File>>,
    callback: &mut F,
) -> Result<()>
where
    F: FnMut(ZipFile) -> Result<()>,
{
    for i in 0..zip_archive.len() {
        if let Ok(file) = zip_archive.by_index(i) {
            if file.name().ends_with(".tmx") {
                callback(file)?;
            }
        }
    }

    Ok(())
}
