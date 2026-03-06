use runscope_core::db::connection::open_connection;
use runscope_core::db::migrations::apply_migrations;
use runscope_core::db::table_exists;
use runscope_core::domain::RunManifestV1;
use runscope_core::services::{AppPaths, IngestRequest, IngestService};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

#[test]
fn sqlite_migration_bootstrap_empty_db() {
    let temp = tempdir().unwrap();
    let db_path = temp.path().join("runscope.sqlite");
    let conn = open_connection(&db_path).unwrap();

    apply_migrations(&conn).unwrap();

    assert!(table_exists(&conn, "projects").unwrap());
    assert!(table_exists(&conn, "runs").unwrap());
    assert!(table_exists(&conn, "metrics").unwrap());
    assert!(table_exists(&conn, "artifacts").unwrap());
}

#[test]
fn duplicate_ingest_returns_existing_run() {
    let temp = tempdir().unwrap();
    let paths = AppPaths {
        db_path: temp.path().join("runscope.sqlite"),
        data_dir: temp.path().join("data"),
    };

    let first = IngestService::ingest_dir(&paths, ingest_request(fixture_dir())).unwrap();
    let second = IngestService::ingest_dir(&paths, ingest_request(fixture_dir())).unwrap();

    assert_eq!(first.run_id, second.run_id);
    assert!(!first.duplicate);
    assert!(second.duplicate);
}

#[test]
fn duplicate_ingest_matches_across_identical_copied_directories() {
    let temp = tempdir().unwrap();
    let fixture_copy = temp.path().join("fixture-copy");
    copy_dir_all(&fixture_dir(), &fixture_copy);
    let paths = AppPaths {
        db_path: temp.path().join("runscope.sqlite"),
        data_dir: temp.path().join("data"),
    };

    let first = IngestService::ingest_dir(&paths, ingest_request(fixture_dir())).unwrap();
    let second = IngestService::ingest_dir(&paths, ingest_request(fixture_copy)).unwrap();

    assert_eq!(first.run_id, second.run_id);
    assert!(second.duplicate);
}

#[test]
fn ingest_localagent_sample() {
    let temp = tempdir().unwrap();
    let paths = AppPaths {
        db_path: temp.path().join("runscope.sqlite"),
        data_dir: temp.path().join("data"),
    };

    let result = IngestService::ingest_dir(&paths, ingest_request(fixture_dir())).unwrap();
    let artifact_root = PathBuf::from(result.artifact_root.unwrap());
    let run_json = artifact_root.join("run.json");

    assert!(run_json.is_file());
    assert!(artifact_root.join("logs/stdout.log").is_file());

    let manifest: RunManifestV1 =
        serde_json::from_str(&fs::read_to_string(run_json).unwrap()).unwrap();
    assert_eq!(manifest.project.slug, "localagent");
    assert_eq!(manifest.source.adapter, "localagent");
    assert_eq!(manifest.metrics.len(), 1);
    assert_eq!(manifest.artifacts.len(), 2);
}

#[test]
fn ingest_localagent_sample_persists_expected_warning_count() {
    let temp = tempdir().unwrap();
    let paths = AppPaths {
        db_path: temp.path().join("runscope.sqlite"),
        data_dir: temp.path().join("data"),
    };

    let result = IngestService::ingest_dir(&paths, ingest_request(fixture_dir())).unwrap();
    assert!(result.warnings.is_empty());

    let artifact_root = PathBuf::from(result.artifact_root.unwrap());
    let manifest: RunManifestV1 =
        serde_json::from_str(&fs::read_to_string(artifact_root.join("run.json")).unwrap()).unwrap();
    assert_eq!(manifest.summary.warning_count, 0);
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

fn fixture_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("localagent")
        .join("basic")
}

fn copy_dir_all(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).unwrap();
    for entry in fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let source = entry.path();
        let destination = dst.join(entry.file_name());
        if source.is_dir() {
            copy_dir_all(&source, &destination);
        } else {
            fs::copy(source, destination).unwrap();
        }
    }
}
