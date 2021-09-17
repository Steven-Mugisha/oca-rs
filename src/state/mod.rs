use core::str::FromStr;

use said::derivation::SelfAddressing;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

mod capture_base;
mod overlay;
use crate::state::capture_base::CaptureBase;
use crate::state::overlay::Overlay;

#[derive(Serialize)]
pub struct OCA {
    pub capture_base: CaptureBase,
    pub overlays: Vec<Box<dyn Overlay>>,
    #[serde(skip)]
    translations: HashMap<Language, OCATranslation>,
}

impl OCA {
    pub fn new(default_encoding: Encoding) -> OCA {
        OCA {
            capture_base: CaptureBase::new(),
            overlays: vec![overlay::CharacterEncoding::new(&default_encoding)],
            translations: HashMap::new(),
        }
    }

    pub fn add_name(mut self, names: HashMap<Language, String>) -> OCA {
        for (lang, name) in names.iter() {
            match self.translations.get_mut(lang) {
                Some(t) => {
                    t.add_name(name.clone());
                }
                None => {
                    let mut tr = OCATranslation::new();
                    tr.add_name(name.clone());
                    self.translations.insert(*lang, tr);
                }
            }
        }
        self
    }

    pub fn add_description(mut self, descriptions: HashMap<Language, String>) -> OCA {
        for (lang, description) in descriptions.iter() {
            match self.translations.get_mut(lang) {
                Some(t) => {
                    t.add_description(description.clone());
                }
                None => {
                    let mut tr = OCATranslation::new();
                    tr.add_description(description.clone());
                    self.translations.insert(*lang, tr);
                }
            }
        }
        self
    }

    pub fn add_attribute(mut self, attr: Attribute) -> OCA {
        self.capture_base.add(&attr);

        if attr.encoding.is_some() {
            let encoding_ov = self
                .overlays
                .iter_mut()
                .find(|x| x.overlay_type().contains("/character_encoding/"));
            if let Some(ov) = encoding_ov {
                ov.add(&attr);
            }
        }

        if attr.format.is_some() {
            let mut format_ov = self
                .overlays
                .iter_mut()
                .find(|x| x.overlay_type().contains("/format/"));
            if format_ov.is_none() {
                self.overlays.push(overlay::Format::new());
                format_ov = self.overlays.last_mut();
            }

            if let Some(ov) = format_ov {
                ov.add(&attr)
            }
        }

        if attr.unit.is_some() {
            let mut unit_ov = self
                .overlays
                .iter_mut()
                .find(|x| x.overlay_type().contains("/unit/"));
            if unit_ov.is_none() {
                self.overlays.push(overlay::Unit::new());
                unit_ov = self.overlays.last_mut();
            }

            if let Some(ov) = unit_ov {
                ov.add(&attr)
            }
        }

        if attr.entry_codes.is_some() {
            let mut entry_code_ov = self
                .overlays
                .iter_mut()
                .find(|x| x.overlay_type().contains("/entry_code/"));
            if entry_code_ov.is_none() {
                self.overlays.push(overlay::EntryCode::new());
                entry_code_ov = self.overlays.last_mut();
            }

            if let Some(ov) = entry_code_ov {
                ov.add(&attr)
            }
        }

        for (lang, attr_tr) in attr.translations.iter() {
            let mut label_ov = self.overlays.iter_mut().find(|x| {
                if let Some(o_lang) = x.language() {
                    return o_lang == lang && x.overlay_type().contains("/label/");
                }
                false
            });
            if label_ov.is_none() {
                self.overlays.push(overlay::Label::new(lang));
                label_ov = self.overlays.last_mut();
            }
            if let Some(ov) = label_ov {
                ov.add(&attr);
            }

            if attr_tr.information.is_some() {
                let mut information_ov = self.overlays.iter_mut().find(|x| {
                    if let Some(o_lang) = x.language() {
                        return o_lang == lang && x.overlay_type().contains("/character_encoding/");
                    }
                    false
                });
                if information_ov.is_none() {
                    self.overlays.push(overlay::Information::new(lang));
                    information_ov = self.overlays.last_mut();
                }
                if let Some(ov) = information_ov {
                    ov.add(&attr);
                }
            }

            if attr_tr.entries.is_some() {
                let mut entry_ov = self.overlays.iter_mut().find(|x| {
                    if let Some(o_lang) = x.language() {
                        return o_lang == lang && x.overlay_type().contains("/entry/");
                    }
                    false
                });
                if entry_ov.is_none() {
                    self.overlays.push(overlay::Entry::new(lang));
                    entry_ov = self.overlays.last_mut();
                }
                if let Some(ov) = entry_ov {
                    ov.add(&attr);
                }
            }
        }
        self
    }

    pub fn finalize(mut self) -> OCA {
        for (lang, translation) in self.translations.iter() {
            self.overlays.push(overlay::Meta::new(lang, translation));
        }

        let cs_json = serde_json::to_string(&self.capture_base).unwrap();
        let sai = format!("{}", SelfAddressing::Blake3_256.derive(cs_json.as_bytes()));
        for o in self.overlays.iter_mut() {
            o.sign(&sai);
        }
        self
    }
}

#[derive(Serialize, Deserialize)]
pub struct Attribute {
    name: String,
    attr_type: AttributeType,
    is_pii: bool,
    translations: HashMap<Language, AttributeTranslation>,
    encoding: Option<Encoding>,
    format: Option<String>,
    unit: Option<String>,
    entry_codes: Option<Vec<String>>,
}

impl Attribute {
    pub fn new(name: String, attr_type: AttributeType) -> Attribute {
        Attribute {
            name,
            attr_type,
            is_pii: false,
            translations: HashMap::new(),
            encoding: None,
            format: None,
            unit: None,
            entry_codes: None,
        }
    }

    pub fn set_pii(mut self) -> Attribute {
        self.is_pii = true;
        self
    }

    pub fn add_encoding(mut self, encoding: Encoding) -> Attribute {
        self.encoding = Some(encoding);
        self
    }

    pub fn add_format(mut self, format: String) -> Attribute {
        self.format = Some(format);
        self
    }

    pub fn add_unit(mut self, unit: String) -> Attribute {
        self.unit = Some(unit);
        self
    }

    pub fn add_label(mut self, labels: HashMap<Language, String>) -> Attribute {
        for (lang, label) in labels.iter() {
            match self.translations.get_mut(lang) {
                Some(t) => {
                    t.add_label(label.clone());
                }
                None => {
                    let mut tr = AttributeTranslation::new();
                    tr.add_label(label.clone());
                    self.translations.insert(*lang, tr);
                }
            }
        }
        self
    }

    pub fn add_entries(mut self, entries: Vec<Entry>) -> Attribute {
        let mut entry_codes = vec![];

        for entry in entries.iter() {
            entry_codes.push(entry.id.clone());

            for (lang, en) in entry.translations.iter() {
                match self.translations.get_mut(lang) {
                    Some(t) => {
                        t.add_entry(entry.id.clone(), en.clone());
                    }
                    None => {
                        let mut tr = AttributeTranslation::new();
                        tr.add_entry(entry.id.clone(), en.clone());
                        self.translations.insert(*lang, tr);
                    }
                }
            }
        }
        self.entry_codes = Some(entry_codes);

        self
    }

    pub fn add_information(mut self, information: HashMap<Language, String>) -> Attribute {
        for (lang, info) in information.iter() {
            match self.translations.get_mut(lang) {
                Some(t) => {
                    t.add_information(info.clone());
                }
                None => {
                    let mut tr = AttributeTranslation::new();
                    tr.add_information(info.clone());
                    self.translations.insert(*lang, tr);
                }
            }
        }
        self
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
pub struct OCATranslation {
    name: Option<String>,
    description: Option<String>,
}

impl Default for OCATranslation {
    fn default() -> Self {
        Self::new()
    }
}

impl OCATranslation {
    pub fn new() -> OCATranslation {
        OCATranslation {
            name: None,
            description: None,
        }
    }

    pub fn add_name(&mut self, name: String) -> &mut OCATranslation {
        self.name = Some(name);
        self
    }

    pub fn add_description(&mut self, description: String) -> &mut OCATranslation {
        self.description = Some(description);
        self
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AttributeTranslation {
    label: Option<String>,
    entries: Option<HashMap<String, String>>,
    information: Option<String>,
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
            self.entries = Some(HashMap::new());
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
    Text,
    Number,
    Date,
    #[serde(rename = "Array[Text]")]
    ArrayText,
}

#[wasm_bindgen]
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum Language {
    #[serde(rename = "en_EN")]
    En,
    #[serde(rename = "en_US")]
    EnUs,
    #[serde(rename = "pl_PL")]
    Pl,
}

impl FromStr for Language {
    type Err = ();

    fn from_str(input: &str) -> Result<Language, Self::Err> {
        match input {
            "0" => Ok(Language::En),
            "En" => Ok(Language::En),
            "en_EN" => Ok(Language::En),
            "1" => Ok(Language::EnUs),
            "EnUs" => Ok(Language::EnUs),
            "en_US" => Ok(Language::EnUs),
            "2" => Ok(Language::Pl),
            "Pl" => Ok(Language::Pl),
            "pl_PL" => Ok(Language::Pl),
            _ => Err(()),
        }
    }
}

#[wasm_bindgen]
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum Encoding {
    #[serde(rename = "utf-8")]
    Utf8,
    #[serde(rename = "iso-8859-1")]
    Iso8859_1,
}

#[cfg(test)]
mod tests {
    use super::*;
    use maplit::hashmap;

    #[test]
    fn test_1() {
        let mut oca = OCA::new(Encoding::Utf8)
            .add_name(hashmap! {
                Language::En => "Driving Licence".to_string(),
                Language::Pl => "Prawo Jazdy".to_string(),
            })
            .add_description(hashmap! {
                Language::En => "DL desc".to_string(),
                Language::Pl => "PJ desc".to_string(),
            });

        let attr1 = Attribute::new(String::from("n1"), AttributeType::Text)
            .set_pii()
            .add_label(hashmap! {
                Language::En => "Name: ".to_string(),
                Language::Pl => "Imię: ".to_string(),
            })
            .add_entries(vec![
                Entry::new(
                    "op1".to_string(),
                    hashmap! {
                        Language::En => "Option 1".to_string(),
                        Language::Pl => "Opcja 1".to_string(),
                    },
                ),
                Entry::new(
                    "op2".to_string(),
                    hashmap! {
                        Language::En => "Option 2".to_string(),
                        Language::Pl => "Opcja 2".to_string(),
                    },
                ),
            ])
            .add_information(hashmap! {
                Language::En => "info en".to_string(),
                Language::Pl => "info pl".to_string(),
            })
            .add_unit("days".to_string());

        let attr2 = Attribute::new(String::from("n2"), AttributeType::Date)
            .add_label(hashmap! {
                Language::En => "Date: ".to_string(),
                Language::Pl => "Data: ".to_string(),
            })
            .add_encoding(Encoding::Iso8859_1)
            .add_format("DD/MM/YYYY".to_string());

        oca = oca.add_attribute(attr1).add_attribute(attr2).finalize();

        println!("{:#?}", serde_json::to_string(&oca).unwrap());

        assert_eq!(oca.capture_base.attributes.len(), 2);
        assert_eq!(oca.capture_base.pii.len(), 1);
    }
}
