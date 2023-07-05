use anyhow::{anyhow, Result};
use bytes::Bytes;
use serde_yaml::Value;
use std::collections::HashMap;

/// returns the value at the given path
fn patch(base: &mut Value, path: &str, value: &str) -> Result<()> {
    let mut current = base;
    for part in path.split('.') {
        // if part is numeric, treat it as array index
        let numeric_part = part.parse::<usize>();
        if numeric_part.is_ok() {
            let index = numeric_part.unwrap();
            current = current
                .get_mut(index)
                .ok_or_else(|| anyhow!("could not find index path {}", path))?;
            continue;
        }

        // otherwise treat it as object key
        current = current
            .get_mut(part)
            .ok_or_else(|| anyhow!("could not find object path {}", path))?;
    }
    *current = Value::from(value);
    Ok(())
}

pub fn update_file(file: &Bytes, changes: &HashMap<String, String>) -> Result<Bytes> {
    let mut parsed: Value = serde_yaml::from_slice(file)?;

    // apply changes
    for (path, value) in changes {
        patch(&mut parsed, path, &value)?;
    }

    Ok(Bytes::from(serde_yaml::to_string(&parsed)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;

    #[test]
    fn test_update_file() {
        let original = Bytes::from(
            r#"
test:
  nested:
    key: changeme
something:
  - else"#,
        );

        let changed = update_file(
            &original,
            &HashMap::from([
                ("test.nested.key".to_string(), "changed".to_string()),
                ("something.0".to_string(), "also changed".to_string()),
            ]),
        )
        .unwrap();

        let parsed: Value = serde_yaml::from_slice(&changed).unwrap();
        assert_eq!(parsed["test"]["nested"]["key"], "changed");
        assert_eq!(parsed["something"][0], "also changed");
    }
}
