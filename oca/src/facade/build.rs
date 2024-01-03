use super::Facade;
use crate::data_storage::Namespace;
#[cfg(feature = "local-references")]
use crate::local_references;
use crate::repositories::{
    CaptureBaseCacheRecord, CaptureBaseCacheRepo, OCABundleCacheRecord,
    OCABundleCacheRepo, OCABundleFTSRecord, OCABundleFTSRepo,
};
#[cfg(feature = "local-references")]
use log::debug;
use oca_ast::ast::{ObjectKind, RefValue, ReferenceAttrType};
use oca_bundle::state::oca::OCABundle;
use oca_bundle::Encode;
use oca_dag::build_core_db_model;

use std::rc::Rc;

#[derive(thiserror::Error, Debug, serde::Serialize)]
#[serde(untagged)]
pub enum Error {
    #[error(transparent)]
    OCAFileParse(#[from] oca_file::ocafile::ParseError),
    #[error(transparent)]
    OCABundleBuild(#[from] oca_bundle::build::Error),
    #[error("Error at line {line_number} ({raw_line}): {message}")]
    InvalidCommand {
        #[serde(rename = "ln")]
        line_number: usize,
        #[serde(rename = "c")]
        raw_line: String,
        #[serde(rename = "e")]
        message: String,
    },
}

impl Facade {
    pub fn build_from_ocafile(
        &mut self,
        ocafile: String,
    ) -> Result<OCABundle, Vec<Error>> {
        let mut errors: Vec<Error> = vec![];
        let mut oca_ast = oca_file::ocafile::parse_from_string(ocafile)
            .map_err(|e| vec![Error::OCAFileParse(e)])?;

        let mut base: Option<OCABundle> = None;
        // TODO this should be avoided if the ast is passed for further processing, the base is
        // checked again in generate bundle
        if let Some(first_command) = oca_ast.commands.get(0) {
            if let (oca_ast::ast::CommandType::From, ObjectKind::OCABundle(content)) = (first_command.clone().kind, first_command.clone().object_kind) {
                match content.said {
                    ReferenceAttrType::Reference(refs) => {
                        match refs {
                            RefValue::Said(said) => {
                                match self.get_oca_bundle(said, false) {
                                    Ok(oca_bundle) => {
                                        // TODO
                                        base = Some(oca_bundle.bundle.clone());
                                    },
                                    Err(e) => {
                                        let default_command_meta = oca_ast::ast::CommandMeta { line_number: 0, raw_line: "unknown".to_string() };
                                        let command_meta = oca_ast.commands_meta.get(&0).unwrap_or(&default_command_meta);
                                        e.iter().for_each(|e| errors.push(
                                            Error::InvalidCommand {
                                                line_number: command_meta.line_number,
                                                raw_line: command_meta.raw_line.clone(),
                                                message: e.clone()
                                            }
                                        ));
                                    }
                                }
                            },
                            RefValue::Name(_) => todo!(),
                        }
                    }
                }
                oca_ast.commands.remove(0);
            }
        }
        if !errors.is_empty() {
            return Err(errors);
        }
        #[cfg(feature = "local-references")]
        let references = self.fetch_all_refs().unwrap();
        #[cfg(feature = "local-references")]
        debug!("References found in local db: {:?}", references);

        // Dereference (refn -> refs) the AST before it start processing bundle steps, otherwise the SAID would
        // not match.
        #[cfg(feature = "local-references")]
        local_references::replace_refn_with_refs(&mut oca_ast, references);

        let oca_build = oca_bundle::build::from_ast(base, &oca_ast).map_err(|e| {
            e.iter().map(|e|
                Error::OCABundleBuild(e.clone())
            ).collect::<Vec<_>>()
        })?;

        #[cfg(feature = "local-references")]
        let schema_name = oca_ast.meta.get("name");
        #[cfg(feature = "local-references")]
        debug!("Schema name found: {:?}", schema_name);

        #[cfg(feature = "local-references")]
        if schema_name.is_some() {
            let schema_name = schema_name.unwrap();
            let said = oca_build.oca_bundle.said.clone().unwrap().to_string();
            self.store_reference(schema_name, said);
        }


        let oca_bundle_cache_repo =
            OCABundleCacheRepo::new(Rc::clone(&self.connection));
        let oca_bundle_cache_record =
            OCABundleCacheRecord::new(&oca_build.oca_bundle);
        oca_bundle_cache_repo.insert(oca_bundle_cache_record);

        let capture_base_cache_repo =
            CaptureBaseCacheRepo::new(Rc::clone(&self.connection));
        let capture_base_cache_record =
            CaptureBaseCacheRecord::new(&oca_build.oca_bundle.capture_base);
        capture_base_cache_repo.insert(capture_base_cache_record);

        let meta_overlays = oca_build
            .oca_bundle
            .overlays
            .iter()
            .filter_map(|x| {
                x.as_any()
                    .downcast_ref::<oca_bundle::state::oca::overlay::Meta>()
            })
            .collect::<Vec<_>>();
        if !meta_overlays.is_empty() {
            let oca_bundle_fts_repo =
                OCABundleFTSRepo::new(Rc::clone(&self.connection));
            for meta_overlay in meta_overlays {
                let oca_bundle_fts_record = OCABundleFTSRecord::new(
                    oca_build.oca_bundle.said.clone().unwrap().to_string(),
                    meta_overlay
                        .attr_pairs
                        .get(&"name".to_string())
                        .unwrap_or(&"".to_string())
                        .clone(),
                    meta_overlay
                        .attr_pairs
                        .get(&"description".to_string())
                        .unwrap_or(&"".to_string())
                        .clone(),
                    meta_overlay.language,
                );

                oca_bundle_fts_repo.insert(oca_bundle_fts_record);
            }
        }

        oca_build.steps.iter().for_each(|step| {
            let mut input: Vec<u8> = vec![];
            match &step.parent_said {
                Some(said) => {
                    input.push(said.to_string().as_bytes().len().try_into().unwrap());
                    input.extend(said.to_string().as_bytes());
                },
                None => {
                    input.push(0);
                }
            }

            let command_str = serde_json::to_string(&step.command).unwrap();
            input.extend(command_str.as_bytes());
            let result_bundle = step.result.clone();
            self.db.insert(
                Namespace::OCA,
                &format!("oca.{}.operation", result_bundle.said.clone().unwrap()),
                &input,
            ).unwrap();

            self.db_cache.insert(
                Namespace::OCABundlesJSON,
                &result_bundle.said.clone().unwrap().to_string(),
                &result_bundle.encode().unwrap(),
            ).unwrap();
            self.db_cache.insert(
                Namespace::OCAObjectsJSON,
                &result_bundle.capture_base.said.clone().unwrap().to_string(),
                &serde_json::to_string(&result_bundle.capture_base).unwrap().into_bytes(),
            ).unwrap();
            result_bundle.overlays.iter().for_each(|overlay| {
                self.db_cache.insert(
                    Namespace::OCAObjectsJSON,
                    &overlay.said().clone().unwrap().to_string(),
                    &serde_json::to_string(&overlay).unwrap().into_bytes(),
                ).unwrap();
            });
        });

        let _ = self.add_relations(oca_build.oca_bundle.clone());

        let result_models = build_core_db_model(&oca_build);
        result_models.iter().for_each(|model| {
            if let Some(command_model) = &model.command {
                self.db
                    .insert(
                        Namespace::CoreModel,
                        &format!("core_model.{}", command_model.digest),
                        &command_model.json.clone().into_bytes(),
                    )
                    .unwrap();
            }

            if let Some(capture_base_model) = &model.capture_base {
                let mut input: Vec<u8> = vec![];
                match &capture_base_model.parent {
                    Some(said) => {
                        input.push(
                            said.to_string()
                                .as_bytes()
                                .len()
                                .try_into()
                                .unwrap(),
                        );
                        input.extend(said.to_string().as_bytes());
                    }
                    None => {
                        input.push(0);
                    }
                }

                input.push(
                    capture_base_model
                        .command_digest
                        .to_string()
                        .as_bytes()
                        .len()
                        .try_into()
                        .unwrap(),
                );
                input.extend(
                    capture_base_model.command_digest.to_string().as_bytes(),
                );

                self.db
                    .insert(
                        Namespace::CoreModel,
                        &format!(
                            "core_model.{}",
                            capture_base_model.capture_base_said
                        ),
                        &input,
                    )
                    .unwrap();
            }

            if let Some(overlay_model) = &model.overlay {
                let mut input: Vec<u8> = vec![];
                match &overlay_model.parent {
                    Some(said) => {
                        input.push(
                            said.to_string()
                                .as_bytes()
                                .len()
                                .try_into()
                                .unwrap(),
                        );
                        input.extend(said.to_string().as_bytes());
                    }
                    None => {
                        input.push(0);
                    }
                }

                input.push(
                    overlay_model
                        .command_digest
                        .to_string()
                        .as_bytes()
                        .len()
                        .try_into()
                        .unwrap(),
                );
                input.extend(
                    overlay_model.command_digest.to_string().as_bytes(),
                );

                self.db
                    .insert(
                        Namespace::CoreModel,
                        &format!("core_model.{}", overlay_model.overlay_said),
                        &input,
                    )
                    .unwrap();
            }

            if let Some(oca_bundle_model) = &model.oca_bundle {
                let mut input: Vec<u8> = vec![];
                match &oca_bundle_model.parent {
                    Some(said) => {
                        input.push(
                            said.to_string()
                                .as_bytes()
                                .len()
                                .try_into()
                                .unwrap(),
                        );
                        input.extend(said.to_string().as_bytes());
                    }
                    None => {
                        input.push(0);
                    }
                }

                input.push(
                    oca_bundle_model
                        .capture_base_said
                        .to_string()
                        .as_bytes()
                        .len()
                        .try_into()
                        .unwrap(),
                );
                input.extend(
                    oca_bundle_model.capture_base_said.to_string().as_bytes(),
                );

                for said in &oca_bundle_model.overlays_said {
                    input.push(
                        said.to_string().as_bytes().len().try_into().unwrap(),
                    );
                    input.extend(said.to_string().as_bytes());
                }

                self.db
                    .insert(
                        Namespace::CoreModel,
                        &format!(
                            "core_model.{}",
                            oca_bundle_model.oca_bundle_said
                        ),
                        &input,
                    )
                    .unwrap();
            }
        });

        Ok(oca_build.oca_bundle)
    }

    // TODO should not be String but SAID
    #[cfg(feature = "local-references")]
    fn store_reference(&mut self, refn: &String, bundle_said: String) {
        if !refn.is_empty() {
            self.db.insert(
                Namespace::OCAReferences,
                refn,
                bundle_said.to_string().as_bytes(),
            ).unwrap();
        }
    }
}
