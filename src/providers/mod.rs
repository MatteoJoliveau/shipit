use serde::Deserialize;

use crate::repository::Repository;

use self::{gitea::Gitea, gitlab::Gitlab};

mod gitea;
mod gitlab;

#[derive(Deserialize)]
#[serde(tag = "provider", rename_all = "snake_case")]
pub enum Provider {
    Gitea {
        api_url: String,
        project_id: String,
        token: String,
    },
    #[serde(rename = "gitlab")]
    GitLab {
        #[serde(default = "gitlab::default_api_url")]
        api_url: String,
        project_id: String,
        token: String,
    },
}

impl Provider {
    pub fn name(&self) -> &'static str {
        match self {
            Provider::Gitea { .. } => "gitea",
            Provider::GitLab { .. } => "gitlab",
        }
    }
}

pub fn get_repository(provider: Provider) -> Box<dyn Repository> {
    match provider {
        Provider::Gitea {
            api_url,
            project_id,
            token,
        } => Box::new(Gitea::new(api_url, project_id, token)),
        Provider::GitLab {
            api_url,
            project_id,
            token,
        } => Box::new(Gitlab::new(api_url, project_id, token)),
    }
}
