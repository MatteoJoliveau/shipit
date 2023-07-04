use crate::{
    commit::{CommitRequest, FileList},
    repository::Repository,
};
use anyhow::{anyhow, Result};
use base64::{engine::general_purpose, Engine};
use bytes::Bytes;
use reqwest::{
    blocking::Client,
    header::{AUTHORIZATION, CONTENT_TYPE},
};
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
struct GiteaFileInfo {
    sha: String,
}

pub struct Gitea {
    api_url: String,
    project_id: String,
    credentials: String,
    client: Client,
}

impl Gitea {
    pub fn new(api_url: String, project_id: String, credentials: String) -> Self {
        Self {
            api_url,
            project_id,
            credentials,
            client: Client::new(),
        }
    }

    fn auth_header(&self) -> String {
        format!(
            "Basic {}",
            general_purpose::STANDARD.encode(&self.credentials)
        )
    }

    fn file_sha(&self, path: &str, reference: &str) -> Result<String> {
        let response = self
            .client
            .get(format!(
                "{}/repos/{}/contents/{}",
                self.api_url, self.project_id, path
            ))
            .query(&[("ref", reference)])
            .header(AUTHORIZATION, self.auth_header())
            .send()?;

        if response.status() == 404 {
            return Ok("".into());
        }

        if !response.status().is_success() {
            return Err(anyhow!(response.text()?));
        }

        Ok(response.json::<GiteaFileInfo>()?.sha)
    }

    fn commit_file(&self, payload: CommitRequest) -> Result<()> {
        let (author, email) = payload.split_author();

        // Our payload should contain exactly one file
        let (file, content) = payload
            .files
            .iter()
            .take(1)
            .next()
            .ok_or_else(|| anyhow!("No file found"))?;

        // if the original file already exists we need to provide their latest SHA
        let sha = self.file_sha(file, &payload.branch)?;

        let body = json!({
            "author": {
              "name": author,
              "email": email
            },
            "message": payload.message,
            "branch": payload.branch,
            "content": general_purpose::STANDARD.encode(content),
            "sha": sha,
        })
        .to_string();

        let response = self
            .client
            .put(format!(
                "{}/repos/{}/contents/{}",
                self.api_url, self.project_id, file
            ))
            .header(AUTHORIZATION, self.auth_header())
            .header(CONTENT_TYPE, "application/json")
            .body(body)
            .send()?;

        if !response.status().is_success() {
            return Err(anyhow!(response.text()?));
        }

        Ok(())
    }
}

impl Repository for Gitea {
    fn get(&self, path: &str, reference: &str) -> Result<Bytes> {
        let response = self
            .client
            .get(format!(
                "{}/repos/{}/raw/{}",
                self.api_url, self.project_id, path
            ))
            .query(&[("ref", reference)])
            .header(AUTHORIZATION, self.auth_header())
            .send()?;
        if !response.status().is_success() {
            return Err(anyhow!(response.text()?));
        }
        Ok(response.bytes()?)
    }

    fn commit(&mut self, payload: CommitRequest) -> Result<()> {
        let multiple_files = payload.files.len() > 1;
        for file in payload.files {
            self.commit_file(CommitRequest {
                branch: payload.branch.clone(),
                author: payload.author.clone(),
                message: if multiple_files {
                    format!("{}: {}", file.0, payload.message.clone())
                } else {
                    payload.message.clone()
                },
                files: FileList::from([file]),
            })?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get() {
        let mut server = mockito::Server::new();
        let get_mock = server
            .mock("GET", "/repos/test/raw/test?ref=master")
            .match_header("authorization", "Basic dGVzdA==")
            .with_body("hello!")
            .create();

        assert_eq!(
            Gitea::new(server.url(), "test".into(), "test".into())
                .get("test", "master")
                .unwrap(),
            Bytes::from("hello!")
        );

        get_mock.assert();
    }

    #[test]
    fn test_commit_new() {
        let mut server = mockito::Server::new();

        let get_mock = server
            .mock("GET", "/repos/test/contents/test?ref=master")
            .match_header("authorization", "Basic dGVzdA==")
            .with_status(404)
            .create();
        let put_mock = server
            .mock("PUT", "/repos/test/contents/test")
            .match_header("authorization", "Basic dGVzdA==")
            .match_header("content-type", "application/json")
            .match_body(r#"{"author":{"email":"author@email.tld","name":"test"},"branch":"master","content":"dGVzdA==","message":"test","sha":""}"#)
            .with_header("content-type", "application/json")
            .with_body(r#"{}"#)
            .create();

        Gitea::new(server.url(), "test".into(), "test".into())
            .commit(CommitRequest {
                branch: "master".into(),
                author: "test <author@email.tld>".into(),
                message: "test".into(),
                files: FileList::from([("test".into(), "test".into())]),
            })
            .unwrap();

        get_mock.assert();
        put_mock.assert();
    }

    #[test]
    fn test_commit_existing() {
        let mut server = mockito::Server::new();

        let get_mock = server
            .mock("GET", "/repos/test/contents/test?ref=master")
            .match_header("authorization", "Basic dGVzdA==")
            .with_body(r#"{"content":"dGVzdA==","sha":"test"}"#)
            .create();
        let put_mock = server
            .mock("PUT", "/repos/test/contents/test")
            .match_header("authorization", "Basic dGVzdA==")
            .match_header("content-type", "application/json")
            .match_body(r#"{"author":{"email":"author@email.tld","name":"test"},"branch":"master","content":"dGVzdA==","message":"test","sha":"test"}"#)
            .with_header("content-type", "application/json")
            .with_body(r#"{}"#)
            .create();

        Gitea::new(server.url(), "test".into(), "test".into())
            .commit(CommitRequest {
                branch: "master".into(),
                author: "test <author@email.tld>".into(),
                message: "test".into(),
                files: FileList::from([("test".into(), "test".into())]),
            })
            .unwrap();

        get_mock.assert();
        put_mock.assert();
    }
}
