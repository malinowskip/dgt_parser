use assert_cmd::prelude::CommandCargoExt;
use assert_fs::{self, TempDir};
use rusqlite::{self, Connection};
use std::{path::PathBuf, process::Command};

fn setup() -> Result<(TempDir, PathBuf), Box<dyn std::error::Error>> {
    let tmp_dir = assert_fs::TempDir::new().unwrap();
    let tmp_path = tmp_dir.path();
    let db_file_path = tmp_path.join("db.sqlite");

    Ok((tmp_dir, db_file_path))
}

fn query_number(conn: &Connection, query: &str) -> Result<u32, Box<dyn std::error::Error>> {
    let result = conn
        .query_row(query, [], |row| {
            let value: u32 = row.get(0).unwrap();
            Ok(value)
        })
        .unwrap();

    Ok(result)
}

#[test]
fn number_of_translation_units_matches() -> Result<(), Box<dyn std::error::Error>> {
    let (_tmp_dir, db_file_path) = setup().unwrap();
    let mut cmd = Command::cargo_bin("dgt_parser").unwrap();
    cmd.args([
        "-l",
        "pl",
        "-l",
        "en",
        "-i",
        "test_data/zipped",
        "sqlite",
        "-o",
        db_file_path.display().to_string().as_str(),
    ]);
    let _output = cmd.output();

    let conn = rusqlite::Connection::open(db_file_path.display().to_string().as_str()).unwrap();

    let count = query_number(&conn, "select count(en_gb) from translation_units").unwrap();

    assert_eq!(count, 462);

    Ok(())
}

#[test]
fn require_each_lang_test() -> Result<(), Box<dyn std::error::Error>> {
    let (_tmp_dir, db_file_path) = setup().unwrap();
    let mut cmd = Command::cargo_bin("dgt_parser").unwrap();
    cmd.args([
        "--require-each-lang",
        "-l",
        "pl",
        "-l",
        "en",
        "-i",
        "test_data/zipped",
        "sqlite",
        "-o",
        db_file_path.display().to_string().as_str(),
    ]);

    let _output = cmd.output();

    let conn = rusqlite::Connection::open(db_file_path.display().to_string().as_str()).unwrap();

    let en_count = query_number(&conn, "select count(en_gb) from translation_units").unwrap();
    let pl_count = query_number(&conn, "select count(pl_01) from translation_units").unwrap();

    assert_eq!(en_count, 440);
    assert_eq!(pl_count, 440);

    Ok(())
}
