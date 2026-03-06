use crate::domain::RunManifestV1;
use crate::error::RunScopeError;
use schemars::schema_for;
use std::fs;
use std::path::Path;

pub fn schema_json_string() -> Result<String, RunScopeError> {
    let mut schema = schema_for!(RunManifestV1);
    ensure_required_top_level_fields(&mut schema);
    Ok(serde_json::to_string_pretty(&schema)?)
}

pub fn write_schema_file(path: &Path) -> Result<(), RunScopeError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, schema_json_string()?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn schema_file_matches_generated_schema() {
        let schema_path = schema_path();
        let committed = fs::read_to_string(schema_path).expect("schema file should exist");
        let generated = schema_json_string().expect("schema generation should succeed");
        assert_eq!(committed, generated);
    }

    #[test]
    #[ignore]
    fn write_committed_schema_snapshot() {
        write_schema_file(&schema_path()).expect("schema snapshot write should succeed");
    }

    fn schema_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("schema")
            .join("run.v1.schema.json")
    }

    #[test]
    fn generated_schema_requires_metrics_and_artifacts() {
        let schema = schema_json_string().expect("schema generation should succeed");
        let value: serde_json::Value =
            serde_json::from_str(&schema).expect("schema JSON should parse");
        let required = value
            .get("required")
            .and_then(serde_json::Value::as_array)
            .expect("required array should exist");

        assert!(required.contains(&serde_json::Value::String("metrics".to_string())));
        assert!(required.contains(&serde_json::Value::String("artifacts".to_string())));
    }
}

fn ensure_required_top_level_fields(schema: &mut schemars::schema::RootSchema) {
    let Some(object) = &mut schema.schema.object else {
        return;
    };

    for field in ["metrics", "artifacts"] {
        if !object.required.contains(field) {
            object.required.insert(field.to_string());
        }
    }
}
