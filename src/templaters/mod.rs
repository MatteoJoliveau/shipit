use anyhow::Result;
use json_patch::Patch;
use serde::Deserialize;

use crate::providers::{
    commit::{FileInfo, FileList},
    repository::Repository,
};

use self::json::update_json_file;

pub mod json;

#[derive(Deserialize)]
#[serde(tag = "templater")]
pub enum Mutation {
    Json { file: FileInfo, patch: Patch },
}

pub fn mutate(repository: &impl Repository, mutations: &[Mutation]) -> Result<FileList> {
    let mut changed: std::collections::HashMap<String, bytes::Bytes> = FileList::default();
    for mutation in mutations {
        let delta = match mutation {
            Mutation::Json { file, patch } => {
                let to_patch = repository.get(&file.file, &file.reference)?;
                let patched = update_json_file(&to_patch, patch)?;
                FileList::from([(file.file.clone(), patched)])
            }
        };
        changed.extend(delta);
    }
    Ok(changed)
}
