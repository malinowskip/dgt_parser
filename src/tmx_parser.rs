use std::collections::HashMap;

use crate::types::IncludedLangs;
use anyhow::Result;
use quick_xml::de::{from_str, DeError};
use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq)]
pub struct Tmx {
    pub header: Header,
    pub body: Body,
}

/// The header of a TMX document may contain metadata.
#[derive(Debug, Deserialize, PartialEq)]
pub struct Header {
    #[serde(flatten)]
    pub attributes: HashMap<String, String>,
}

/// The body of a TMX document contains a collection of translation units.
#[derive(Debug, Deserialize, PartialEq)]
pub struct Body {
    #[serde(rename = "tu")]
    pub translation_units: Vec<TranslationUnit>,
}

/// Contains the translations of a text in a set of languages.
#[derive(Debug, Deserialize, PartialEq)]
pub struct TranslationUnit {
    #[serde(rename = "prop", default)]
    pub props: Vec<Prop>,
    #[serde(rename = "tuv", default)]
    pub segments: Vec<Tuv>,
}

/// The `prop` element defines metadata. In the context of the DGT-Translation
/// Memory, this element is used to specify the name/id of the EU legislation
/// that a given translation unit comes from.
/// ## Example
/// ```xml
/// <tu>
///     <prop type="Txt::Doc. No.">22019A0315(01)</prop>
///     <tuv lang="EN-GB">
///         <seg>Agreement</seg>
///     </tuv>
///     <tuv lang="DE-DE">
///         <seg>ÜBERSETZUNG</seg>
///     </tuv>
///     ...
/// </tu>
/// ```
#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct Prop {
    #[serde(rename = "type")]
    pub key: String,
    #[serde(rename = "$value")]
    pub value: String,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Tuv {
    #[serde(alias = "lang", default)]
    #[serde(alias = "xml:lang")]
    pub lang: String,
    #[serde(rename = "seg", default)]
    pub content: String,
}

/// Deserialize an XML string into a [Tmx] struct.
pub fn parse_tmx(xml_string: String) -> Result<Tmx, DeError> {
    from_str(&xml_string)
}

impl TranslationUnit {
    /// Name/ID of EU legislation associated with the translation unit.
    pub fn doc_name(&self) -> Option<String> {
        let name_props = &self
            .props
            .iter()
            .filter(|el| el.key == "Txt::Doc. No.")
            .collect::<Vec<&Prop>>();

        return match name_props.get(0) {
            Some(name) => Some(name.value.clone()),
            None => None,
        };
    }

    /// Checks whether the translation unit contains texts in **each** of the
    /// specified languages.
    pub fn contains_each_lang(&self, langs: &IncludedLangs) -> bool {
        return match langs.clone() {
            IncludedLangs::Unlimited => true,
            IncludedLangs::Each(langs) | IncludedLangs::Some(langs) => {
                langs.iter().fold(true, |acc, lang| {
                    if !acc {
                        return false;
                    }

                    for segment in &self.segments {
                        if &segment.lang == lang {
                            return true;
                        }
                    }

                    false
                })
            }
        };
    }

    /// Checks whether the translation unit contains texts in **any** of the
    /// specified languages.
    pub fn contains_any_lang(&self, langs: &IncludedLangs) -> bool {
        return match langs.clone() {
            IncludedLangs::Unlimited => true,
            IncludedLangs::Each(langs) | IncludedLangs::Some(langs) => {
                for lang in langs {
                    for segment in &self.segments {
                        if segment.lang == lang {
                            return true;
                        }
                    }
                }
                return false;
            }
        };
    }
}
