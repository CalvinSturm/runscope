use runscope_core::domain::SetBaselineRequest;
use runscope_core::services::{
    AppPaths, BaselineService, IngestRequest, IngestService, QueryService,
};
use std::path::{Path, PathBuf};
use tempfile::tempdir;

#[test]
fn baseline_set_replaces_active_binding_same_scope() {
    let temp = tempdir().unwrap();
    let paths = AppPaths {
        db_path: temp.path().join("runscope.sqlite"),
        data_dir: temp.path().join("data"),
    };

    let first =
        IngestService::ingest_dir(&paths, ingest_request(videoforge_fixture_dir())).unwrap();

    let second_fixture = temp.path().join("videoforge-copy");
    copy_dir_all(&videoforge_fixture_dir(), &second_fixture);
    let renamed_manifest = second_fixture.join("videoforge_run.json");
    let content = std::fs::read_to_string(&renamed_manifest).unwrap();
    let updated = content.replace("\"label\": \"main smoke\"", "\"label\": \"main smoke v2\"");
    std::fs::write(&renamed_manifest, updated).unwrap();

    let second = IngestService::ingest_dir(&paths, ingest_request(second_fixture)).unwrap();

    let first_binding = BaselineService::set_active_baseline(
        &paths,
        SetBaselineRequest {
            run_id: first.run_id.clone(),
            label: "default".to_string(),
        },
    )
    .unwrap();
    let second_binding = BaselineService::set_active_baseline(
        &paths,
        SetBaselineRequest {
            run_id: second.run_id.clone(),
            label: "default".to_string(),
        },
    )
    .unwrap();

    assert_ne!(first_binding.id, second_binding.id);

    let baselines = BaselineService::list_baselines(&paths, "videoforge").unwrap();
    assert_eq!(baselines.len(), 1);
    assert_eq!(baselines[0].run_id, second.run_id);

    let detail = QueryService::get_run(&paths, &second.run_id).unwrap();
    assert_eq!(detail.active_baselines.len(), 1);
    assert_eq!(detail.active_baselines[0].run_id, second.run_id);
}

#[test]
fn baseline_list_returns_active_project_bindings() {
    let temp = tempdir().unwrap();
    let paths = AppPaths {
        db_path: temp.path().join("runscope.sqlite"),
        data_dir: temp.path().join("data"),
    };

    let videoforge =
        IngestService::ingest_dir(&paths, ingest_request(videoforge_fixture_dir())).unwrap();
    let localagent =
        IngestService::ingest_dir(&paths, ingest_request(localagent_fixture_dir())).unwrap();

    BaselineService::set_active_baseline(
        &paths,
        SetBaselineRequest {
            run_id: videoforge.run_id.clone(),
            label: "default".to_string(),
        },
    )
    .unwrap();
    BaselineService::set_active_baseline(
        &paths,
        SetBaselineRequest {
            run_id: localagent.run_id.clone(),
            label: "default".to_string(),
        },
    )
    .unwrap();

    let videoforge_baselines = BaselineService::list_baselines(&paths, "videoforge").unwrap();
    assert_eq!(videoforge_baselines.len(), 1);
    assert_eq!(videoforge_baselines[0].run_id, videoforge.run_id);
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

fn copy_dir_all(src: &Path, dst: &Path) {
    std::fs::create_dir_all(dst).unwrap();
    for entry in std::fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let source = entry.path();
        let destination = dst.join(entry.file_name());
        if source.is_dir() {
            copy_dir_all(&source, &destination);
        } else {
            std::fs::copy(source, destination).unwrap();
        }
    }
}
