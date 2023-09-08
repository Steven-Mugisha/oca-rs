use super::Facade;
use crate::data_storage::Namespace;
use crate::{
    data_storage::DataStorage,
    repositories::{OCABundleCacheRepo, OCABundleFTSRepo},
};
use oca_bundle::build::OCABuildStep;
use oca_bundle::state::oca::{capture_base::CaptureBase, OCABundle};

use std::rc::Rc;

use convert_case::{Case, Casing};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct SearchResult {
    #[serde(rename = "r")]
    pub records: Vec<SearchRecord>,
    #[serde(rename = "m")]
    pub metadata: SearchMetadata,
}

#[derive(Debug, Serialize)]
pub struct SearchRecord {
    pub oca_bundle: OCABundle,
    pub metadata: SearchRecordMetadata,
}

#[derive(Debug, Serialize)]
pub struct SearchRecordMetadata {
    pub phrase: String,
    pub scope: String,
    pub score: f32,
}

#[derive(Debug, Serialize)]
pub struct SearchMetadata {
    pub total: usize,
    pub page: usize,
}

impl Facade {
    pub fn search_oca_bundle(
        &self,
        language: Option<isolang::Language>,
        query: String,
        limit: usize,
        page: usize,
    ) -> SearchResult {
        let oca_bundle_fts_repo =
            OCABundleFTSRepo::new(Rc::clone(&self.connection));
        let search_result =
            oca_bundle_fts_repo.search(language, query, limit, page);
        let records = search_result
            .records
            .iter()
            .map(|record| SearchRecord {
                oca_bundle: self
                    .get_oca_bundle(record.oca_bundle_said.clone())
                    .unwrap(),
                metadata: SearchRecordMetadata {
                    phrase: record.metadata.phrase.clone(),
                    scope: record.metadata.scope.clone(),
                    score: record.metadata.score,
                },
            })
            .collect();
        SearchResult {
            records,
            metadata: SearchMetadata {
                total: search_result.metadata.total,
                page: search_result.metadata.page,
            },
        }
    }

    pub fn fetch_all_oca_bundle(
        &self,
        limit: usize,
    ) -> Result<Vec<OCABundle>, Vec<String>> {
        let mut oca_bundles = vec![];
        let mut errors = vec![];

        let oca_bundle_cache_repo =
            OCABundleCacheRepo::new(Rc::clone(&self.connection));
        let oca_bundle_cache_records =
            oca_bundle_cache_repo.fetch_all(limit as i32);
        for oca_bundle_cache_record in oca_bundle_cache_records {
            match serde_json::from_str(&oca_bundle_cache_record.oca_bundle) {
                Ok(oca_bundle) => {
                    oca_bundles.push(oca_bundle);
                }
                Err(e) => {
                    errors.push(format!("Failed to parse oca bundle: {}", e));
                }
            }
        }
        if !errors.is_empty() {
            return Err(errors);
        }

        Ok(oca_bundles)
    }

    pub fn fetch_all_capture_base(
        &self,
        limit: usize,
    ) -> Result<Vec<CaptureBase>, Vec<String>> {
        let mut capture_bases = vec![];
        let mut errors = vec![];

        let capture_base_cache_repo =
            crate::repositories::CaptureBaseCacheRepo::new(Rc::clone(
                &self.connection,
            ));
        let capture_base_cache_records =
            capture_base_cache_repo.fetch_all(limit as i32);
        for capture_base_cache_record in capture_base_cache_records {
            match serde_json::from_str(&capture_base_cache_record.capture_base)
            {
                Ok(capture_base) => {
                    capture_bases.push(capture_base);
                }
                Err(e) => {
                    errors.push(format!("Failed to parse capture base: {}", e));
                }
            }
        }
        if !errors.is_empty() {
            return Err(errors);
        }

        Ok(capture_bases)
    }

    pub fn get_oca_bundle(&self, said: String) -> Result<OCABundle, Vec<String>> {
        let r = self.db.get(Namespace::OCAJsonCache, &format!("oca.{}", said)).map_err(|e| vec![format!("{}", e)])?;
        let oca_bundle_str = String::from_utf8(
            r.ok_or_else(|| vec![format!("No OCA Bundle found for said: {}", said)])?
        ).unwrap();
        serde_json::from_str(&oca_bundle_str)
            .map_err(|e| vec![format!("Failed to parse oca bundle: {}", e)])
    }

    pub fn get_oca_bundle_steps(&self, said: String) -> Result<Vec<OCABuildStep>, Vec<String>> {
        let mut said = said;
        #[allow(clippy::borrowed_box)]
        fn extract_operation(db: &Box<dyn DataStorage>, said: &String) -> Result<(String, oca_ast::ast::Command), Vec<String>> {
            let r = db.get(Namespace::OCA, &format!("oca.{}.operation", said))
                .map_err(|e| vec![format!("{}", e)])?
                .ok_or_else(|| vec![format!("No history found for said: {}", said)])?;

            let said_length = r.first().unwrap();
            let parent_said = String::from_utf8_lossy(&r[1..*said_length as usize + 1]).to_string();
            let op_length = r[*said_length as usize + 1];
            let op = String::from_utf8_lossy(&r[*said_length as usize + 2..*said_length as usize + 2 + op_length as usize]).to_string();

            Ok((
                parent_said,
                serde_json::from_str(&op).unwrap()
            ))
        }

        let mut history: Vec<OCABuildStep> = vec![];

        loop {
            let (parent_said, command) = extract_operation(&self.db, &said)?;
            if parent_said == said {
                dbg!("Malformed history for said: {}", said);
                return Err(vec![format!("Malformed history")]);
            }
            history.push(
                OCABuildStep {
                    parent_said: parent_said.clone().parse().ok(),
                    command,
                    result: self.get_oca_bundle(said.clone()).unwrap(),
                }
            );
            said = parent_said;

            if said.is_empty() {
                break;
            }
        };
        history.reverse();
        Ok(history)
    }

    pub fn get_oca_bundle_ocafile(&self, said: String) -> Result<String, Vec<String>> {
        let mut steps = self.get_oca_bundle_steps(said)?;
        let mut ocafile = String::new();

        steps.iter_mut().for_each(|step| {
            let mut line = String::new();

            if let oca_ast::ast::CommandType::Add = step.command.kind {
                line.push_str("ADD ");
                match &step.command.object_kind {
                    oca_ast::ast::ObjectKind::CaptureBase => {
                        if let Some(ref content) = step.command.content {
                            if let Some(ref attributes) = content.attributes {
                                line.push_str("ATTRIBUTE ");
                                attributes.iter().for_each(|(key, value)| {
                                    if let oca_ast::ast::NestedValue::Value(value) = value {
                                        line.push_str(format!("{}={} ", key, value).as_str());
                                    }
                                });
                            }
                        };
                    },
                    oca_ast::ast::ObjectKind::Overlay(o_type) => {
                        match o_type {
                            oca_ast::ast::OverlayType::Meta => {
                                line.push_str("META ");
                                if let Some(ref mut content) = step.command.content {
                                    if let Some(ref mut properties) = content.properties {
                                        if let Some(
                                            oca_ast::ast::NestedValue::Value(lang)
                                        ) = properties.remove("lang") {
                                            line.push_str(format!("{} ", lang).as_str());
                                        }
                                        if !properties.is_empty() {
                                            line.push_str("PROPS ");
                                            properties.iter().for_each(|(key, value)| {
                                                if let oca_ast::ast::NestedValue::Value(value) = value {
                                                    line.push_str(format!("{}=\"{}\" ", key, value).as_str());
                                                }
                                            });
                                        }
                                    }
                                };
                            },
                            oca_ast::ast::OverlayType::Unit => {
                                line.push_str("UNIT ");
                                if let Some(ref mut content) = step.command.content {
                                    if let Some(ref mut properties) = content.properties {
                                        if let Some(
                                            oca_ast::ast::NestedValue::Value(unit_system)
                                        ) = properties.remove("unit_system") {
                                            line.push_str(format!("{} ", unit_system).as_str());
                                        }
                                        if !properties.is_empty() {
                                            line.push_str("PROPS ");
                                            properties.iter().for_each(|(key, value)| {
                                                if let oca_ast::ast::NestedValue::Value(value) = value {
                                                    line.push_str(format!("{}=\"{}\" ", key, value).as_str());
                                                }
                                            });
                                        }
                                        if let Some(ref attributes) = content.attributes {
                                            line.push_str("ATTRS ");
                                            attributes.iter().for_each(|(key, value)| {
                                                if let oca_ast::ast::NestedValue::Value(value) = value {
                                                    line.push_str(format!("{}=\"{}\" ", key, value).as_str());
                                                }
                                            });
                                        }
                                    }
                                };
                            },
                            oca_ast::ast::OverlayType::EntryCode => {
                                line.push_str("ENTRY_CODE ");
                                if let Some(ref mut content) = step.command.content {
                                    if let Some(ref mut properties) = content.properties {
                                        if !properties.is_empty() {
                                            line.push_str("PROPS ");
                                            properties.iter().for_each(|(key, value)| {
                                                if let oca_ast::ast::NestedValue::Value(value) = value {
                                                    line.push_str(format!("{}={} ", key, value).as_str());
                                                }
                                            });
                                        }
                                        if let Some(ref attributes) = content.attributes {
                                            line.push_str("ATTRS ");
                                            attributes.iter().for_each(|(key, value)| {
                                                if let oca_ast::ast::NestedValue::Array(values) = value {
                                                    let codes = values.iter().filter_map(|value| {
                                                        if let oca_ast::ast::NestedValue::Value(value) = value {
                                                            Some(format!("\"{}\"", value))
                                                        } else {
                                                            None
                                                        }
                                                    }).collect::<Vec<String>>().join(", ");
                                                    line.push_str(format!("{}=[{}] ", key, codes).as_str());
                                                }
                                            });
                                        }
                                    }
                                };
                            },
                            oca_ast::ast::OverlayType::Entry => {
                                line.push_str("ENTRY ");
                                if let Some(ref mut content) = step.command.content {
                                    if let Some(ref mut properties) = content.properties {
                                        if let Some(
                                            oca_ast::ast::NestedValue::Value(lang)
                                        ) = properties.remove("lang") {
                                            line.push_str(format!("{} ", lang).as_str());
                                        }
                                        if !properties.is_empty() {
                                            line.push_str("PROPS ");
                                            properties.iter().for_each(|(key, value)| {
                                                if let oca_ast::ast::NestedValue::Value(value) = value {
                                                    line.push_str(format!("{}={} ", key, value).as_str());
                                                }
                                            });
                                        }
                                        if let Some(ref attributes) = content.attributes {
                                            line.push_str("ATTRS ");
                                            attributes.iter().for_each(|(key, value)| {
                                                if let oca_ast::ast::NestedValue::Object(values) = value {
                                                    let codes = values.iter().filter_map(|(code, label)| {
                                                        if let oca_ast::ast::NestedValue::Value(label) = label {
                                                            Some(format!("\"{}\": \"{}\"", code, label))
                                                        } else {
                                                            None
                                                        }
                                                    }).collect::<Vec<String>>().join(", ");
                                                    line.push_str(format!("{}={{ {} }} ", key, codes).as_str());
                                                }
                                            });
                                        }
                                    }
                                };
                            },
                            _ => {
                                line.push_str(
                                    format!(
                                        "{} ",
                                        o_type.to_string().to_case(Case::UpperSnake)
                                    ).as_str()
                                );

                                if let Some(ref mut content) = step.command.content {
                                    if let Some(ref mut properties) = content.properties {
                                        if let Some(
                                            oca_ast::ast::NestedValue::Value(lang)
                                        ) = properties.remove("lang") {
                                            line.push_str(format!("{} ", lang).as_str());
                                        }
                                        if !properties.is_empty() {
                                            line.push_str("PROPS ");
                                            properties.iter().for_each(|(key, value)| {
                                                if let oca_ast::ast::NestedValue::Value(value) = value {
                                                    line.push_str(format!("{}=\"{}\" ", key, value).as_str());
                                                }
                                            });
                                        }
                                    }
                                    if let Some(ref attributes) = content.attributes {
                                        line.push_str("ATTRS ");
                                        attributes.iter().for_each(|(key, value)| {
                                            if let oca_ast::ast::NestedValue::Value(value) = value {
                                                line.push_str(format!("{}=\"{}\" ", key, value).as_str());
                                            }
                                        });
                                    }
                                };
                            }
                        }
                    },
                    _ => {}
                }
            }

            ocafile.push_str(format!("{}\n", line).as_str());
        });

        Ok(ocafile)
    }
}
