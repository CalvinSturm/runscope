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
