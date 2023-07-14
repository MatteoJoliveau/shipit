use crate::{commit::CommitRequest, repository::Repository};
use anyhow::Result;
use base64::{engine::general_purpose, Engine};
use bytes::Bytes;
use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};
use reqwest::{
    blocking::Client,
    header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE},
    StatusCode,
};
use serde::{Deserialize, Serialize};

const USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));
const DEFAULT_GITLAB_API_URL: &str = "https://gitlab.com/api/v4";
const FRAGMENT: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'<')
    .add(b'>')
    .add(b'`')
    .add(b'/');

pub fn default_api_url() -> String {
    DEFAULT_GITLAB_API_URL.into()
}

#[derive(Deserialize)]
struct CommitResponse {
    web_url: String,
}

#[derive(Debug, Serialize)]
struct CommitPayload {
    branch: String,
    commit_message: String,
    actions: Vec<CommitAction>,
    author_name: String,
    author_email: String,
}

#[derive(Debug, Serialize)]
struct CommitAction {
    action: Action,
    file_path: String,
    content: String,
    encoding: Encoding,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum Action {
    Create,
    Update,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum Encoding {
    Text,
    Base64,
}

pub struct Gitlab {
    api_url: String,
    project_id: String,
    token: String,
    client: Client,
}

impl Gitlab {
    pub fn new(api_url: impl ToString, project_id: impl ToString, token: impl ToString) -> Self {
        Self {
            api_url: api_url.to_string(),
            project_id: project_id.to_string(),
            token: token.to_string(),
            client: Client::builder()
                .user_agent(USER_AGENT)
                .build()
                .expect("failed to contruct HTTP client"),
        }
    }

    fn auth_header(&self) -> String {
        format!("Bearer {}", self.token)
    }

    fn file_to_action(
        &self,
        file_path: impl Into<String>,
        reference: &str,
        content: Bytes,
    ) -> Result<CommitAction> {
        let file_path = file_path.into();
        let action = if self.check_file_exists(&file_path, reference)? {
            Action::Update
        } else {
            Action::Create
        };
        let (encoding, content) = match std::str::from_utf8(&content) {
            Ok(content) => (Encoding::Text, content.into()),
            Err(_) => (Encoding::Base64, general_purpose::STANDARD.encode(content)),
        };

        Ok(CommitAction {
            action,
            file_path,
            content,
            encoding,
        })
    }

    fn check_file_exists(&self, path: &str, reference: &str) -> Result<bool> {
        let response = self
            .client
            .head(format!(
                "{}/projects/{}/repository/files/{}/raw",
                self.api_url,
                utf8_percent_encode(&self.project_id, FRAGMENT),
                utf8_percent_encode(path, FRAGMENT)
            ))
            .query(&[("ref", reference)])
            .header(AUTHORIZATION, self.auth_header())
            .send()?;

        if response.status() == StatusCode::NOT_FOUND {
            return Ok(false);
        }

        response.error_for_status()?;

        Ok(true)
    }
}

impl Repository for Gitlab {
    fn get(&self, path: &str, reference: &str) -> Result<Bytes> {
        log::debug!(
            "fetching file path={path} project={} ref={reference} api_url={}",
            self.project_id,
            self.api_url
        );

        let response = self
            .client
            .get(format!(
                "{}/projects/{}/repository/files/{}/raw",
                self.api_url,
                utf8_percent_encode(&self.project_id, FRAGMENT),
                utf8_percent_encode(path, FRAGMENT)
            ))
            .query(&[("ref", reference)])
            .header(AUTHORIZATION, self.auth_header())
            .send()?
            .error_for_status()?;

        Ok(response.bytes()?)
    }

    fn commit(&mut self, payload: CommitRequest) -> Result<()> {
        log::debug!(
            "committing changes author={} ref={} message={} project={} api_url={}",
            payload.author,
            payload.branch,
            payload.message,
            self.project_id,
            self.api_url
        );

        let (author_name, author_email) = payload.split_author();

        let actions = payload
            .files
            .into_iter()
            .map(|(file, content)| self.file_to_action(file, &payload.branch, content))
            .collect::<Result<Vec<CommitAction>>>()?;

        #[cfg(test)]
        let actions = {
            let mut actions = actions;
            // Since payload.files is backed by a map, the order of files is not stable
            // To simplify matching against the JSON body during testing, we sort the vector
            actions.sort_by(|a, b| a.file_path.cmp(&b.file_path));
            actions
        };

        let body = &CommitPayload {
            branch: payload.branch,
            commit_message: payload.message,
            actions,
            author_name,
            author_email,
        };

        let CommitResponse { web_url } = self
            .client
            .post(format!(
                "{}/projects/{}/repository/commits",
                self.api_url,
                utf8_percent_encode(&self.project_id, FRAGMENT),
            ))
            .header(CONTENT_TYPE, "application/json")
            .header(AUTHORIZATION, self.auth_header())
            .header(ACCEPT, "application/json")
            .json(body)
            .send()?
            .error_for_status()?
            .json()?;

        log::info!("commit URL: {web_url}");

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use bytes::{BufMut, BytesMut};
    use mockito::Matcher;

    use crate::commit::FileList;

    use super::*;

    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[test]
    fn test_get() {
        init();

        let mut server = mockito::Server::new();
        let get_mock = server
            .mock(
                "GET",
                "/projects/test%2Ftest/repository/files/test%2Ftest.txt/raw?ref=main",
            )
            .match_header("authorization", "Bearer gitlab-token")
            .match_header("user-agent", USER_AGENT)
            .with_body("hello!")
            .create();

        assert_eq!(
            Gitlab::new(server.url(), "test/test", "gitlab-token")
                .get("test/test.txt", "main")
                .unwrap(),
            Bytes::from("hello!")
        );

        get_mock.assert();
    }

    #[test]
    fn test_commit_new() {
        init();

        let mut server = mockito::Server::new();

        let get_txt_mock = server
            .mock(
                "HEAD",
                "/projects/test%2Ftest/repository/files/test%2Ftest.txt/raw?ref=main",
            )
            .match_header("authorization", "Bearer gitlab-token")
            .match_header("user-agent", USER_AGENT)
            .with_status(404)
            .create();
        let get_bin_mock = server
            .mock(
                "HEAD",
                "/projects/test%2Ftest/repository/files/test.bin/raw?ref=main",
            )
            .match_header("authorization", "Bearer gitlab-token")
            .match_header("user-agent", USER_AGENT)
            .with_status(404)
            .create();
        let put_mock = server
            .mock("POST", "/projects/test%2Ftest/repository/commits")
            .match_header("authorization", "Bearer gitlab-token")
            .match_header("content-type", "application/json")
            .match_header("accept", "application/json")
            .match_header("user-agent", USER_AGENT)
            .match_body(Matcher::Json(serde_json::json!({
                "branch": "main",
                "commit_message": "test",
                "actions": [
                    {
                        "action": "create",
                        "file_path": "test.bin",
                        "content": "BNI=",
                        "encoding": "base64"
                    },
                    {
                        "action": "create",
                        "file_path": "test/test.txt",
                        "content": "test",
                        "encoding": "text"
                    },
                ],
                "author_name": "test",
                "author_email": "author@email.tld",
            })))
            .with_header("content-type", "application/json")
            .with_body(
                serde_json::to_vec(&serde_json::json!({ "web_url": "https://example.com" }))
                    .unwrap(),
            )
            .create();

        let mut non_utf8 = BytesMut::new();
        non_utf8.put_u16(1234);

        Gitlab::new(server.url(), "test/test", "gitlab-token")
            .commit(CommitRequest {
                branch: "main".into(),
                author: "test <author@email.tld>".into(),
                message: "test".into(),
                files: FileList::from([
                    ("test/test.txt".into(), "test".into()),
                    ("test.bin".into(), non_utf8.into()),
                ]),
            })
            .unwrap();

        get_txt_mock.assert();
        get_bin_mock.assert();
        put_mock.assert();
    }

    #[test]
    fn test_commit_existing() {
        init();

        let mut server = mockito::Server::new();

        let get_txt_mock = server
            .mock(
                "HEAD",
                "/projects/test%2Ftest/repository/files/test%2Ftest.txt/raw?ref=main",
            )
            .match_header("authorization", "Bearer gitlab-token")
            .match_header("user-agent", USER_AGENT)
            .create();
        let get_bin_mock = server
            .mock(
                "HEAD",
                "/projects/test%2Ftest/repository/files/test.bin/raw?ref=main",
            )
            .match_header("authorization", "Bearer gitlab-token")
            .match_header("user-agent", USER_AGENT)
            .create();
        let put_mock = server
            .mock("POST", "/projects/test%2Ftest/repository/commits")
            .match_header("authorization", "Bearer gitlab-token")
            .match_header("content-type", "application/json")
            .match_header("accept", "application/json")
            .match_header("user-agent", USER_AGENT)
            .match_body(Matcher::Json(serde_json::json!({
                "branch": "main",
                "commit_message": "test",
                "actions": [
                    {
                        "action": "update",
                        "file_path": "test.bin",
                        "content": "BNI=",
                        "encoding": "base64"
                    },
                    {
                        "action": "update",
                        "file_path": "test/test.txt",
                        "content": "test",
                        "encoding": "text"
                    },
                ],
                "author_name": "test",
                "author_email": "author@email.tld",
            })))
            .with_header("content-type", "application/json")
            .with_body(
                serde_json::to_vec(&serde_json::json!({ "web_url": "https://example.com" }))
                    .unwrap(),
            )
            .create();

        let mut non_utf8 = BytesMut::new();
        non_utf8.put_u16(1234);

        Gitlab::new(server.url(), "test/test", "gitlab-token")
            .commit(CommitRequest {
                branch: "main".into(),
                author: "test <author@email.tld>".into(),
                message: "test".into(),
                files: FileList::from([
                    ("test/test.txt".into(), "test".into()),
                    ("test.bin".into(), non_utf8.into()),
                ]),
            })
            .unwrap();

        get_txt_mock.assert();
        get_bin_mock.assert();
        put_mock.assert();
    }
}
