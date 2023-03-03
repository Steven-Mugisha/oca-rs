use crate::state::{attribute::Attribute, oca::Overlay};
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::collections::HashMap;
use isolang::Language;

pub trait Information {
    fn set_information(&mut self, l: Language, information: String);
}

impl Information for Attribute {
    fn set_information(&mut self, l: Language, information: String) {
        if let Some(informations) = &mut self.informations {
            informations.insert(l, information);
        } else {
            let mut informations = HashMap::new();
            informations.insert(l, information);
            self.informations = Some(informations);
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InformationOverlay {
    capture_base: String,
    said: String,
    #[serde(rename = "type")]
    overlay_type: String,
    language: Language,
    pub attribute_information: HashMap<String, String>,
}

impl Overlay for InformationOverlay {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn capture_base(&self) -> &String {
        &self.capture_base
    }
    fn capture_base_mut(&mut self) -> &mut String {
        &mut self.capture_base
    }
    fn overlay_type(&self) -> &String {
        &self.overlay_type
    }
    fn said(&self) -> &String {
        &self.said
    }
    fn said_mut(&mut self) -> &mut String {
        &mut self.said
    }
    fn language(&self) -> Option<&Language> {
        Some(&self.language)
    }
    fn attributes(&self) -> Vec<&String> {
        self.attribute_information.keys().collect::<Vec<&String>>()
    }

    fn add(&mut self, attribute: &Attribute) {
        if let Some(informations) = &attribute.informations {
            if let Some(value) = informations.get(&self.language) {
                self.attribute_information
                    .insert(attribute.name.clone(), value.to_string());
            }
        }
    }
}
impl InformationOverlay {
    pub fn new(lang: Language) -> InformationOverlay {
        InformationOverlay {
            capture_base: String::new(),
            said: String::from("############################################"),
            overlay_type: "spec/overlays/information/1.0".to_string(),
            language: lang,
            attribute_information: HashMap::new(),
        }
    }
}
