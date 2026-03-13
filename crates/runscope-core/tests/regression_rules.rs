use runscope_core::domain::{CreateRegressionRuleRequest, RegressionComparator, SetBaselineRequest};
use runscope_core::services::{
    AppPaths, BaselineService, CompareService, IngestRequest, IngestService, RegressionRuleService,
};
use std::path::{Path, PathBuf};
use tempfile::tempdir;

#[test]
fn compare_report_flags_regression_against_active_baseline() {
    let temp = tempdir().unwrap();
    let paths = AppPaths {
        db_path: temp.path().join("runscope.sqlite"),
        data_dir: temp.path().join("data"),
    };

    let baseline =
        IngestService::ingest_dir(&paths, ingest_request(videoforge_fixture_dir())).unwrap();
    BaselineService::set_active_baseline(
        &paths,
        SetBaselineRequest {
            run_id: baseline.run_id.clone(),
            label: "default".to_string(),
        },
    )
    .unwrap();
    RegressionRuleService::create_rule(
        &paths,
        CreateRegressionRuleRequest {
            run_id: baseline.run_id.clone(),
            label: "default".to_string(),
            metric_key: "fps".to_string(),
            comparator: RegressionComparator::PctDropGt,
            threshold_value: 5.0,
        },
    )
    .unwrap();

    let candidate_dir = temp.path().join("videoforge-candidate");
    copy_dir_all(&videoforge_fixture_dir(), &candidate_dir);
    let manifest_path = candidate_dir.join("videoforge_run.json");
    let content = std::fs::read_to_string(&manifest_path).unwrap();
    let updated = content
        .replace("\"run_id\": \"vf-20260305-001\"", "\"run_id\": \"vf-20260305-002\"")
        .replace("\"fps\": 42.1", "\"fps\": 35.0")
        .replace(
            "\"label\": \"main rtx4090 smoke\"",
            "\"label\": \"main rtx4090 smoke candidate\"",
        );
    std::fs::write(&manifest_path, updated).unwrap();

    let candidate = IngestService::ingest_dir(&paths, ingest_request(candidate_dir)).unwrap();
    let report = CompareService::compare_runs(&paths, &baseline.run_id, &candidate.run_id).unwrap();

    assert!(report.regression_flags.iter().any(|flag| {
        flag.metric_key == "fps" && flag.status == "triggered" && flag.label == "default"
    }));
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
