use std::io::Write;

use crate::tmx_parser::TranslationUnit;

/// How many translation units to insert in one batch.
const INSERT_SIZE: usize = 1_000;

pub struct Handler<T: Write> {
    translation_unit_buffer: Vec<TranslationUnit>,
    output: T,
}
