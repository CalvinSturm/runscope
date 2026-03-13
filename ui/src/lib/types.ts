export type ExecStatus = "pass" | "fail" | "error" | "unknown";
export type MetricDirection =
  | "higher_is_better"
  | "lower_is_better"
  | "target"
  | "none";

export interface MetricRecord {
  key: string;
  group_name: string;
  value_num: number | null;
  value_text: string | null;
  unit: string | null;
  direction: MetricDirection;
  is_primary: boolean;
}

export interface ArtifactRecord {
  role: string;
  rel_path: string;
  media_type: string;
  sha256: string | null;
  size_bytes: number | null;
}

export interface RunManifest {
  schema_version: string;
  run_id: string;
  project: {
    slug: string;
    display_name: string;
  };
  source: {
    adapter: string;
    source_kind: string;
    source_path: string | null;
    external_run_id: string | null;
    ingested_at: string;
  };
  identity: {
    suite: string | null;
    scenario: string | null;
    label: string | null;
  };
  git?: {
    commit_sha: string | null;
    branch: string | null;
    dirty: boolean | null;
  } | null;
  runtime: {
    started_at: string | null;
    finished_at: string | null;
    duration_ms: number | null;
    exit_code: number | null;
    exec_status: ExecStatus;
  };
  environment?: {
    machine_name: string | null;
    os: string | null;
    cpu: string | null;
    gpu: string | null;
    backend: string | null;
    model: string | null;
    precision: string | null;
  } | null;
  workload?: {
    dataset: string | null;
    input_count: number | null;
    command_argv: string[];
    display_command: string | null;
    cwd: string | null;
    env_snapshot_ref: string | null;
  } | null;
  summary: {
    error_count: number;
    warning_count: number;
  };
  metrics: MetricRecord[];
  artifacts: ArtifactRecord[];
}

export interface RunListFilter {
  project?: string;
  suite?: string;
  scenario?: string;
  backend?: string;
  model?: string;
  precision?: string;
  exec_status?: ExecStatus;
  query_text?: string;
  limit?: number;
  offset?: number;
}

export interface RunListItem {
  run_id: string;
  project_slug: string;
  adapter: string;
  suite: string | null;
  scenario: string | null;
  label: string | null;
  exec_status: ExecStatus;
  started_at: string | null;
  finished_at: string | null;
  duration_ms: number | null;
  backend: string | null;
  model: string | null;
  precision: string | null;
  warning_count: number;
  primary_metrics: MetricRecord[];
  tags: string[];
}

export interface RunListPage {
  items: RunListItem[];
  total: number;
  limit: number;
  offset: number;
}

export interface WarningRecord {
  code: string;
  message: string;
  created_at: string;
}

export interface NoteRecord {
  id: number;
  body: string;
  created_at: string;
}

export interface RunDetail {
  run_root: string;
  manifest: RunManifest;
  warnings: WarningRecord[];
  notes: NoteRecord[];
  tags: string[];
  active_baselines: BaselineBinding[];
}

export interface ComparisonScope {
  branch: string | null;
  suite: string | null;
  scenario: string | null;
  backend: string | null;
  model: string | null;
  precision: string | null;
  dataset: string | null;
}

export interface BaselineBinding {
  id: number;
  project_slug: string;
  label: string;
  scope: ComparisonScope;
  scope_hash: string;
  run_id: string;
  active: boolean;
  created_at: string;
}

export interface SetBaselineRequest {
  run_id: string;
  label: string;
}

export interface FieldDiff {
  field: string;
  left: string | null;
  right: string | null;
}

export interface MetricDiff {
  key: string;
  group_name: string;
  left_num: number | null;
  right_num: number | null;
  left_text: string | null;
  right_text: string | null;
  unit: string | null;
  direction: MetricDirection;
  abs_delta: number | null;
  pct_delta: number | null;
}

export interface ArtifactDiff {
  role: string;
  left_rel_path: string | null;
  right_rel_path: string | null;
}

export interface CompareReport {
  left_run_id: string;
  right_run_id: string;
  metadata_diffs: FieldDiff[];
  metric_diffs: MetricDiff[];
  artifact_diffs: ArtifactDiff[];
  regression_flags: RegressionFlag[];
}

export type RegressionComparator =
  | "pct_drop_gt"
  | "pct_increase_gt"
  | "abs_delta_gt"
  | "abs_delta_lt";

export interface RegressionRule {
  id: number;
  project_slug: string;
  label: string;
  scope: ComparisonScope;
  scope_hash: string;
  metric_key: string;
  comparator: RegressionComparator;
  threshold_value: number;
  enabled: boolean;
  created_at: string;
}

export interface CreateRegressionRuleRequest {
  run_id: string;
  label: string;
  metric_key: string;
  comparator: RegressionComparator;
  threshold_value: number;
}

export interface RegressionFlag {
  metric_key: string;
  comparator: RegressionComparator;
  threshold_value: number;
  baseline_run_id: string;
  candidate_run_id: string;
  actual_value: number | null;
  status: string;
  label: string;
}
