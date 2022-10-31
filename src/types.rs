use crate::tmx_parser::TranslationUnit;

/// Passed to the handler to specify which languages should be included in the
/// output. Language codes are in the same format as in the source TMX files,
/// i.e. `EN-GB`, `PL-01`.
#[derive(Clone)]
pub enum RequestedLangs {
    /// 1. Include all languages.
    /// 2. Don’t skip any translation units.
    Unlimited,

    /// 1. Include only the specified languages.
    /// 2. Include translation units that contain **at least one** of the specified languages.
    Some(Vec<String>),

    /// 1. Include only the specified languages.
    /// 2. Include translation units that contain **each** of the specified languages.
    Each(Vec<String>),
}

pub trait TranslationUnitHandler {
    /// Process a [TranslationUnit], e.g. insert it into a database.
    fn handle(&mut self, translation_unit: TranslationUnit, sequential_number_in_doc: u32);
}
