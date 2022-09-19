use crate::state::{attribute::Attribute, oca::Overlay};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::any::Any;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UnitOverlay {
    capture_base: String,
    #[serde(rename = "type")]
    overlay_type: String,
    pub metric_system: String,
    pub attribute_units: BTreeMap<String, String>,
}

impl Overlay for UnitOverlay {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn capture_base(&mut self) -> &mut String {
        &mut self.capture_base
    }
    fn overlay_type(&self) -> &String {
        &self.overlay_type
    }
    fn metric_system(&self) -> Option<&String> {
        Some(&self.metric_system)
    }
    fn attributes(&self) -> Vec<&String> {
        self.attribute_units.keys().collect::<Vec<&String>>()
    }

    fn add(&mut self, attribute: &Attribute) {
        if attribute.unit.is_some() {
            self.attribute_units.insert(
                attribute.name.clone(),
                attribute.unit.as_ref().unwrap().clone(),
            );
        }
    }
}
impl UnitOverlay {
    pub fn new(metric_system: String) -> Box<UnitOverlay> {
        Box::new(UnitOverlay {
            capture_base: String::new(),
            overlay_type: "spec/overlays/unit/1.0".to_string(),
            metric_system,
            attribute_units: BTreeMap::new(),
        })
    }
}
