use anyhow::Result;
use bytes::Bytes;
use json_patch::{patch, Patch};

pub fn update_json_file(file: &Bytes, changes: &Patch) -> Result<Bytes> {
    let mut value = serde_json::from_slice(&file)?;

    patch(&mut value, &changes)?;

    Ok(Bytes::from(serde_json::to_vec(&value)?))
}

#[cfg(test)]
mod tests {
    use json_patch::{PatchOperation, ReplaceOperation};
    use serde::Deserialize;
    use serde_json::json;

    use super::*;

    #[derive(Deserialize)]
    struct NestedStruct {
        pub nested: String,
    }

    #[derive(Deserialize)]
    struct TestStruct {
        pub test: NestedStruct,
    }

    #[test]
    fn test_update_json_file() {
        let original = json!({"test": {"nested": "dummy"}});

        let file = update_json_file(
            &Bytes::from(serde_json::to_vec(&original).unwrap()),
            &Patch(vec![PatchOperation::Replace(ReplaceOperation {
                path: "/test/nested".to_string(),
                value: json!("changed"),
            })]),
        )
        .unwrap();

        let parsed: TestStruct = serde_json::from_slice(&file).unwrap();

        assert_eq!(parsed.test.nested, "changed");
    }
}
