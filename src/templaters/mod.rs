use std::collections::HashMap;

use anyhow::Result;
use serde::Deserialize;

use crate::{commit::FileList, repository::Repository};

mod json;
mod yaml;

#[derive(Deserialize)]
#[serde(tag = "templater", rename_all = "snake_case")]
pub enum Mutation {
    Json {
        file: String,
        changes: HashMap<String, String>,
    },
    Yaml {
        file: String,
        changes: HashMap<String, String>,
    },
}

pub fn mutate(
    repository: &impl Repository,
    branch: &str,
    mutations: &[Mutation],
) -> Result<FileList> {
    let mut changed: std::collections::HashMap<String, bytes::Bytes> = FileList::default();
    for mutation in mutations {
        let delta = match mutation {
            Mutation::Json { file, changes } => {
                let to_patch = repository.get(file, branch)?;
                let patched = json::update_file(&to_patch, changes)?;
                FileList::from([(file.into(), patched)])
            }
            Mutation::Yaml { file, changes } => {
                let to_patch = repository.get(file, branch)?;
                let patched = yaml::update_file(&to_patch, changes)?;
                FileList::from([(file.into(), patched)])
            }
        };
        changed.extend(delta);
    }
    Ok(changed)
}
