# RunScope v1 PR-Ready Spec

## 0. Decision

Ship this as a **Rust workspace** with one shared core crate and two shells:

* `runscope-core` for domain, adapters, ingest, storage, compare, SQLite
* `runscope-cli` for `runscope ...`
* `apps/desktop/src-tauri` for the Tauri shell
* `ui/` for React/Vite frontend

Use:

* `rusqlite` for SQLite
* `serde` + `schemars` for manifest serialization/schema generation
* `clap` for CLI
* `anyhow` only at shell boundaries
* typed error enum inside core

Do **not** make replay execution part of v1. Store replay metadata only.

---

# 1. Scope and non-goals for this PR set

## In scope

* normalized canonical manifest: `run.json`
* adapter pipeline for:

  * `LocalAgent`
  * `VideoForge`
  * `faceapp`
* manual run recording
* SQLite metadata store
* filesystem artifact store
* run listing
* run detail
* compare view data model
* scoped baselines
* threshold-based regression rules
* notes and tags

## Out of scope

* cloud sync
* distributed execution
* multi-user
* artifact visual diffing
* automatic replay correctness guarantees
* watcher mode in initial PRs

---

# 2. Workspace and exact Rust module boundaries

## Repo layout

```text
runscope/
├─ Cargo.toml
├─ crates/
│  ├─ runscope-core/
│  │  ├─ Cargo.toml
│  │  ├─ migrations/
│  │  │  └─ 0001_init.sql
│  │  └─ src/
│  │     ├─ lib.rs
│  │     ├─ error.rs
│  │     ├─ domain/
│  │     │  ├─ mod.rs
│  │     │  ├─ run_manifest.rs
│  │     │  ├─ scope.rs
│  │     │  ├─ compare.rs
│  │     │  ├─ note.rs
│  │     │  ├─ tag.rs
│  │     │  └─ ids.rs
│  │     ├─ adapters/
│  │     │  ├─ mod.rs
│  │     │  ├─ traits.rs
│  │     │  ├─ detect.rs
│  │     │  ├─ localagent.rs
│  │     │  ├─ videoforge.rs
│  │     │  └─ faceapp.rs
│  │     ├─ store/
│  │     │  ├─ mod.rs
│  │     │  ├─ layout.rs
│  │     │  ├─ artifact_store.rs
│  │     │  └─ hashing.rs
│  │     ├─ db/
│  │     │  ├─ mod.rs
│  │     │  ├─ connection.rs
│  │     │  ├─ migrations.rs
│  │     │  ├─ row_types.rs
│  │     │  └─ repos/
│  │     │     ├─ mod.rs
│  │     │     ├─ projects.rs
│  │     │     ├─ runs.rs
│  │     │     ├─ metrics.rs
│  │     │     ├─ artifacts.rs
│  │     │     ├─ warnings.rs
│  │     │     ├─ notes.rs
│  │     │     ├─ tags.rs
│  │     │     ├─ baselines.rs
│  │     │     └─ regression_rules.rs
│  │     ├─ services/
│  │     │  ├─ mod.rs
│  │     │  ├─ ingest.rs
│  │     │  ├─ record.rs
│  │     │  ├─ query.rs
│  │     │  ├─ compare.rs
│  │     │  ├─ baselines.rs
│  │     │  ├─ notes.rs
│  │     │  └─ tags.rs
│  │     └─ schema/
│  │        ├─ mod.rs
│  │        └─ generate.rs
│  └─ runscope-cli/
│     ├─ Cargo.toml
│     └─ src/
│        ├─ main.rs
│        ├─ cli.rs
│        └─ commands/
│           ├─ mod.rs
│           ├─ ingest.rs
│           ├─ record.rs
│           ├─ list.rs
│           ├─ show.rs
│           ├─ compare.rs
│           ├─ baseline.rs
│           ├─ note.rs
│           └─ tag.rs
├─ apps/
│  └─ desktop/
│     ├─ src-tauri/
│     │  ├─ Cargo.toml
│     │  └─ src/
│     │     ├─ lib.rs
│     │     ├─ state.rs
│     │     └─ commands/
│     │        ├─ mod.rs
│     │        ├─ runs.rs
│     │        ├─ compare.rs
│     │        ├─ baselines.rs
│     │        ├─ notes.rs
│     │        └─ tags.rs
│     └─ ...
└─ ui/
   └─ ...
```

## Boundary rules

### `domain/`

Pure types only.

Must not depend on:

* SQLite
* Tauri
* Clap
* filesystem layout concerns

Contains:

* canonical manifest structs
* enums
* scope definitions
* compare result DTOs
* ID types

### `adapters/`

Convert raw producer artifacts into a canonical in-memory manifest plus file attachments.

May depend on:

* `domain`
* `error`

Must not depend on:

* `db`
* Tauri
* Clap

### `store/`

Owns filesystem artifact layout and hashing.

May depend on:

* `domain`
* `error`

Must not depend on:

* `db`
* adapters

### `db/`

SQLite connection, migrations, row mapping, repositories.

May depend on:

* `domain`
* `error`

Must not depend on:

* adapters
* Tauri
* Clap

### `services/`

Application layer.

Owns:

* ingest orchestration
* dedupe
* DB writes
* artifact copy
* compare execution
* baseline management

Depends on:

* `adapters`
* `store`
* `db`
* `domain`

### `runscope-cli`

Shell only.

Depends on:

* `runscope-core`
* `clap`
* `serde_json`

No direct SQL.

### `src-tauri`

Shell only.

Depends on:

* `runscope-core`
* `tauri`

No direct SQL.

---

# 3. Public Rust API surface

## `runscope-core/src/lib.rs`

```rust
pub mod error;
pub mod domain;
pub mod adapters;
pub mod store;
pub mod db;
pub mod services;
pub mod schema;
```

## Core service entrypoints

These are the public service methods shells call.

```rust
pub struct AppPaths {
    pub db_path: std::path::PathBuf,
    pub data_dir: std::path::PathBuf,
}

pub struct IngestService;
impl IngestService {
    pub fn ingest_dir(
        paths: &AppPaths,
        req: IngestRequest,
    ) -> Result<IngestResult, RunScopeError>;
}

pub struct RecordService;
impl RecordService {
    pub fn record_manual(
        paths: &AppPaths,
        req: ManualRecordRequest,
    ) -> Result<RecordResult, RunScopeError>;
}

pub struct QueryService;
impl QueryService {
    pub fn list_runs(
        paths: &AppPaths,
        filter: RunListFilter,
    ) -> Result<RunListPage, RunScopeError>;

    pub fn get_run(
        paths: &AppPaths,
        run_id: &str,
    ) -> Result<RunDetail, RunScopeError>;
}

pub struct CompareService;
impl CompareService {
    pub fn compare_runs(
        paths: &AppPaths,
        left_run_id: &str,
        right_run_id: &str,
    ) -> Result<CompareReport, RunScopeError>;
}

pub struct BaselineService;
impl BaselineService {
    pub fn set_active_baseline(
        paths: &AppPaths,
        req: SetBaselineRequest,
    ) -> Result<BaselineBinding, RunScopeError>;

    pub fn list_baselines(
        paths: &AppPaths,
        project_slug: &str,
    ) -> Result<Vec<BaselineBinding>, RunScopeError>;
}

pub struct NoteService;
impl NoteService {
    pub fn add_note(
        paths: &AppPaths,
        req: AddNoteRequest,
    ) -> Result<NoteRecord, RunScopeError>;
}

pub struct TagService;
impl TagService {
    pub fn add_tags(
        paths: &AppPaths,
        req: AddTagsRequest,
    ) -> Result<Vec<String>, RunScopeError>;

    pub fn remove_tags(
        paths: &AppPaths,
        req: RemoveTagsRequest,
    ) -> Result<Vec<String>, RunScopeError>;
}
```

---

# 4. Canonical `run.json` schema

## Schema version

```rust
pub const RUN_SCHEMA_VERSION: &str = "runscope.run.v1";
```

## Exact Rust types

Place in `crates/runscope-core/src/domain/run_manifest.rs`.

```rust
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecStatus {
    Pass,
    Fail,
    Error,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    ArtifactDir,
    ManualRecord,
    ImportedManifest,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MetricDirection {
    HigherIsBetter,
    LowerIsBetter,
    Target,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RunManifestV1 {
    pub schema_version: String,
    pub run_id: String,
    pub project: ProjectRef,
    pub source: RunSource,
    pub identity: RunIdentity,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git: Option<GitContext>,
    pub runtime: RuntimeContext,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment: Option<EnvironmentContext>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workload: Option<WorkloadContext>,
    pub summary: SummaryContext,
    #[serde(default)]
    pub metrics: Vec<MetricRecord>,
    #[serde(default)]
    pub artifacts: Vec<ArtifactRecord>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub adapter_payload: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProjectRef {
    pub slug: String,
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RunSource {
    pub adapter: String,
    pub source_kind: SourceKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_run_id: Option<String>,
    pub ingested_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct RunIdentity {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suite: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scenario: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct GitContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit_sha: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dirty: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RuntimeContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    pub exec_status: ExecStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct EnvironmentContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub machine_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub os: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpu: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub precision: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct WorkloadContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dataset: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_count: Option<u64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub command_argv: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env_snapshot_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SummaryContext {
    pub error_count: u32,
    pub warning_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MetricRecord {
    pub key: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub group_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_num: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    pub direction: MetricDirection,
    #[serde(default)]
    pub is_primary: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ArtifactRecord {
    pub role: String,
    pub rel_path: String,
    pub media_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
}
```

## Validation rules

Implement in `domain/run_manifest.rs` as `impl RunManifestV1 { pub fn validate(&self) -> Result<(), RunScopeError> }`.

Required rules:

1. `schema_version == "runscope.run.v1"`
2. `run_id` non-empty
3. `project.slug` non-empty
4. `source.adapter` non-empty
5. `source.ingested_at` must be RFC3339 UTC
6. `started_at` and `finished_at`, if present, must be RFC3339 UTC
7. `duration_ms`, if present, must be non-negative
8. each metric must have either `value_num` or `value_text`
9. `artifacts[*].rel_path` must be relative, never absolute
10. `artifacts[*].role` must be lower snake case
11. `workload.env_snapshot_ref`, if present, must be relative
12. `adapter_payload` keys must be adapter names only, one key max for v1

## Recommended normalized artifact roles

Not hard-enforced enum in v1, but adapters should prefer:

* `stdout_log`
* `stderr_log`
* `report_json`
* `report_html`
* `screenshot`
* `video`
* `input_manifest`
* `env_snapshot`
* `raw_source_manifest`
* `replay_script`

## Example `run.json`

```json
{
  "schema_version": "runscope.run.v1",
  "run_id": "01JNP8M2A4HD7Q7RAN6TKPS9YF",
  "project": {
    "slug": "videoforge",
    "display_name": "VideoForge"
  },
  "source": {
    "adapter": "videoforge",
    "source_kind": "artifact_dir",
    "source_path": "C:/work/videoforge/artifacts/run_20260305_001",
    "external_run_id": "vf-20260305-001",
    "ingested_at": "2026-03-05T17:20:31Z"
  },
  "identity": {
    "suite": "perf_smoke",
    "scenario": "esrgan_x4_fp16",
    "label": "main rtx4090 smoke"
  },
  "git": {
    "commit_sha": "abc123def456",
    "branch": "main",
    "dirty": false
  },
  "runtime": {
    "started_at": "2026-03-05T17:17:04Z",
    "finished_at": "2026-03-05T17:18:11Z",
    "duration_ms": 67000,
    "exit_code": 0,
    "exec_status": "pass"
  },
  "environment": {
    "machine_name": "DESKTOP-01",
    "os": "Windows 11",
    "cpu": "Ryzen 9 7950X",
    "gpu": "RTX 4090",
    "backend": "tensorrt",
    "model": "realesrgan-x4plus",
    "precision": "fp16"
  },
  "workload": {
    "dataset": "smoke_set_v1",
    "input_count": 3,
    "command_argv": [
      "videoforge",
      "run",
      "--backend",
      "tensorrt"
    ],
    "display_command": "videoforge run --backend tensorrt",
    "cwd": "C:/work/videoforge",
    "env_snapshot_ref": "artifacts/env.redacted.json"
  },
  "summary": {
    "error_count": 0,
    "warning_count": 1
  },
  "metrics": [
    {
      "key": "fps",
      "group_name": "",
      "value_num": 42.1,
      "unit": "frames/s",
      "direction": "higher_is_better",
      "is_primary": true
    },
    {
      "key": "latency_p50_ms",
      "group_name": "",
      "value_num": 18.4,
      "unit": "ms",
      "direction": "lower_is_better",
      "is_primary": true
    }
  ],
  "artifacts": [
    {
      "role": "stdout_log",
      "rel_path": "logs/stdout.log",
      "media_type": "text/plain",
      "sha256": "abc...",
      "size_bytes": 18294
    },
    {
      "role": "report_json",
      "rel_path": "derived/report.json",
      "media_type": "application/json",
      "sha256": "def...",
      "size_bytes": 814
    }
  ],
  "adapter_payload": {
    "videoforge": {
      "engine": "v2",
      "pipeline": "nvdec_cuda_tensorrt_nvenc"
    }
  }
}
```

## Generated JSON Schema file

Add a schema generator in `schema/generate.rs` and commit the generated file:

```text
crates/runscope-core/schema/run.v1.schema.json
```

Rule:

* schema file is generated from `RunManifestV1` using `schemars`
* CI fails if generated schema and committed schema diverge

---

# 5. Scope model for baselines and regressions

Place in `domain/scope.rs`.

```rust
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct ComparisonScope {
    pub branch: Option<String>,
    pub suite: Option<String>,
    pub scenario: Option<String>,
    pub backend: Option<String>,
    pub model: Option<String>,
    pub precision: Option<String>,
    pub dataset: Option<String>,
}
```

Normalization rule:

* serialize `ComparisonScope` to canonical JSON with stable key order
* `scope_hash = sha256(scope_json)`
* same scope JSON must always produce same hash

---

# 6. SQLite DDL

Place in:

```text
crates/runscope-core/migrations/0001_init.sql
```

## Exact DDL

```sql
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS projects (
    id INTEGER PRIMARY KEY,
    slug TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS runs (
    id TEXT PRIMARY KEY,
    project_id INTEGER NOT NULL,
    schema_version TEXT NOT NULL,
    adapter_kind TEXT NOT NULL,
    source_kind TEXT NOT NULL CHECK (source_kind IN ('artifact_dir', 'manual_record', 'imported_manifest')),
    source_path TEXT,
    external_run_id TEXT,
    suite TEXT,
    scenario TEXT,
    label TEXT,
    exec_status TEXT NOT NULL CHECK (exec_status IN ('pass', 'fail', 'error', 'unknown')),
    started_at TEXT,
    finished_at TEXT,
    duration_ms INTEGER,
    exit_code INTEGER,
    git_commit_sha TEXT,
    git_branch TEXT,
    git_dirty INTEGER NOT NULL DEFAULT 0 CHECK (git_dirty IN (0, 1)),
    machine_name TEXT,
    os TEXT,
    cpu TEXT,
    gpu TEXT,
    backend TEXT,
    model TEXT,
    precision TEXT,
    dataset TEXT,
    input_count INTEGER,
    command_json TEXT,
    display_command TEXT,
    cwd TEXT,
    env_snapshot_rel_path TEXT,
    raw_manifest_rel_path TEXT,
    error_count INTEGER NOT NULL DEFAULT 0,
    warning_count INTEGER NOT NULL DEFAULT 0,
    ingest_fingerprint TEXT NOT NULL UNIQUE,
    source_hash TEXT,
    ingested_at TEXT NOT NULL,
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE RESTRICT
);

CREATE INDEX IF NOT EXISTS idx_runs_project_started_at
ON runs(project_id, started_at DESC);

CREATE INDEX IF NOT EXISTS idx_runs_project_branch
ON runs(project_id, git_branch);

CREATE INDEX IF NOT EXISTS idx_runs_project_backend_model_precision
ON runs(project_id, backend, model, precision);

CREATE INDEX IF NOT EXISTS idx_runs_exec_status
ON runs(exec_status);

CREATE INDEX IF NOT EXISTS idx_runs_git_commit
ON runs(git_commit_sha);

CREATE TABLE IF NOT EXISTS metrics (
    id INTEGER PRIMARY KEY,
    run_id TEXT NOT NULL,
    key TEXT NOT NULL,
    group_name TEXT NOT NULL DEFAULT '',
    value_num REAL,
    value_text TEXT,
    unit TEXT,
    direction TEXT NOT NULL CHECK (direction IN ('higher_is_better', 'lower_is_better', 'target', 'none')),
    is_primary INTEGER NOT NULL DEFAULT 0 CHECK (is_primary IN (0, 1)),
    display_order INTEGER NOT NULL DEFAULT 0,
    CHECK (value_num IS NOT NULL OR value_text IS NOT NULL),
    FOREIGN KEY (run_id) REFERENCES runs(id) ON DELETE CASCADE,
    UNIQUE (run_id, key, group_name)
);

CREATE INDEX IF NOT EXISTS idx_metrics_run_primary
ON metrics(run_id, is_primary DESC, display_order ASC, key ASC);

CREATE INDEX IF NOT EXISTS idx_metrics_key
ON metrics(key);

CREATE TABLE IF NOT EXISTS artifacts (
    id INTEGER PRIMARY KEY,
    run_id TEXT NOT NULL,
    role TEXT NOT NULL,
    rel_path TEXT NOT NULL,
    media_type TEXT NOT NULL,
    sha256 TEXT,
    size_bytes INTEGER,
    created_at TEXT NOT NULL,
    FOREIGN KEY (run_id) REFERENCES runs(id) ON DELETE CASCADE,
    UNIQUE (run_id, rel_path)
);

CREATE INDEX IF NOT EXISTS idx_artifacts_run_role
ON artifacts(run_id, role);

CREATE TABLE IF NOT EXISTS run_warnings (
    id INTEGER PRIMARY KEY,
    run_id TEXT NOT NULL,
    code TEXT NOT NULL,
    message TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (run_id) REFERENCES runs(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_run_warnings_run
ON run_warnings(run_id);

CREATE TABLE IF NOT EXISTS notes (
    id INTEGER PRIMARY KEY,
    run_id TEXT NOT NULL,
    body TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (run_id) REFERENCES runs(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_notes_run_created
ON notes(run_id, created_at DESC);

CREATE TABLE IF NOT EXISTS tags (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE
);

CREATE TABLE IF NOT EXISTS run_tags (
    run_id TEXT NOT NULL,
    tag_id INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    PRIMARY KEY (run_id, tag_id),
    FOREIGN KEY (run_id) REFERENCES runs(id) ON DELETE CASCADE,
    FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_run_tags_tag
ON run_tags(tag_id);

CREATE TABLE IF NOT EXISTS baseline_bindings (
    id INTEGER PRIMARY KEY,
    project_id INTEGER NOT NULL,
    label TEXT NOT NULL DEFAULT 'default',
    scope_json TEXT NOT NULL,
    scope_hash TEXT NOT NULL,
    run_id TEXT NOT NULL,
    active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1)),
    created_at TEXT NOT NULL,
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
    FOREIGN KEY (run_id) REFERENCES runs(id) ON DELETE CASCADE
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_baseline_bindings_active_unique
ON baseline_bindings(project_id, label, scope_hash)
WHERE active = 1;

CREATE INDEX IF NOT EXISTS idx_baseline_bindings_lookup
ON baseline_bindings(project_id, scope_hash, active);

CREATE TABLE IF NOT EXISTS regression_rules (
    id INTEGER PRIMARY KEY,
    project_id INTEGER NOT NULL,
    label TEXT NOT NULL DEFAULT 'default',
    scope_json TEXT NOT NULL,
    scope_hash TEXT NOT NULL,
    metric_key TEXT NOT NULL,
    comparator TEXT NOT NULL CHECK (comparator IN ('pct_drop_gt', 'pct_increase_gt', 'abs_delta_gt', 'abs_delta_lt')),
    threshold_value REAL NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
    created_at TEXT NOT NULL,
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_regression_rules_lookup
ON regression_rules(project_id, scope_hash, enabled, metric_key);
```

## Storage decisions encoded by this DDL

* `runs.id` is a **ULID string**
* timestamps are RFC3339 UTC stored as `TEXT`
* `command_json` stores `Vec<String>` serialized JSON
* baseline is **scoped**, not global
* regression findings are computed on read, not materialized in v1

---

# 7. Artifact store layout

Place under the configured data dir.

```text
<data_dir>/
├─ runscope.sqlite
└─ artifacts/
   └─ <project-slug>/
      └─ <YYYY>/
         └─ <MM>/
            └─ <run-id>/
               ├─ run.json
               ├─ raw/
               ├─ derived/
               ├─ logs/
               └─ attachments/
```

## Rules

* RunScope always writes canonical `run.json`
* raw producer files are copied into `raw/`
* normalized or derived files go in `derived/`
* logs go in `logs/`
* attached manual files go in `attachments/`
* all `artifacts.rel_path` values are relative to the run root
* default ingest mode is **copy**
* no hardlink/symlink mode in first PRs

---

# 8. Adapter contract

Place in `adapters/traits.rs`.

```rust
use std::path::Path;

pub trait RunAdapter {
    fn name(&self) -> &'static str;
    fn detect(&self, artifact_dir: &Path) -> Result<bool, RunScopeError>;
    fn parse(&self, artifact_dir: &Path) -> Result<ParsedRun, RunScopeError>;
}

pub struct ParsedRun {
    pub manifest: RunManifestV1,
    pub files_to_copy: Vec<SourceFile>,
    pub warnings: Vec<AdapterWarning>,
}

pub struct SourceFile {
    pub source_path: std::path::PathBuf,
    pub target_rel_path: String,
    pub role: String,
    pub media_type: String,
}

pub struct AdapterWarning {
    pub code: String,
    pub message: String,
}
```

## Detection order

Implement in `adapters/detect.rs`.

Order:

1. explicit adapter if user provided `--adapter`
2. `LocalAgent`
3. `VideoForge`
4. `faceapp`

If zero matches:

* return `RUN_ADAPTER_NOT_DETECTED`

If multiple match:

* return `RUN_ADAPTER_AMBIGUOUS`

## Adapter output rules

Adapters must:

* preserve raw source reports as copied artifacts
* fill known normalized fields
* leave unknown fields as `None`
* emit warnings instead of failing for non-fatal missing metadata
* never write DB rows directly
* never write files directly

---

# 9. Ingest flow

Implement in `services/ingest.rs`.

## Ingest request

```rust
pub struct IngestRequest {
    pub artifact_dir: std::path::PathBuf,
    pub adapter: Option<String>,
    pub project_override: Option<String>,
    pub label_override: Option<String>,
    pub tags: Vec<String>,
    pub note: Option<String>,
    pub dry_run: bool,
}
```

## Ingest algorithm

1. open DB connection
2. run migrations
3. detect or instantiate adapter
4. parse artifact dir into `ParsedRun`
5. apply overrides
6. generate `run_id` if absent
7. validate manifest
8. compute `source_hash`
9. compute `ingest_fingerprint`
10. if fingerprint already exists:

* return existing run id
* do not duplicate rows or files

11. create project if missing
12. create artifact directory
13. copy source files into managed store
14. compute hashes and sizes
15. write canonical `run.json`
16. insert DB rows in one transaction
17. attach tags/note if provided
18. return `IngestResult`

## Dedupe rule

`ingest_fingerprint = sha256(canonical-json({adapter, project_slug, external_run_id, source_hash, started_at, finished_at, suite, scenario, label}))`

Purpose:

* duplicate ingest of same artifact folder becomes idempotent

---

# 10. Manual record flow

Implement in `services/record.rs`.

## Manual record request

```rust
pub struct ManualRecordRequest {
    pub project_slug: String,
    pub project_display_name: Option<String>,
    pub exec_status: ExecStatus,
    pub suite: Option<String>,
    pub scenario: Option<String>,
    pub label: Option<String>,
    pub commit_sha: Option<String>,
    pub branch: Option<String>,
    pub git_dirty: Option<bool>,
    pub machine_name: Option<String>,
    pub os: Option<String>,
    pub cpu: Option<String>,
    pub gpu: Option<String>,
    pub backend: Option<String>,
    pub model: Option<String>,
    pub precision: Option<String>,
    pub dataset: Option<String>,
    pub input_count: Option<u64>,
    pub command_argv: Vec<String>,
    pub display_command: Option<String>,
    pub cwd: Option<String>,
    pub env_snapshot_file: Option<std::path::PathBuf>,
    pub metrics: Vec<MetricRecord>,
    pub attachments: Vec<ManualAttachment>,
    pub note: Option<String>,
    pub tags: Vec<String>,
}
```

```rust
pub struct ManualAttachment {
    pub role: String,
    pub path: std::path::PathBuf,
    pub media_type: String,
}
```

---

# 11. Query and compare DTOs

Place in `domain/compare.rs`.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunListFilter {
    pub project: Option<String>,
    pub branch: Option<String>,
    pub backend: Option<String>,
    pub model: Option<String>,
    pub precision: Option<String>,
    pub exec_status: Option<ExecStatus>,
    pub tags: Vec<String>,
    pub limit: u32,
    pub offset: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunListItem {
    pub run_id: String,
    pub project_slug: String,
    pub suite: Option<String>,
    pub scenario: Option<String>,
    pub label: Option<String>,
    pub exec_status: ExecStatus,
    pub started_at: Option<String>,
    pub duration_ms: Option<u64>,
    pub git_branch: Option<String>,
    pub git_commit_sha: Option<String>,
    pub backend: Option<String>,
    pub model: Option<String>,
    pub precision: Option<String>,
    pub primary_metrics: Vec<MetricRecord>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunDetail {
    pub manifest: RunManifestV1,
    pub warnings: Vec<WarningRecord>,
    pub notes: Vec<NoteRecord>,
    pub tags: Vec<String>,
    pub active_baselines: Vec<BaselineBinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompareReport {
    pub left_run_id: String,
    pub right_run_id: String,
    pub metadata_diffs: Vec<FieldDiff>,
    pub metric_diffs: Vec<MetricDiff>,
    pub artifact_diffs: Vec<ArtifactDiff>,
    pub regression_flags: Vec<RegressionFlag>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDiff {
    pub field: String,
    pub left: Option<String>,
    pub right: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricDiff {
    pub key: String,
    pub group_name: String,
    pub left_num: Option<f64>,
    pub right_num: Option<f64>,
    pub left_text: Option<String>,
    pub right_text: Option<String>,
    pub unit: Option<String>,
    pub direction: MetricDirection,
    pub abs_delta: Option<f64>,
    pub pct_delta: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactDiff {
    pub role: String,
    pub left_rel_path: Option<String>,
    pub right_rel_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegressionFlag {
    pub metric_key: String,
    pub comparator: String,
    pub threshold_value: f64,
    pub baseline_run_id: String,
    pub candidate_run_id: String,
    pub actual_value: f64,
    pub status: String,
}
```

---

# 12. CLI command definitions

Place in `runscope-cli/src/cli.rs`.

## Top-level command

```text
runscope [OPTIONS] <COMMAND>
```

## Global options

```text
--db <PATH>          Override sqlite path
--data-dir <PATH>    Override data root
--json               Emit structured JSON output
```

If `--db` omitted:

* use `<data-dir>/runscope.sqlite`

If `--data-dir` omitted:

* use platform default app data dir

---

## `ingest`

```text
runscope ingest <ARTIFACT_DIR> [OPTIONS]
```

### Options

```text
--adapter <auto|localagent|videoforge|faceapp>   default: auto
--project-override <SLUG>
--label <TEXT>
--tag <TAG>                                      repeatable
--note <TEXT>
--dry-run
```

### Behavior

* detects adapter
* parses source artifact dir
* validates manifest
* writes canonical `run.json`
* copies artifacts
* persists rows
* returns `run_id`

### JSON output

```json
{
  "run_id": "01JNP8M2A4HD7Q7RAN6TKPS9YF",
  "project_slug": "videoforge",
  "adapter": "videoforge",
  "ingest_fingerprint": "sha256...",
  "artifact_root": "...",
  "warnings": []
}
```

---

## `record`

```text
runscope record [OPTIONS]
```

### Required options

```text
--project <SLUG>
--status <pass|fail|error|unknown>
```

### Optional metadata

```text
--project-name <DISPLAY_NAME>
--suite <TEXT>
--scenario <TEXT>
--label <TEXT>
--commit-sha <SHA>
--branch <BRANCH>
--git-dirty
--machine <TEXT>
--os <TEXT>
--cpu <TEXT>
--gpu <TEXT>
--backend <TEXT>
--model <TEXT>
--precision <TEXT>
--dataset <TEXT>
--input-count <N>
--argv <ARG>                 repeatable, preserves exact argv order
--display-command <TEXT>
--cwd <PATH>
--env-file <PATH>
--metric <KEY=VALUE>         repeatable, numeric only in v1
--artifact <ROLE=PATH>       repeatable
--tag <TAG>                  repeatable
--note <TEXT>
```

### Parsing rules

* `--argv` may be repeated zero or more times
* `--metric fps=42.1` creates:

  * `key = "fps"`
  * `value_num = 42.1`
  * `direction = none`
* `--artifact stdout_log=./out.log` infers media type from extension when possible, else `application/octet-stream`

### JSON output

same shape as `ingest`.

---

## `list`

```text
runscope list [OPTIONS]
```

### Options

```text
--project <SLUG>
--branch <BRANCH>
--backend <TEXT>
--model <TEXT>
--precision <TEXT>
--status <pass|fail|error|unknown>
--tag <TAG>                  repeatable
--limit <N>                  default: 50
--offset <N>                 default: 0
```

### Default sort

* `started_at DESC NULLS LAST`
* then `ingested_at DESC`

### JSON output

```json
{
  "items": [],
  "limit": 50,
  "offset": 0,
  "total": 123
}
```

---

## `show`

```text
runscope show <RUN_ID>
```

Returns full manifest, warnings, notes, tags, baseline bindings.

---

## `compare`

```text
runscope compare <LEFT_RUN_ID> <RIGHT_RUN_ID>
```

### Behavior

* compares metadata fields
* compares overlapping metrics
* computes:

  * absolute delta
  * percent delta when both numeric and left non-zero
* includes regression flags if right run violates any active rule against applicable baseline

---

## `baseline set`

```text
runscope baseline set <RUN_ID> [OPTIONS]
```

### Options

```text
--label <TEXT>               default: default
--scope-branch <BRANCH>
--scope-suite <TEXT>
--scope-scenario <TEXT>
--scope-backend <TEXT>
--scope-model <TEXT>
--scope-precision <TEXT>
--scope-dataset <TEXT>
```

### Behavior

* builds canonical `ComparisonScope`
* deactivates existing active binding for `(project, label, scope_hash)`
* inserts new active binding

---

## `baseline list`

```text
runscope baseline list --project <SLUG>
```

---

## `note add`

```text
runscope note add <RUN_ID> --text <TEXT>
```

---

## `tag add`

```text
runscope tag add <RUN_ID> <TAG>...
```

## `tag remove`

```text
runscope tag remove <RUN_ID> <TAG>...
```

---

# 13. Error model

Place in `error.rs`.

```rust
#[derive(thiserror::Error, Debug)]
pub enum RunScopeError {
    #[error("adapter not detected")]
    AdapterNotDetected,
    #[error("adapter ambiguous")]
    AdapterAmbiguous,
    #[error("manifest validation failed: {0}")]
    ManifestValidation(String),
    #[error("duplicate ingest: existing run {0}")]
    DuplicateIngest(String),
    #[error("run not found: {0}")]
    RunNotFound(String),
    #[error("baseline scope invalid: {0}")]
    BaselineScopeInvalid(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}
```

CLI exit codes:

* `0` success
* `2` invalid args
* `3` adapter detection/parsing failure
* `4` manifest validation failure
* `5` DB/storage failure
* `6` not found

---

# 14. Tauri command boundary

Tauri should expose only application DTOs.

## `src-tauri/src/commands/runs.rs`

* `list_runs(filter) -> RunListPage`
* `get_run(run_id) -> RunDetail`

## `src-tauri/src/commands/compare.rs`

* `compare_runs(left_run_id, right_run_id) -> CompareReport`

## `src-tauri/src/commands/baselines.rs`

* `set_active_baseline(req) -> BaselineBinding`
* `list_baselines(project_slug) -> Vec<BaselineBinding>`

## `src-tauri/src/commands/notes.rs`

* `add_note(req) -> NoteRecord`

## `src-tauri/src/commands/tags.rs`

* `add_tags(req) -> Vec<String>`
* `remove_tags(req) -> Vec<String>`

Rule:

* no Tauri command returns raw SQL rows
* all Tauri commands consume/return `domain` or `services` DTOs only

---

# 15. Suggested PR slicing

## PR1: Core domain + migrations + LocalAgent ingest

Includes:

* workspace scaffold
* `runscope-core`
* migration `0001_init.sql`
* `RunManifestV1`
* schema generation
* artifact store layout
* LocalAgent adapter
* `runscope ingest`
* duplicate ingest handling

Acceptance:

* ingest one LocalAgent artifact dir end-to-end
* `run.json` written
* DB rows inserted
* duplicate ingest returns existing run id

## PR2: VideoForge + faceapp adapters + manual record

Includes:

* VideoForge adapter
* faceapp adapter
* `runscope record`
* artifact attachments for manual records

Acceptance:

* three adapters ingest real samples
* manual record creates valid run + artifacts

## PR3: Query/read path

Includes:

* `list_runs`
* `get_run`
* `runscope list`
* `runscope show`
* notes/tags repositories + CLI

Acceptance:

* filters work
* detail includes notes/tags/warnings/artifacts

## PR4: Compare + baselines + rules

Includes:

* compare DTOs
* metric diff logic
* baseline bindings
* regression rules lookup
* `runscope compare`
* `runscope baseline set/list`

Acceptance:

* compare works on numeric metrics
* scoped baseline replacement works
* regression flags appear when thresholds crossed

## PR5: Tauri shell wiring + initial UI

Includes:

* Tauri commands
* runs list
* run detail
* compare view

Acceptance:

* desktop UI can browse and compare real data

---

# 16. Test plan

## Unit tests

* `run_manifest_validate_rejects_absolute_artifact_paths`
* `run_manifest_validate_rejects_missing_metric_values`
* `comparison_scope_hash_is_stable`
* `metric_diff_computes_abs_and_pct_delta`
* `artifact_store_layout_is_deterministic`

## Integration tests

* `ingest_localagent_sample`
* `ingest_videoforge_sample`
* `ingest_faceapp_sample`
* `manual_record_with_attachments`
* `duplicate_ingest_returns_existing_run`
* `baseline_set_replaces_active_binding_same_scope`
* `compare_report_flags_regression_against_active_baseline`
* `sqlite_migration_bootstrap_empty_db`

## Golden tests

Commit small fixture directories under `tests/fixtures/` and assert generated `run.json` contents.

---

# 17. Final implementation notes

## Use `rusqlite`, not `sqlx`

Reason:

* lower complexity
* better fit for local desktop + CLI
* no async tax in core path

## Generate ULIDs for `runs.id`

Reason:

* sortable
* readable
* safe for local-first IDs

## Store exec result separately from operator tags

Do not overload one field.

* machine outcome: `exec_status`
* human labels: tags like `baseline`, `investigate`, `regressed`, `important`

## Keep thresholds explicit

Regression rules are data, not UI-only state.

## Keep replay metadata passive

Store:

* argv
* display command
* cwd
* env snapshot ref

Do not promise deterministic re-execution in v1.

---

# 18. The first files I would create

```text
crates/runscope-core/src/domain/run_manifest.rs
crates/runscope-core/migrations/0001_init.sql
crates/runscope-core/src/adapters/traits.rs
crates/runscope-core/src/services/ingest.rs
crates/runscope-cli/src/cli.rs
crates/runscope-cli/src/commands/ingest.rs
```