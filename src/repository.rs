use anyhow::Result;
use bytes::Bytes;
use std::collections::HashMap;

use super::commit::CommitRequest;

pub trait Repository {
    fn get(&self, path: &str, reference: &str) -> Result<Bytes>;
    fn commit(&mut self, payload: CommitRequest) -> Result<()>;
}

pub struct InMemoryRepository {
    files: HashMap<String, Bytes>,
}

impl Default for InMemoryRepository {
    fn default() -> Self {
        InMemoryRepository {
            files: Default::default(),
        }
    }
}

impl Repository for InMemoryRepository {
    fn get(&self, path: &str, reference: &str) -> Result<Bytes> {
        let key = format!("{}/{}", reference, path);
        self.files
            .get(&key)
            .cloned()
            .ok_or(anyhow::anyhow!("File not found"))
    }

    fn commit(&mut self, payload: CommitRequest) -> Result<()> {
        for (filename, content) in payload.files.iter() {
            let key = format!("{}/{}", payload.branch, filename);
            self.files.insert(key, content.clone());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_inmemory_repository() {
        // initialize repository with a file
        let test_content = Bytes::from("Hello World");
        let mut repo = InMemoryRepository {
            files: HashMap::from([("test/file.txt".to_string(), test_content.clone())]),
        };

        // try getting the file
        assert_eq!(repo.get("file.txt", "test").unwrap(), &test_content);

        // try adding a new file
        let other_file_name = "file2.txt";
        let other_test_content = Bytes::from("hello again!!");
        repo.commit(CommitRequest {
            files: HashMap::from([(other_file_name.to_string(), other_test_content.clone())]),
            branch: "test".to_string(),
            ..Default::default()
        })
        .unwrap();

        // try getting the new file
        assert_eq!(
            repo.get(other_file_name, "test").unwrap(),
            &other_test_content
        );
    }
}
