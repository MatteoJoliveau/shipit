use anyhow::Result;
use json_patch::Patch;
use serde::Deserialize;

use crate::{commit::FileList, repository::Repository};

mod json;

#[derive(Deserialize)]
#[serde(tag = "templater", rename_all = "snake_case")]
pub enum Mutation {
    Json { file: String, patch: Patch },
}

pub fn mutate(
    repository: &impl Repository,
    branch: &str,
    mutations: &[Mutation],
) -> Result<FileList> {
    let mut changed: std::collections::HashMap<String, bytes::Bytes> = FileList::default();
    for mutation in mutations {
        let delta = match mutation {
            Mutation::Json { file, patch } => {
                let to_patch = repository.get(&file, branch)?;
                let patched = json::update_file(&to_patch, patch)?;
                FileList::from([(file.into(), patched)])
            }
        };
        changed.extend(delta);
    }
    Ok(changed)
}
