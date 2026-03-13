use super::{AppPaths, BaselineService, QueryService, RegressionRuleService};
use crate::domain::{
    ArtifactDiff, CompareReport, FieldDiff, MetricDiff, RegressionComparator, RegressionFlag,
    RunManifestV1,
};
use crate::error::RunScopeError;
use std::collections::{BTreeMap, BTreeSet};

pub struct CompareService;

impl CompareService {
    pub fn compare_runs(
        paths: &AppPaths,
        left_run_id: &str,
        right_run_id: &str,
    ) -> Result<CompareReport, RunScopeError> {
        let left = QueryService::get_run(paths, left_run_id)?;
        let right = QueryService::get_run(paths, right_run_id)?;

        Ok(CompareReport {
            left_run_id: left_run_id.to_string(),
            right_run_id: right_run_id.to_string(),
            metadata_diffs: metadata_diffs(&left.manifest, &right.manifest),
            metric_diffs: metric_diffs(&left.manifest, &right.manifest),
            artifact_diffs: artifact_diffs(&left.manifest, &right.manifest),
            regression_flags: regression_flags(paths, &right.manifest)?,
        })
    }
}

fn regression_flags(
    paths: &AppPaths,
    candidate: &RunManifestV1,
) -> Result<Vec<RegressionFlag>, RunScopeError> {
    let baselines = BaselineService::list_baselines(paths, &candidate.project.slug)?;
    let rules = RegressionRuleService::list_rules(paths, &candidate.project.slug)?;
    let scope_hash = crate::domain::ComparisonScope::from_manifest(candidate).scope_hash()?;
    let candidate_metrics: BTreeMap<(&str, &str), f64> = candidate
        .metrics
        .iter()
        .filter_map(|metric| metric.value_num.map(|value| ((metric.group_name.as_str(), metric.key.as_str()), value)))
        .collect();

    let mut flags = Vec::new();
    for baseline in baselines
        .into_iter()
        .filter(|baseline| baseline.scope_hash == scope_hash)
    {
        let baseline_detail = QueryService::get_run(paths, &baseline.run_id)?;
        let baseline_metrics: BTreeMap<(&str, &str), f64> = baseline_detail
            .manifest
            .metrics
            .iter()
            .filter_map(|metric| {
                metric
                    .value_num
                    .map(|value| ((metric.group_name.as_str(), metric.key.as_str()), value))
            })
            .collect();

        for rule in rules
            .iter()
            .filter(|rule| rule.scope_hash == scope_hash && rule.label == baseline.label)
        {
            let baseline_value = baseline_metrics
                .iter()
                .find(|((_, key), _)| *key == rule.metric_key.as_str())
                .map(|(_, value)| *value);
            let candidate_value = candidate_metrics
                .iter()
                .find(|((_, key), _)| *key == rule.metric_key.as_str())
                .map(|(_, value)| *value);
            let (actual_value, status) =
                evaluate_rule(rule.comparator.clone(), rule.threshold_value, baseline_value, candidate_value);
            flags.push(RegressionFlag {
                metric_key: rule.metric_key.clone(),
                comparator: rule.comparator.clone(),
                threshold_value: rule.threshold_value,
                baseline_run_id: baseline.run_id.clone(),
                candidate_run_id: candidate.run_id.clone(),
                actual_value,
                status: status.to_string(),
                label: rule.label.clone(),
            });
        }
    }

    Ok(flags)
}

fn evaluate_rule(
    comparator: RegressionComparator,
    threshold: f64,
    baseline_value: Option<f64>,
    candidate_value: Option<f64>,
) -> (Option<f64>, &'static str) {
    let Some(baseline_value) = baseline_value else {
        return (None, "not_applicable");
    };
    let Some(candidate_value) = candidate_value else {
        return (None, "not_applicable");
    };

    match comparator {
        RegressionComparator::PctDropGt => {
            if baseline_value == 0.0 {
                return (None, "not_applicable");
            }
            let drop_pct = ((baseline_value - candidate_value) / baseline_value) * 100.0;
            (Some(drop_pct), if drop_pct > threshold { "triggered" } else { "ok" })
        }
        RegressionComparator::PctIncreaseGt => {
            if baseline_value == 0.0 {
                return (None, "not_applicable");
            }
            let increase_pct = ((candidate_value - baseline_value) / baseline_value) * 100.0;
            (
                Some(increase_pct),
                if increase_pct > threshold {
                    "triggered"
                } else {
                    "ok"
                },
            )
        }
        RegressionComparator::AbsDeltaGt => {
            let delta = (candidate_value - baseline_value).abs();
            (Some(delta), if delta > threshold { "triggered" } else { "ok" })
        }
        RegressionComparator::AbsDeltaLt => {
            let delta = (candidate_value - baseline_value).abs();
            (Some(delta), if delta < threshold { "triggered" } else { "ok" })
        }
    }
}

fn metadata_diffs(left: &RunManifestV1, right: &RunManifestV1) -> Vec<FieldDiff> {
    let pairs = [
        (
            "project.slug",
            Some(left.project.slug.clone()),
            Some(right.project.slug.clone()),
        ),
        (
            "identity.suite",
            left.identity.suite.clone(),
            right.identity.suite.clone(),
        ),
        (
            "identity.scenario",
            left.identity.scenario.clone(),
            right.identity.scenario.clone(),
        ),
        (
            "identity.label",
            left.identity.label.clone(),
            right.identity.label.clone(),
        ),
        (
            "runtime.exec_status",
            Some(format!("{:?}", left.runtime.exec_status).to_lowercase()),
            Some(format!("{:?}", right.runtime.exec_status).to_lowercase()),
        ),
        (
            "runtime.started_at",
            left.runtime.started_at.clone(),
            right.runtime.started_at.clone(),
        ),
        (
            "runtime.finished_at",
            left.runtime.finished_at.clone(),
            right.runtime.finished_at.clone(),
        ),
        (
            "runtime.duration_ms",
            left.runtime.duration_ms.map(|value| value.to_string()),
            right.runtime.duration_ms.map(|value| value.to_string()),
        ),
        (
            "environment.backend",
            left.environment
                .as_ref()
                .and_then(|value| value.backend.clone()),
            right
                .environment
                .as_ref()
                .and_then(|value| value.backend.clone()),
        ),
        (
            "environment.model",
            left.environment
                .as_ref()
                .and_then(|value| value.model.clone()),
            right
                .environment
                .as_ref()
                .and_then(|value| value.model.clone()),
        ),
        (
            "environment.precision",
            left.environment
                .as_ref()
                .and_then(|value| value.precision.clone()),
            right
                .environment
                .as_ref()
                .and_then(|value| value.precision.clone()),
        ),
        (
            "workload.dataset",
            left.workload
                .as_ref()
                .and_then(|value| value.dataset.clone()),
            right
                .workload
                .as_ref()
                .and_then(|value| value.dataset.clone()),
        ),
        (
            "git.branch",
            left.git.as_ref().and_then(|value| value.branch.clone()),
            right.git.as_ref().and_then(|value| value.branch.clone()),
        ),
        (
            "git.commit_sha",
            left.git.as_ref().and_then(|value| value.commit_sha.clone()),
            right
                .git
                .as_ref()
                .and_then(|value| value.commit_sha.clone()),
        ),
    ];

    pairs
        .into_iter()
        .filter_map(|(field, left, right)| {
            if left == right {
                None
            } else {
                Some(FieldDiff {
                    field: field.to_string(),
                    left,
                    right,
                })
            }
        })
        .collect()
}

fn metric_diffs(left: &RunManifestV1, right: &RunManifestV1) -> Vec<MetricDiff> {
    let left_metrics: BTreeMap<(String, String), _> = left
        .metrics
        .iter()
        .map(|metric| ((metric.group_name.clone(), metric.key.clone()), metric))
        .collect();
    let right_metrics: BTreeMap<(String, String), _> = right
        .metrics
        .iter()
        .map(|metric| ((metric.group_name.clone(), metric.key.clone()), metric))
        .collect();

    let keys: BTreeSet<(String, String)> = left_metrics
        .keys()
        .cloned()
        .chain(right_metrics.keys().cloned())
        .collect();

    keys.into_iter()
        .map(|(group_name, key)| {
            let left_metric = left_metrics.get(&(group_name.clone(), key.clone()));
            let right_metric = right_metrics.get(&(group_name.clone(), key.clone()));
            let left_num = left_metric.and_then(|metric| metric.value_num);
            let right_num = right_metric.and_then(|metric| metric.value_num);
            let abs_delta = match (left_num, right_num) {
                (Some(left_num), Some(right_num)) => Some(right_num - left_num),
                _ => None,
            };
            let pct_delta = match (left_num, right_num) {
                (Some(left_num), Some(right_num)) if left_num != 0.0 => {
                    Some(((right_num - left_num) / left_num) * 100.0)
                }
                _ => None,
            };

            MetricDiff {
                key,
                group_name,
                left_num,
                right_num,
                left_text: left_metric.and_then(|metric| metric.value_text.clone()),
                right_text: right_metric.and_then(|metric| metric.value_text.clone()),
                unit: left_metric
                    .and_then(|metric| metric.unit.clone())
                    .or_else(|| right_metric.and_then(|metric| metric.unit.clone())),
                direction: left_metric
                    .map(|metric| metric.direction.clone())
                    .or_else(|| right_metric.map(|metric| metric.direction.clone()))
                    .expect("metric must exist on one side"),
                abs_delta,
                pct_delta,
            }
        })
        .collect()
}

fn artifact_diffs(left: &RunManifestV1, right: &RunManifestV1) -> Vec<ArtifactDiff> {
    let left_artifacts: BTreeMap<String, String> = left
        .artifacts
        .iter()
        .map(|artifact| (artifact.role.clone(), artifact.rel_path.clone()))
        .collect();
    let right_artifacts: BTreeMap<String, String> = right
        .artifacts
        .iter()
        .map(|artifact| (artifact.role.clone(), artifact.rel_path.clone()))
        .collect();

    let roles: BTreeSet<String> = left_artifacts
        .keys()
        .cloned()
        .chain(right_artifacts.keys().cloned())
        .collect();

    roles
        .into_iter()
        .filter_map(|role| {
            let left_rel_path = left_artifacts.get(&role).cloned();
            let right_rel_path = right_artifacts.get(&role).cloned();
            if left_rel_path == right_rel_path {
                None
            } else {
                Some(ArtifactDiff {
                    role,
                    left_rel_path,
                    right_rel_path,
                })
            }
        })
        .collect()
}
