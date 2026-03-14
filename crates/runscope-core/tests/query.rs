use runscope_core::domain::{ExecStatus, RunListFilter};
use runscope_core::services::{AppPaths, IngestRequest, IngestService, QueryService};
use std::path::{Path, PathBuf};
use tempfile::tempdir;

#[test]
fn list_runs_returns_video_forge_and_localagent_rows() {
    let temp = tempdir().unwrap();
    let paths = AppPaths {
        db_path: temp.path().join("runscope.sqlite"),
        data_dir: temp.path().join("data"),
    };

    let localagent =
        IngestService::ingest_dir(&paths, ingest_request(localagent_fixture_dir())).unwrap();
    let videoforge =
        IngestService::ingest_dir(&paths, ingest_request(videoforge_fixture_dir())).unwrap();

    let page = QueryService::list_runs(
        &paths,
        RunListFilter {
            limit: 20,
            ..RunListFilter::default()
        },
    )
    .unwrap();

    assert_eq!(page.total, 2);
    assert_eq!(page.items.len(), 2);
    assert!(page
        .items
        .iter()
        .any(|item| item.run_id == localagent.run_id && item.project_slug == "localagent"));
    assert!(page.items.iter().any(|item| {
        item.run_id == localagent.run_id
            && item.backend.as_deref() == Some("local")
            && item.model.as_deref() == Some("assistant-basic")
            && item.precision.as_deref() == Some("int8")
    }));
    assert!(page
        .items
        .iter()
        .any(|item| item.run_id == videoforge.run_id && item.project_slug == "videoforge"));
    assert!(page
        .items
        .iter()
        .any(|item| item.project_slug == "videoforge" && !item.primary_metrics.is_empty()));
}

#[test]
fn list_runs_applies_project_and_status_filters() {
    let temp = tempdir().unwrap();
    let paths = AppPaths {
        db_path: temp.path().join("runscope.sqlite"),
        data_dir: temp.path().join("data"),
    };

    IngestService::ingest_dir(&paths, ingest_request(localagent_fixture_dir())).unwrap();
    IngestService::ingest_dir(&paths, ingest_request(videoforge_fixture_dir())).unwrap();

    let page = QueryService::list_runs(
        &paths,
        RunListFilter {
            project: Some("videoforge".to_string()),
            exec_status: Some(ExecStatus::Pass),
            limit: 10,
            ..RunListFilter::default()
        },
    )
    .unwrap();

    assert_eq!(page.total, 1);
    assert_eq!(page.items[0].project_slug, "videoforge");
}

#[test]
fn get_run_reads_manifest_and_related_records() {
    let temp = tempdir().unwrap();
    let paths = AppPaths {
        db_path: temp.path().join("runscope.sqlite"),
        data_dir: temp.path().join("data"),
    };

    let ingested =
        IngestService::ingest_dir(&paths, ingest_request(videoforge_fixture_dir())).unwrap();
    let detail = QueryService::get_run(&paths, &ingested.run_id).unwrap();

    assert_eq!(detail.manifest.project.slug, "videoforge");
    assert_eq!(detail.manifest.source.adapter, "videoforge");
    assert!(detail.run_root.ends_with(&ingested.run_id));
    assert!(detail.tags.iter().any(|tag| tag == "smoke"));
    assert_eq!(detail.notes.len(), 1);
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
