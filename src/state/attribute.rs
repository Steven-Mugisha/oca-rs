use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use wasm_bindgen::prelude::*;

use crate::state::{encoding::Encoding, language::Language, entry_codes::EntryCodes};

#[derive(Serialize, Deserialize)]
pub struct Attribute {
    pub name: String,
    pub attr_type: AttributeType,
    pub is_pii: bool,
    pub translations: HashMap<Language, AttributeTranslation>,
    pub encoding: Option<Encoding>,
    pub format: Option<String>,
    pub unit: Option<String>,
    pub entry_codes: Option<EntryCodes>,
}

pub struct AttributeBuilder {
    pub attribute: Attribute,
}

impl AttributeBuilder {
    pub fn new(name: String, attr_type: AttributeType) -> AttributeBuilder {
        AttributeBuilder {
            attribute: Attribute {
                name,
                attr_type,
                is_pii: false,
                translations: HashMap::new(),
                encoding: None,
                format: None,
                unit: None,
                entry_codes: None,
            },
        }
    }

    pub fn set_pii(mut self) -> AttributeBuilder {
        self.attribute.is_pii = true;
        self
    }

    pub fn add_encoding(mut self, encoding: Encoding) -> AttributeBuilder {
        self.attribute.encoding = Some(encoding);
        self
    }

    pub fn add_format(mut self, format: String) -> AttributeBuilder {
        self.attribute.format = Some(format);
        self
    }

    pub fn add_unit(mut self, unit: String) -> AttributeBuilder {
        self.attribute.unit = Some(unit);
        self
    }

    pub fn add_label(mut self, labels: HashMap<Language, String>) -> AttributeBuilder {
        for (lang, label) in labels.iter() {
            match self.attribute.translations.get_mut(lang) {
                Some(t) => {
                    t.add_label(label.clone());
                }
                None => {
                    let mut tr = AttributeTranslation::new();
                    tr.add_label(label.clone());
                    self.attribute.translations.insert(lang.clone(), tr);
                }
            }
        }
        self
    }

    pub fn add_entry_codes(mut self, entry_codes: EntryCodes) -> AttributeBuilder {
        self.attribute.entry_codes = Some(entry_codes);
        self
    }

    pub fn add_entries(mut self, entries: Vec<Entry>) -> AttributeBuilder {
        for entry in entries.iter() {
            for (lang, en) in entry.translations.iter() {
                match self.attribute.translations.get_mut(lang) {
                    Some(t) => {
                        t.add_entry(entry.id.clone(), en.clone());
                    }
                    None => {
                        let mut tr = AttributeTranslation::new();
                        tr.add_entry(entry.id.clone(), en.clone());
                        self.attribute.translations.insert(lang.clone(), tr);
                    }
                }
            }
        }
        self
    }

    pub fn add_information(mut self, information: HashMap<Language, String>) -> AttributeBuilder {
        for (lang, info) in information.iter() {
            match self.attribute.translations.get_mut(lang) {
                Some(t) => {
                    t.add_information(info.clone());
                }
                None => {
                    let mut tr = AttributeTranslation::new();
                    tr.add_information(info.clone());
                    self.attribute.translations.insert(lang.clone(), tr);
                }
            }
        }
        self
    }

    pub fn build(self) -> Attribute {
        self.attribute
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Entry {
    id: String,
    translations: HashMap<Language, String>,
}

impl Entry {
    pub fn new(id: String, translations: HashMap<Language, String>) -> Entry {
        Entry { id, translations }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AttributeTranslation {
    pub label: Option<String>,
    pub entries: Option<BTreeMap<String, String>>,
    pub information: Option<String>,
}

impl Default for AttributeTranslation {
    fn default() -> Self {
        Self::new()
    }
}

impl AttributeTranslation {
    pub fn new() -> AttributeTranslation {
        AttributeTranslation {
            label: None,
            entries: None,
            information: None,
        }
    }

    pub fn add_label(&mut self, label: String) -> &mut AttributeTranslation {
        self.label = Some(label);
        self
    }

    pub fn add_entry(&mut self, id: String, tr: String) -> &mut AttributeTranslation {
        if self.entries.is_none() {
            self.entries = Some(BTreeMap::new());
        }
        if let Some(mut entries) = self.entries.clone() {
            entries.insert(id, tr);
            self.entries = Some(entries);
        }
        self
    }

    pub fn add_information(&mut self, information: String) -> &mut AttributeTranslation {
        self.information = Some(information);
        self
    }
}

#[wasm_bindgen]
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum AttributeType {
    Boolean,
    Text,
    Number,
    Date,
    #[serde(rename = "SAI")]
    Sai,
    #[serde(rename = "Array[Text]")]
    ArrayText,
}
