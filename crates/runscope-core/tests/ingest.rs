use runscope_core::db::connection::open_connection;
use runscope_core::db::migrations::apply_migrations;
use runscope_core::db::table_exists;
use runscope_core::domain::{ExecStatus, MetricDirection, MetricRecord, RunManifestV1, SourceKind};
use runscope_core::services::{
    AppPaths, IngestRequest, IngestService, ManualAttachment, ManualRecordRequest, RecordService,
};
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
    assert_eq!(manifest.runtime.started_at.as_deref(), Some("2026-03-05T17:17:04Z"));
    assert_eq!(
        manifest.environment.as_ref().and_then(|env| env.backend.as_deref()),
        Some("local")
    );
    assert_eq!(
        manifest.environment.as_ref().and_then(|env| env.model.as_deref()),
        Some("assistant-basic")
    );
    assert_eq!(
        manifest.workload.as_ref().and_then(|workload| workload.display_command.as_deref()),
        Some("localagent eval --scenario assistant_basic")
    );
    assert_eq!(
        manifest.workload.as_ref().and_then(|workload| workload.env_snapshot_ref.as_deref()),
        Some("attachments/env.redacted.json")
    );
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

#[test]
fn ingest_localagent_eval_results_json_file() {
    let temp = tempdir().unwrap();
    let paths = AppPaths {
        db_path: temp.path().join("runscope.sqlite"),
        data_dir: temp.path().join("data"),
    };
    let results_json = temp.path().join("results_2026-03-07T22-20-30.0697001Z.json");
    fs::write(
        &results_json,
        r#"{
            "schema_version":"openagent.eval.v1",
            "created_at":"2026-03-07T22:20:30Z",
            "config":{
                "provider":"ollama",
                "models":["qwen2.5-coder-7b-instruct@q8_0"],
                "pack":"commoncodingux",
                "mode":"single",
                "runs_per_task":2,
                "max_steps":60,
                "timeout_seconds":120
            },
            "summary":{"total_runs":4,"passed":3,"failed":1,"skipped":0,"pass_rate":0.75},
            "runs":[
                {
                    "model":"qwen2.5-coder-7b-instruct@q8_0",
                    "task_id":"task-a",
                    "run_index":0,
                    "run_id":"run-a",
                    "exit_reason":"completed",
                    "status":"passed",
                    "required_flags":[],
                    "passed":true,
                    "failures":[],
                    "stats":{"steps":10,"tool_calls":4},
                    "metrics":{"steps":10,"tool_calls":4,"tool_sequence":[],"tool_calls_by_side_effects":{},"bytes_read":100,"bytes_written":50,"wall_time_ms":1200,"verifier_time_ms":200,"provider":{"http_retries":0,"provider_errors":0},"tool_retries":0,"tool_failures_by_class":{},"step_invariant_violations":0},
                    "verifier":{"ran":true,"ok":true,"summary":"ok","stdout_truncated":false,"stderr_truncated":false},
                    "ux_metric_rows":[]
                }
            ],
            "ux_summary_metric_rows":[
                {"key":"ux.task_success_rate","group_name":"ux","value_num":0.75,"direction":"higher_is_better","is_primary":true}
            ]
        }"#,
    )
    .unwrap();

    let result = IngestService::ingest_dir(&paths, ingest_request(results_json.clone())).unwrap();
    let artifact_root = PathBuf::from(result.artifact_root.unwrap());
    let manifest: RunManifestV1 =
        serde_json::from_str(&fs::read_to_string(artifact_root.join("run.json")).unwrap()).unwrap();

    assert_eq!(manifest.source.source_kind, SourceKind::ImportedManifest);
    assert_eq!(manifest.identity.suite.as_deref(), Some("commoncodingux"));
    assert_eq!(
        manifest.identity.label.as_deref(),
        Some("commoncodingux qwen2.5-coder-7b-instruct@q8_0")
    );
    assert_eq!(manifest.runtime.exec_status, ExecStatus::Fail);
    assert!(artifact_root
        .join("raw/results_2026-03-07T22-20-30.0697001Z.json")
        .is_file());
    assert!(manifest
        .adapter_payload
        .get("localagent")
        .and_then(|value| value.get("eval_summary"))
        .is_some());
}

#[test]
fn ingest_videoforge_sample() {
    let temp = tempdir().unwrap();
    let paths = AppPaths {
        db_path: temp.path().join("runscope.sqlite"),
        data_dir: temp.path().join("data"),
    };

    let result =
        IngestService::ingest_dir(&paths, ingest_request(videoforge_fixture_dir())).unwrap();
    let artifact_root = PathBuf::from(result.artifact_root.unwrap());
    let manifest: RunManifestV1 =
        serde_json::from_str(&fs::read_to_string(artifact_root.join("run.json")).unwrap()).unwrap();

    assert_eq!(manifest.project.slug, "videoforge");
    assert_eq!(manifest.metrics.len(), 2);
    assert!(artifact_root.join("logs/stdout.log").is_file());
}

#[test]
fn ingest_faceapp_sample() {
    let temp = tempdir().unwrap();
    let paths = AppPaths {
        db_path: temp.path().join("runscope.sqlite"),
        data_dir: temp.path().join("data"),
    };

    let result = IngestService::ingest_dir(&paths, ingest_request(faceapp_fixture_dir())).unwrap();
    let artifact_root = PathBuf::from(result.artifact_root.unwrap());
    let manifest: RunManifestV1 =
        serde_json::from_str(&fs::read_to_string(artifact_root.join("run.json")).unwrap()).unwrap();

    assert_eq!(manifest.project.slug, "faceapp");
    assert_eq!(manifest.metrics.len(), 2);
    assert!(artifact_root.join("raw/faceapp_benchmark.json").is_file());
}

#[test]
fn manual_record_creation() {
    let temp = tempdir().unwrap();
    let paths = AppPaths {
        db_path: temp.path().join("runscope.sqlite"),
        data_dir: temp.path().join("data"),
    };

    let result = RecordService::record_manual(&paths, manual_record_request(temp.path())).unwrap();
    let artifact_root = PathBuf::from(result.artifact_root.unwrap());
    let manifest: RunManifestV1 =
        serde_json::from_str(&fs::read_to_string(artifact_root.join("run.json")).unwrap()).unwrap();

    assert_eq!(manifest.project.slug, "manual-bench");
    assert_eq!(manifest.source.source_kind, SourceKind::ManualRecord);
    assert_eq!(manifest.runtime.exec_status, ExecStatus::Pass);
    assert_eq!(manifest.metrics.len(), 1);
}

#[test]
fn manual_record_attachment_ingest() {
    let temp = tempdir().unwrap();
    let paths = AppPaths {
        db_path: temp.path().join("runscope.sqlite"),
        data_dir: temp.path().join("data"),
    };

    let result = RecordService::record_manual(&paths, manual_record_request(temp.path())).unwrap();
    let artifact_root = PathBuf::from(result.artifact_root.unwrap());
    let manifest: RunManifestV1 =
        serde_json::from_str(&fs::read_to_string(artifact_root.join("run.json")).unwrap()).unwrap();

    assert!(manifest
        .artifacts
        .iter()
        .any(|artifact| artifact.role == "env_snapshot"
            && artifact.rel_path.starts_with("attachments/")));
    assert!(manifest
        .artifacts
        .iter()
        .any(|artifact| artifact.role == "stdout_log"
            && artifact.rel_path.starts_with("attachments/")));
    assert!(artifact_root.join("attachments").is_dir());
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

fn videoforge_fixture_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("videoforge")
        .join("basic")
}

fn faceapp_fixture_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("faceapp")
        .join("basic")
}

fn manual_record_request(root: &Path) -> ManualRecordRequest {
    let attachment_path = root.join("stdout.log");
    let env_path = root.join("env.redacted.json");
    fs::write(&attachment_path, "manual stdout").unwrap();
    fs::write(&env_path, "{\"TOKEN\":\"redacted\"}").unwrap();

    ManualRecordRequest {
        project_slug: "manual-bench".to_string(),
        project_display_name: Some("Manual Bench".to_string()),
        exec_status: ExecStatus::Pass,
        suite: Some("smoke".to_string()),
        scenario: Some("manual_case".to_string()),
        label: Some("manual run".to_string()),
        commit_sha: Some("abc123".to_string()),
        branch: Some("main".to_string()),
        git_dirty: Some(false),
        machine_name: Some("DEVBOX".to_string()),
        os: Some("Windows 11".to_string()),
        cpu: Some("Ryzen".to_string()),
        gpu: Some("RTX".to_string()),
        backend: Some("cuda".to_string()),
        model: Some("demo-model".to_string()),
        precision: Some("fp16".to_string()),
        dataset: Some("fixture".to_string()),
        input_count: Some(2),
        command_argv: vec!["runscope-demo".to_string(), "--flag".to_string()],
        display_command: Some("runscope-demo --flag".to_string()),
        cwd: Some("C:/work/demo".to_string()),
        env_snapshot_file: Some(env_path),
        metrics: vec![MetricRecord {
            key: "fps".to_string(),
            group_name: String::new(),
            value_num: Some(42.0),
            value_text: None,
            unit: Some("frames/s".to_string()),
            direction: MetricDirection::HigherIsBetter,
            is_primary: true,
        }],
        attachments: vec![ManualAttachment {
            role: "stdout_log".to_string(),
            path: attachment_path,
            media_type: "text/plain".to_string(),
        }],
        note: Some("manual note".to_string()),
        tags: vec!["manual".to_string()],
    }
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
