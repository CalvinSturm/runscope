use runscope_core::services::{AppPaths, CompareService, IngestRequest, IngestService};
use std::path::{Path, PathBuf};
use tempfile::tempdir;

#[test]
fn compare_runs_reports_metadata_metric_and_artifact_diffs() {
    let temp = tempdir().unwrap();
    let paths = AppPaths {
        db_path: temp.path().join("runscope.sqlite"),
        data_dir: temp.path().join("data"),
    };

    let localagent =
        IngestService::ingest_dir(&paths, ingest_request(localagent_fixture_dir())).unwrap();
    let videoforge =
        IngestService::ingest_dir(&paths, ingest_request(videoforge_fixture_dir())).unwrap();

    let report =
        CompareService::compare_runs(&paths, &localagent.run_id, &videoforge.run_id).unwrap();

    assert_eq!(report.left_run_id, localagent.run_id);
    assert_eq!(report.right_run_id, videoforge.run_id);
    assert!(report
        .metadata_diffs
        .iter()
        .any(|diff| diff.field == "project.slug"));
    assert!(report.metric_diffs.iter().any(|diff| diff.key == "fps"));
    assert!(!report.artifact_diffs.is_empty());
}

fn ingest_request(artifact_dir: PathBuf) -> IngestRequest {
    IngestRequest {
        artifact_dir,
        adapter: Some("auto".to_string()),
        project_override: None,
        label_override: None,
        tags: vec!["smoke".to_string()],
        note: Some("fixture ingest".to_string()),
        dry_run: false,
    }
}

fn localagent_fixture_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("localagent")
        .join("basic")
}

fn videoforge_fixture_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("videoforge")
        .join("basic")
}
