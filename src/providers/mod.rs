use serde::Deserialize;

use crate::repository::Repository;

use self::gitea::Gitea;

mod gitea;

#[derive(Deserialize)]
#[serde(tag = "templater")]
pub enum Provider {
    Gitea {
        api_url: String,
        project_id: String,
        token: String,
    },
}

pub fn get_repository(provider: Provider) -> impl Repository {
    match provider {
        Provider::Gitea {
            api_url,
            project_id,
            token,
        } => Gitea::new(api_url, project_id, token),
    }
}
