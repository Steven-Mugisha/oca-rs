use crate::state::oca::OCABundle;
use crate::state::oca::overlay::unit::{ Unit, AttributeUnit, MeasurementSystem, MeasurementUnit, ImperialUnit, MetricUnit };
#[cfg(feature = "format_overlay")]
use crate::state::oca::overlay::format::Formats;
use crate::state::oca::overlay::entry::Entries;
use crate::state::oca::overlay::entry_code::EntryCodes;
use crate::state::oca::overlay::cardinality::Cardinalitys;
use crate::state::oca::overlay::conformance::Conformances;
use crate::state::oca::overlay::information::Information;
use crate::state::oca::overlay::character_encoding::CharacterEncodings;
use crate::state::oca::overlay::meta::Metas;
use crate::state::oca::overlay::label::Labels;
use std::str::FromStr;
use std::collections::HashMap;
use oca_ast::ast;
use crate::state::{oca::OCABox, encoding::Encoding, attribute::{Attribute, AttributeType}, entry_codes::EntryCodes as EntryCodesValue, entries::EntriesElement};

#[derive(Debug)]
pub struct OCABuild {
    pub oca_bundle: OCABundle,
    pub steps: Vec<OCABuildStep>,
}

#[derive(Debug)]
pub struct OCABuildStep {
    pub parent_said: Option<said::SelfAddressingIdentifier>,
    pub command: ast::Command,
    pub result: OCABundle,
}

pub fn from_ast(from_oca: Option<OCABundle>, oca_ast: ast::OCAAst) -> Result<OCABuild, Vec<String>> {
    let mut errors = vec![];
    let mut steps = vec![];
    let mut base: Option<OCABox> = from_oca.map(OCABox::from);
    let mut parent_said: Option<said::SelfAddressingIdentifier> = None;
    for (line, command) in oca_ast.commands.iter().enumerate() {
        match apply_command(base.clone(), command.clone()) {
            Ok(oca_box) => {
                let mut oca_box_mut = oca_box.clone();
                let oca_bundle = oca_box_mut.generate_bundle();
                steps.push(
                    OCABuildStep {
                        parent_said: parent_said.clone(),
                        command: command.clone(),
                        result: oca_bundle.clone(),
                    }
                );
                parent_said = oca_bundle.said.clone();
                base = Some(oca_box);
            }
            Err(mut err) => {
                errors.extend(err.iter_mut().map(|e|
                    format!("Error at step {}: {}", line + 1, e)
                ));
            }
        }
    }
    if errors.is_empty() {
        Ok(
            OCABuild {
                oca_bundle: base.unwrap().generate_bundle(),
                steps,
            }
        )
    } else {
        Err(errors)
    }
}

fn apply_command(base: Option<OCABox>, op: ast::Command) -> Result<OCABox, Vec<String>> {
    let mut errors = vec![];
    let mut oca: OCABox = match base {
        Some(oca) => oca,
        None => OCABox::new()
    };

    match op.kind {
        ast::CommandType::From => {
            errors.push("Unsupported FROM command".to_string());
        },
        ast::CommandType::Add => {
            match op.object_kind {
                ast::ObjectKind::CaptureBase => {
                    if let Some(ref content) = op.content {
                        if let Some(ref attributes) = content.attributes {
                            for (attr_name, attr_type_value) in attributes {
                                let mut attribute = Attribute::new(attr_name.clone());
                                if let ast::NestedValue::Value(attr_value) = attr_type_value {
                                    match AttributeType::from_str(attr_value) {
                                        Ok(attribute_type) => attribute.set_attribute_type(attribute_type),
                                        Err(_) => errors.push(format!("Unknown attribute ({attr_name}) type: {attr_value}"))
                                    }
                                    oca.add_attribute(attribute);
                                }
                            }
                        }
                        if let Some(ref properties) = content.properties {
                            for (prop_name, prop_value) in properties {
                                if prop_name.eq("classification") {
                                    if let ast::NestedValue::Value(value) = prop_value {
                                        oca.add_classification(value.clone());
                                    }
                                }
                            }
                        }
                    }
                },
                ast::ObjectKind::Overlay(ref overlay_type) => {
                    match overlay_type {
                        ast::OverlayType::Meta => {
                            if let Some(ref content) = op.content {
                                if let Some(ref properties) = content.properties {
                                    let mut mut_properties = properties.clone();
                                    let lang = mut_properties.remove("lang");
                                    let mut lang_iso = None;
                                    if let Some(ast::NestedValue::Value(lang_str)) = lang {
                                        lang_iso = isolang::Language::from_639_1(lang_str.as_str());
                                    }

                                    for (prop_name, prop_value) in mut_properties {
                                        if let ast::NestedValue::Value(value) = prop_value {
                                            oca.add_meta(lang_iso.unwrap(), prop_name.clone(), value.clone());
                                        }
                                    }
                                }
                            }
                        },
                        ast::OverlayType::Label => {
                            if let Some(ref content) = op.content {
                                let mut lang_iso = None;
                                if let Some(ref properties) = content.properties {
                                    let mut mut_properties = properties.clone();
                                    let lang = mut_properties.remove("lang");
                                    if let Some(ast::NestedValue::Value(lang_str)) = lang {
                                        lang_iso = isolang::Language::from_639_1(lang_str.as_str());
                                    }
                                }
                                if let Some(ref attributes) = content.attributes {
                                    for (attr_name, attr_type_value) in attributes {
                                        let mut attribute = oca.attributes
                                            .get(attr_name)
                                            .ok_or_else(|| {
                                                errors.push(format!("Undefined attribute: {attr_name}"));
                                                errors.clone()
                                            })?.clone();
                                        if let ast::NestedValue::Value(attr_label) = attr_type_value {
                                            attribute.set_label(lang_iso.unwrap(), attr_label.clone());
                                        }
                                        oca.add_attribute(attribute);
                                    }
                                }
                            }
                        },
                        ast::OverlayType::Information => {
                            if let Some(ref content) = op.content {
                                let mut lang_iso = None;
                                if let Some(ref properties) = content.properties {
                                    let mut mut_properties = properties.clone();
                                    let lang = mut_properties.remove("lang");
                                    if let Some(ast::NestedValue::Value(lang_str)) = lang {
                                        lang_iso = isolang::Language::from_639_1(lang_str.as_str());
                                    }
                                }
                                if let Some(ref attributes) = content.attributes {
                                    for (attr_name, attr_type_value) in attributes {
                                        let mut attribute = oca.attributes
                                            .get(attr_name)
                                            .ok_or_else(|| {
                                                errors.push(format!("Undefined attribute: {attr_name}"));
                                                errors.clone()
                                            })?.clone();
                                        if let ast::NestedValue::Value(attr_info) = attr_type_value {
                                            attribute.set_information(lang_iso.unwrap(), attr_info.clone());
                                        }
                                        oca.add_attribute(attribute);
                                    }
                                }
                            }
                        },
                        ast::OverlayType::CharacterEncoding => {
                            if let Some(ref content) = op.content {
                                if let Some(ref attributes) = content.attributes {
                                    for (attr_name, attr_type_value) in attributes {
                                        let mut attribute = oca.attributes
                                            .get(attr_name)
                                            .ok_or_else(|| {
                                                errors.push(format!("Undefined attribute: {attr_name}"));
                                                errors.clone()
                                            })?.clone();
                                        if let ast::NestedValue::Value(attr_encoding) = attr_type_value {
                                            attribute.set_encoding(
                                                Encoding::from_str(attr_encoding)
                                                    .map_err(|_| {
                                                        errors.push(format!("Unknown encoding: {attr_encoding}"));
                                                        errors.clone()
                                                    })?
                                            );
                                        }
                                        oca.add_attribute(attribute);
                                    }
                                }
                            }
                        },
                        ast::OverlayType::Conformance => {
                            if let Some(ref content) = op.content {
                                if let Some(ref attributes) = content.attributes {
                                    for (attr_name, attr_type_value) in attributes {
                                        let mut attribute = oca.attributes
                                            .get(attr_name)
                                            .ok_or_else(|| {
                                                errors.push(format!("Undefined attribute: {attr_name}"));
                                                errors.clone()
                                            })?.clone();

                                        if let ast::NestedValue::Value(attr_conformance) = attr_type_value {
                                            attribute.set_conformance(attr_conformance.clone());
                                        }
                                        oca.add_attribute(attribute);
                                    }
                                }
                            }
                        },
                        ast::OverlayType::Format => {
                            #[cfg(feature = "format_overlay")]
                            {
                            if let Some(ref content) = op.content {
                                if let Some(ref attributes) = content.attributes {
                                    for (attr_name, attr_type_value) in attributes {
                                        let mut attribute = oca.attributes
                                            .get(attr_name)
                                            .ok_or_else(|| {
                                                errors.push(format!("Undefined attribute: {attr_name}"));
                                                errors.clone()
                                            })?.clone();
                                        if let ast::NestedValue::Value(attr_format) = attr_type_value {
                                            attribute.set_format(attr_format.clone());
                                        }
                                        oca.add_attribute(attribute);
                                    }
                                }
                            }
                            }
                        }
                        ast::OverlayType::Unit => {
                            if let Some(ref content) = op.content {
                                let mut unit_system_op = None;
                                if let Some(ref properties) = content.properties {
                                    let mut mut_properties = properties.clone();
                                    let unit_system_prop = mut_properties.remove("unit_system");
                                    if let Some(ast::NestedValue::Value(unit_system_str)) = unit_system_prop {
                                        unit_system_op = MeasurementSystem::from_str(&unit_system_str).ok();
                                    }
                                }
                                if let Some(unit_system) = unit_system_op {
                                    if let Some(ref attributes) = content.attributes {
                                        for (attr_name, attr_type_value) in attributes {
                                            let mut attribute = oca.attributes
                                                .get(attr_name)
                                                .ok_or_else(|| {
                                                    errors.push(format!("Undefined attribute: {attr_name}"));
                                                    errors.clone()
                                                })?.clone();
                                            if let ast::NestedValue::Value(attr_unit) = attr_type_value {

                                                let unit = match unit_system {
                                                        MeasurementSystem::Metric => {
                                                            Some(MeasurementUnit::Metric(MetricUnit::from_str(attr_unit).unwrap_or_else(|_| panic!("{}", "Invalid metric unit: {attr_unit}"))))
                                                        },
                                                        MeasurementSystem::Imperial => {
                                                            Some(MeasurementUnit::Imperial(ImperialUnit::from_str(attr_unit).unwrap_or_else(|_| panic!("{}", "Invalid imperial unit: {attr_unit}"))))
                                                        }
                                                    };
                                                attribute.set_unit(
                                                    AttributeUnit {
                                                        measurement_system: unit_system.clone(),
                                                        unit: unit.unwrap()
                                                    }
                                                );
                                            }
                                            oca.add_attribute(attribute);
                                        }
                                    }
                                }
                            }
                        }
                        ast::OverlayType::Cardinality => {
                            if let Some(ref content) = op.content {
                                if let Some(ref attributes) = content.attributes {
                                    for (attr_name, attr_type_value) in attributes {
                                        let mut attribute = oca.attributes
                                            .get(attr_name)
                                            .ok_or_else(|| {
                                                errors.push(format!("Undefined attribute: {attr_name}"));
                                                errors.clone()
                                            })?.clone();
                                        if let ast::NestedValue::Value(attr_cardinality) = attr_type_value {
                                            attribute.set_cardinality(attr_cardinality.clone());
                                        }
                                        oca.add_attribute(attribute);
                                    }
                                }
                            }
                        },
                        ast::OverlayType::EntryCode => {
                            if let Some(ref content) = op.content {
                                if let Some(ref attributes) = content.attributes {
                                    for (attr_name, attr_type_value) in attributes {
                                        let mut attribute = oca.attributes
                                            .get(attr_name)
                                            .ok_or_else(|| {
                                                errors.push(format!("Undefined attribute: {attr_name}"));
                                                errors.clone()
                                            })?.clone();
                                        match attr_type_value {
                                            ast::NestedValue::Value(attr_entry_codes_sai) => {
                                                attribute.set_entry_codes(EntryCodesValue::Sai(attr_entry_codes_sai.clone()));
                                            },
                                            ast::NestedValue::Array(attr_entry_codes) => {
                                                let mut entry_codes: Vec<String> = vec![];
                                                for attr_entry_code in attr_entry_codes {
                                                    if let ast::NestedValue::Value(entry_code) = attr_entry_code {
                                                        entry_codes.push(entry_code.clone());
                                                    }
                                                }
                                                attribute.set_entry_codes(EntryCodesValue::Array(entry_codes));
                                            },
                                            _ => ()

                                        }
                                        oca.add_attribute(attribute);
                                    }
                                }
                            }
                        },
                        ast::OverlayType::Entry => {
                            if let Some(ref content) = op.content {
                                let mut lang_iso = None;
                                if let Some(ref properties) = content.properties {
                                    let mut mut_properties = properties.clone();
                                    let lang = mut_properties.remove("lang");
                                    if let Some(ast::NestedValue::Value(lang_str)) = lang {
                                        lang_iso = isolang::Language::from_639_1(lang_str.as_str());
                                    }
                                }
                                if let Some(ref attributes) = content.attributes {
                                    for (attr_name, attr_type_value) in attributes {
                                        let mut attribute = oca.attributes
                                            .get(attr_name)
                                            .ok_or_else(|| {
                                                errors.push(format!("Undefined attribute: {attr_name}"));
                                                errors.clone()
                                            })?.clone();
                                        match attr_type_value {
                                            ast::NestedValue::Value(attr_entries) => {
                                                attribute.set_entry(lang_iso.unwrap(), EntriesElement::Sai(attr_entries.clone()));
                                            }
                                            ast::NestedValue::Object(attr_entries) => {
                                                let mut entries = HashMap::new();
                                                for (attr_entry_key, attr_entry_value) in attr_entries {
                                                    if let ast::NestedValue::Value(entry_value) = attr_entry_value {
                                                        entries.insert(attr_entry_key.clone(), entry_value.clone());
                                                    }
                                                }
                                                attribute.set_entry(lang_iso.unwrap(), EntriesElement::Object(entries));
                                            },
                                            _ => ()
                                        }
                                        oca.add_attribute(attribute);
                                    }
                                }
                            }
                        }
                        _ => ()
                    }
                },
                _ => ()
            }
        },
        _ => ()
    }

    if errors.is_empty() {
        Ok(oca)
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::IndexMap;
    use said::version::Encode;

    #[test]
    fn test_add_step() {
        let mut commands = vec![];

        let mut attributes = IndexMap::new();
        attributes.insert("d".to_string(), ast::NestedValue::Value("Text".to_string()));
        attributes.insert("i".to_string(), ast::NestedValue::Value("Text".to_string()));
        attributes.insert("passed".to_string(), ast::NestedValue::Value("Boolean".to_string()));
        commands.push(
            ast::Command {
                kind: ast::CommandType::Add,
                object_kind: ast::ObjectKind::CaptureBase,
                content: Some(ast::Content {
                    attributes: Some(attributes),
                    properties: None,
                }),
            }
        );

        let mut properties = IndexMap::new();
        properties.insert("lang".to_string(), ast::NestedValue::Value("en".to_string()));
        properties.insert("name".to_string(), ast::NestedValue::Value("Entrance credential".to_string()));
        properties.insert("description".to_string(), ast::NestedValue::Value("Entrance credential".to_string()));
        commands.push(
            ast::Command {
                kind: ast::CommandType::Add,
                object_kind: ast::ObjectKind::Overlay(ast::OverlayType::Meta),
                content: Some(ast::Content {
                    attributes: None,
                    properties: Some(properties),
                }),
            }
        );

        let mut attributes = IndexMap::new();
        attributes.insert("d".to_string(), ast::NestedValue::Value("Schema digest".to_string()));
        attributes.insert("i".to_string(), ast::NestedValue::Value("Credential Issuee".to_string()));
        attributes.insert("passed".to_string(), ast::NestedValue::Value("Passed".to_string()));
        let mut properties = IndexMap::new();
        properties.insert("lang".to_string(), ast::NestedValue::Value("en".to_string()));
        commands.push(
            ast::Command {
                kind: ast::CommandType::Add,
                object_kind: ast::ObjectKind::Overlay(ast::OverlayType::Label),
                content: Some(ast::Content {
                    attributes: Some(attributes),
                    properties: Some(properties),
                }),
            }
        );

        let mut attributes = IndexMap::new();
        attributes.insert("d".to_string(), ast::NestedValue::Value("Schema digest".to_string()));
        attributes.insert("i".to_string(), ast::NestedValue::Value("Credential Issuee".to_string()));
        attributes.insert("passed".to_string(), ast::NestedValue::Value("Enables or disables passing".to_string()));
        let mut properties = IndexMap::new();
        properties.insert("lang".to_string(), ast::NestedValue::Value("en".to_string()));
        commands.push(
            ast::Command {
                kind: ast::CommandType::Add,
                object_kind: ast::ObjectKind::Overlay(ast::OverlayType::Information),
                content: Some(ast::Content {
                    attributes: Some(attributes),
                    properties: Some(properties),
                }),
            }
        );

        let mut attributes = IndexMap::new();
        attributes.insert("d".to_string(), ast::NestedValue::Value("utf-8".to_string()));
        attributes.insert("i".to_string(), ast::NestedValue::Value("utf-8".to_string()));
        attributes.insert("passed".to_string(), ast::NestedValue::Value("utf-8".to_string()));
        commands.push(
            ast::Command {
                kind: ast::CommandType::Add,
                object_kind: ast::ObjectKind::Overlay(ast::OverlayType::CharacterEncoding),
                content: Some(ast::Content {
                    attributes: Some(attributes),
                    properties: None,
                }),
            }
        );

        let mut attributes = IndexMap::new();
        attributes.insert("d".to_string(), ast::NestedValue::Value("M".to_string()));
        attributes.insert("i".to_string(), ast::NestedValue::Value("M".to_string()));
        attributes.insert("passed".to_string(), ast::NestedValue::Value("M".to_string()));
        commands.push(
            ast::Command {
                kind: ast::CommandType::Add,
                object_kind: ast::ObjectKind::Overlay(ast::OverlayType::Conformance),
                content: Some(ast::Content {
                    attributes: Some(attributes),
                    properties: None,
                }),
            }
        );

        let mut base: Option<OCABox> = None;
        for command in commands {
            match apply_command(base.clone(), command.clone()) {
                Ok(oca) => {
                    base = Some(oca);
                },
                Err(errors) => {
                    println!("{:?}", errors);
                }
            }
            // let mut oca = apply_command(base, command);
            // println!("{}", serde_json::to_string_pretty(&oca.generate_bundle()).unwrap());
            // base = Some(oca);
        }
    }

    #[test]
    fn build_from_ast() {
        let mut commands = vec![];

        let mut attributes = IndexMap::new();
        attributes.insert("d".to_string(), ast::NestedValue::Value("Text".to_string()));
        attributes.insert("i".to_string(), ast::NestedValue::Value("Text".to_string()));
        attributes.insert("passed".to_string(), ast::NestedValue::Value("Boolean".to_string()));
        commands.push(
            ast::Command {
                kind: ast::CommandType::Add,
                object_kind: ast::ObjectKind::CaptureBase,
                content: Some(ast::Content {
                    attributes: Some(attributes),
                    properties: None,
                }),
            }
        );

        let mut properties = IndexMap::new();
        properties.insert("lang".to_string(), ast::NestedValue::Value("en".to_string()));
        properties.insert("name".to_string(), ast::NestedValue::Value("Entrance credential".to_string()));
        properties.insert("description".to_string(), ast::NestedValue::Value("Entrance credential".to_string()));
        commands.push(
            ast::Command {
                kind: ast::CommandType::Add,
                object_kind: ast::ObjectKind::Overlay(ast::OverlayType::Meta),
                content: Some(ast::Content {
                    attributes: None,
                    properties: Some(properties),
                }),
            }
        );

        let mut attributes = IndexMap::new();
        attributes.insert("d".to_string(), ast::NestedValue::Value("Schema digest".to_string()));
        attributes.insert("i".to_string(), ast::NestedValue::Value("Credential Issuee".to_string()));
        attributes.insert("passed".to_string(), ast::NestedValue::Value("Passed".to_string()));
        let mut properties = IndexMap::new();
        properties.insert("lang".to_string(), ast::NestedValue::Value("en".to_string()));
        commands.push(
            ast::Command {
                kind: ast::CommandType::Add,
                object_kind: ast::ObjectKind::Overlay(ast::OverlayType::Label),
                content: Some(ast::Content {
                    attributes: Some(attributes),
                    properties: Some(properties),
                }),
            }
        );

        let mut attributes = IndexMap::new();
        attributes.insert("d".to_string(), ast::NestedValue::Value("Schema digest".to_string()));
        attributes.insert("i".to_string(), ast::NestedValue::Value("Credential Issuee".to_string()));
        attributes.insert("passed".to_string(), ast::NestedValue::Value("Enables or disables passing".to_string()));
        let mut properties = IndexMap::new();
        properties.insert("lang".to_string(), ast::NestedValue::Value("en".to_string()));
        commands.push(
            ast::Command {
                kind: ast::CommandType::Add,
                object_kind: ast::ObjectKind::Overlay(ast::OverlayType::Information),
                content: Some(ast::Content {
                    attributes: Some(attributes),
                    properties: Some(properties),
                }),
            }
        );

        let mut attributes = IndexMap::new();
        attributes.insert("d".to_string(), ast::NestedValue::Value("utf-8".to_string()));
        attributes.insert("i".to_string(), ast::NestedValue::Value("utf-8".to_string()));
        attributes.insert("passed".to_string(), ast::NestedValue::Value("utf-8".to_string()));
        commands.push(
            ast::Command {
                kind: ast::CommandType::Add,
                object_kind: ast::ObjectKind::Overlay(ast::OverlayType::CharacterEncoding),
                content: Some(ast::Content {
                    attributes: Some(attributes),
                    properties: None,
                }),
            }
        );

        let mut attributes = IndexMap::new();
        attributes.insert("d".to_string(), ast::NestedValue::Value("M".to_string()));
        attributes.insert("i".to_string(), ast::NestedValue::Value("M".to_string()));
        attributes.insert("passed".to_string(), ast::NestedValue::Value("M".to_string()));
        commands.push(
            ast::Command {
                kind: ast::CommandType::Add,
                object_kind: ast::ObjectKind::Overlay(ast::OverlayType::Conformance),
                content: Some(ast::Content {
                    attributes: Some(attributes),
                    properties: None,
                }),
            }
        );

        let oca_ast = ast::OCAAst {
            version: "1.0".to_string(),
            commands,
        };

        let build_result = from_ast(None, oca_ast);
        match build_result {
            Ok(oca_build) => {
                let oca_bundle_encoded = oca_build.oca_bundle.encode().unwrap();
                let oca_bundle_json = String::from_utf8(oca_bundle_encoded).unwrap();
                println!("{}", oca_bundle_json);
            },
            Err(e) => {
                println!("{:?}", e);
            }
        }
    }
}
