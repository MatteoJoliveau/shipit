use bytes::Bytes;
use std::collections::HashMap;

pub type FileList = HashMap<String, Bytes>;

#[derive(Clone, Default)]
pub struct CommitRequest {
    pub branch: String,
    pub author: String,
    pub message: String,
    pub files: FileList,
}

impl CommitRequest {
    /// split_author splits the author field to return a tuple of (name, email) fields.
    pub fn split_author(&self) -> (String, String) {
        match self.author.split_once('<') {
            None => (self.author.to_string(), "".to_string()),
            Some((name, email)) => (
                name.trim().to_string(),
                email
                    .trim_matches(|c: char| c.is_whitespace() || c == '<' || c == '>')
                    .to_string(),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_author() {
        let (name, email) = ("test-author", "author@example.com");
        let combined = format!("{} <{}>", name, email);

        assert_eq!(
            CommitRequest {
                author: combined,
                ..Default::default()
            }
            .split_author(),
            (name.to_string(), email.to_string())
        );

        assert_eq!(
            CommitRequest {
                author: name.to_string(),
                ..Default::default()
            }
            .split_author(),
            (name.to_string(), "".into())
        );

        assert_eq!(
            CommitRequest {
                author: format!("<{}>", email),
                ..Default::default()
            }
            .split_author(),
            ("".into(), email.to_string())
        );
    }
}
