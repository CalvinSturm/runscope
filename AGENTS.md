# AGENTS.md

## TL;DR

RunScope is a local-first run history, benchmark, and reproducibility dashboard for engineering projects.

It ingests artifact folders and manual test runs from systems like `LocalAgent`, `VideoForge`, and `faceapp`, normalizes them into a canonical `run.json` manifest, stores searchable metadata in SQLite, and preserves raw artifacts on disk for inspection, comparison, and later reproduction.

Primary product loop:

1. ingest or record a run
2. normalize it into `runscope.run.v1`
3. persist metadata in SQLite
4. preserve artifacts in the managed store
5. list, inspect, compare, baseline, and tag runs locally

This repo should optimize for:

- deterministic ingest
- explicit schemas
- boring and readable Rust
- local-first operation
- auditable data flow
- strong adapter boundaries
- zero accidental cloud coupling

## Product definition

RunScope is a local-first run registry for engineering artifacts: ingest, normalize, compare, and inspect runs with enough provenance to reproduce them later.

### Core use cases

- ingest run artifacts from `LocalAgent`, `VideoForge`, and `faceapp`
- record manual benchmark or test sessions with structured metadata
- compare runs or compare against a scoped baseline
- track trend lines over time for explicit numeric metrics
- flag regressions when thresholds are crossed
- inspect replay metadata such as command, cwd, env snapshot reference, and artifact paths

### Non-goals for v1

- multi-user collaboration
- cloud sync
- distributed execution
- full test orchestration
- full deterministic replay engine
- image or video diff tooling beyond artifact linking
- watcher mode in the first implementation slice

## Architecture contract

RunScope is a Rust workspace with one shared core crate and two shells.

Expected layout:

```text
runscope/
├─ crates/
│  ├─ runscope-core/
│  └─ runscope-cli/
├─ apps/
│  └─ desktop/
└─ ui/
```

### Crate responsibilities

#### `runscope-core`
Owns all domain logic.

Includes:

- canonical manifest types
- adapter trait and adapter implementations
- artifact store layout and hashing
- SQLite migrations and repositories
- ingest orchestration
- record/query/compare/baseline services

Must not depend on:

- Tauri UI internals
- React frontend code
- shell-specific CLI logic

#### `runscope-cli`
Thin shell around core services.

Includes:

- `clap` argument parsing
- human-readable output
- optional JSON output
- process exit codes

Must not:

- implement domain logic directly
- execute raw SQL directly
- reimplement validation already owned by core

#### `apps/desktop/src-tauri`
Thin shell around core services for desktop UI.

Includes:

- Tauri command handlers
- application state wiring
- DTO transport between frontend and core

Must not:

- implement business logic directly
- own schema rules
- own ingest logic

#### `ui/`
React + TypeScript + Vite frontend.

Includes:

- runs list
- run detail view
- compare view
- baseline interactions
- trend charts

Must not:

- define source-of-truth run schema
- compute comparison rules independently of core
- silently reinterpret metric semantics

## Module boundaries inside `runscope-core`

Expected module tree:

```text
src/
├─ lib.rs
├─ error.rs
├─ domain/
├─ adapters/
├─ store/
├─ db/
├─ services/
└─ schema/
```

### `domain/`
Pure data model and DTO layer.

Contains:

- `run_manifest.rs`
- `scope.rs`
- `compare.rs`
- note and tag DTOs
- ID helpers if needed

Rules:

- no SQLite code
- no filesystem operations
- no Tauri or Clap types
- serde types should be stable and explicit

### `adapters/`
Parses producer-specific artifact folders into one canonical in-memory run model.

Rules:

- adapters parse only
- adapters never write DB rows directly
- adapters never copy files directly
- adapters return warnings for partial metadata instead of failing when possible
- adapters must preserve raw source files as managed artifacts through the ingest pipeline

### `store/`
Owns managed filesystem layout.

Rules:

- one predictable run root per run
- canonical `run.json` always written by RunScope
- all artifact paths stored as relative paths under the run root
- default ingest mode is copy
- no symlink or hardlink complexity in early PRs

### `db/`
Owns SQLite access.

Rules:

- `rusqlite` is the v1 choice
- timestamps are RFC3339 UTC stored as `TEXT`
- no shell-specific code
- repositories should remain simple and table-oriented

### `services/`
Application orchestration layer.

Owns:

- ingest flow
- manual record flow
- query flow
- compare logic
- baseline binding logic
- notes/tags operations

Rules:

- this is the only layer that coordinates adapters + artifact store + DB
- dedupe logic lives here
- shells call services, not repositories directly

### `schema/`
Owns JSON Schema generation for `run.json`.

Rules:

- generated schema must be committed
- CI should fail when committed schema drifts from Rust source types

## Canonical data contract

### Schema version

Current canonical manifest schema:

- `runscope.run.v1`

This is the source-of-truth interchange format for normalized runs.

### `run.json` requirements

Every normalized run must serialize to a canonical manifest with:

- `schema_version`
- `run_id`
- `project`
- `source`
- `identity`
- `runtime`
- `summary`
- `metrics`
- `artifacts`
- optional `git`
- optional `environment`
- optional `workload`
- optional `adapter_payload`

### Validation rules

Agents must preserve these rules:

1. `schema_version` must equal `runscope.run.v1`
2. `run_id` must be non-empty
3. `project.slug` must be non-empty
4. `source.adapter` must be non-empty
5. `source.ingested_at` must be RFC3339 UTC
6. metric records must include either `value_num` or `value_text`
7. artifact paths must be relative, never absolute
8. env snapshot references must be relative
9. adapters may add project-specific payloads only under `adapter_payload`
10. unknown source fields must not leak into the normalized top-level contract

### Metric semantics

Metrics are records, not an arbitrary JSON map.

Each metric should carry:

- `key`
- `group_name`
- numeric or text value
- optional unit
- direction
- primary metric flag

Preferred directions:

- `higher_is_better`
- `lower_is_better`
- `target`
- `none`

Do not invent metric comparison rules in the UI. Core owns metric diff semantics.

### Artifact semantics

Preferred artifact roles:

- `stdout_log`
- `stderr_log`
- `report_json`
- `report_html`
- `screenshot`
- `video`
- `input_manifest`
- `env_snapshot`
- `raw_source_manifest`
- `replay_script`

These are conventions, not a hard enum in v1, but new roles should be rare and justified.

## SQLite contract

RunScope stores searchable metadata in SQLite and artifact bytes on disk.

### Core tables

- `projects`
- `runs`
- `metrics`
- `artifacts`
- `run_warnings`
- `notes`
- `tags`
- `run_tags`
- `baseline_bindings`
- `regression_rules`

### Important modeling decisions

#### Runs
A run is the normalized summary record for one ingest or manual record event.

`runs.id` should be a stable local ID, preferably a ULID string.

#### Metrics
Metrics belong to a run and are stored row-wise.

Do not collapse metrics into opaque JSON if they need to be filtered, compared, or charted.

#### Baselines
A run is not globally “the baseline.”
A run is only a baseline for a defined comparison scope.

That is why baseline bindings are keyed by:

- project
- label
- canonical scope hash

#### Regression rules
Regression rules are explicit data.
They should not exist only in transient UI state.

#### Tags vs execution status
Keep these separate.

- machine outcome lives in `exec_status`
- human/operator labeling lives in tags like `investigate`, `important`, `baseline`, `regressed`

Do not overload one field with both concepts.

## Comparison and baseline rules

### Comparison scope

Comparison scope is a structured object that may include:

- branch
- suite
- scenario
- backend
- model
- precision
- dataset

Scope hashing must be deterministic.

Implementation rule:

- serialize canonical scope JSON with stable ordering
- compute `scope_hash = sha256(scope_json)`

### Compare behavior

Compare should produce:

- metadata diffs
- metric diffs
- artifact presence/path diffs
- regression flags when applicable

For numeric metrics:

- compute absolute delta when both sides are numeric
- compute percent delta when both sides are numeric and denominator is non-zero

### Regression logic

Keep comparators small and explicit in v1.

Supported comparator families:

- `pct_drop_gt`
- `pct_increase_gt`
- `abs_delta_gt`
- `abs_delta_lt`

Do not add a generic expression engine in early versions.

## Artifact store contract

Managed layout:

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

Rules:

- RunScope always writes the canonical `run.json`
- raw producer files go into `raw/`
- normalized/generated files go into `derived/`
- text logs go into `logs/`
- manual attachments go into `attachments/`
- DB stores relative artifact paths only

## Adapter contract

Adapters implement a strict parse-only interface.

Expected trait shape:

```rust
pub trait RunAdapter {
    fn name(&self) -> &'static str;
    fn detect(&self, artifact_dir: &Path) -> Result<bool, RunScopeError>;
    fn parse(&self, artifact_dir: &Path) -> Result<ParsedRun, RunScopeError>;
}
```

Where `ParsedRun` contains:

- canonical in-memory manifest
- files to copy into managed store
- non-fatal warnings

### Detection order

If adapter is not explicitly selected, detection order is:

1. `LocalAgent`
2. `VideoForge`
3. `faceapp`

If no adapter matches, fail clearly.
If multiple adapters match, fail clearly.
Do not silently guess.

### Adapter quality bar

A good adapter:

- preserves source fidelity
- fills normalized fields when known
- leaves unknowns as `None`
- emits warnings for partial metadata
- does not mutate source directories

## CLI contract

Top-level command:

```text
runscope [OPTIONS] <COMMAND>
```

### Supported commands for v1

- `runscope ingest <ARTIFACT_DIR>`
- `runscope record ...`
- `runscope list`
- `runscope show <RUN_ID>`
- `runscope compare <LEFT_RUN_ID> <RIGHT_RUN_ID>`
- `runscope baseline set <RUN_ID> ...`
- `runscope baseline list --project <SLUG>`
- `runscope note add <RUN_ID> --text <TEXT>`
- `runscope tag add <RUN_ID> <TAG>...`
- `runscope tag remove <RUN_ID> <TAG>...`

### CLI principles

- CLI is a shell, not the business logic layer
- support `--json` for structured scripting output
- preserve argument order for recorded command argv
- return clear exit codes for adapter failure, validation failure, storage failure, and not-found cases

## Tauri / UI contract

### Tauri command boundary

Tauri commands should only call core services and return DTOs.

Good:

- `list_runs(filter)`
- `get_run(run_id)`
- `compare_runs(left, right)`
- `set_active_baseline(req)`
- `list_baselines(project)`
- `add_note(req)`
- `add_tags(req)`
- `remove_tags(req)`

Bad:

- raw SQL from Tauri command handlers
- duplicating compare logic in frontend
- frontend computing authoritative regression results

### UI responsibilities

Initial UI should stay narrow:

- runs list
- run detail page
- compare view
- simple trend charts for explicit numeric metrics

Do not let the UI force new backend abstractions before the core loop is stable.

## Coding standards

### Rust standards

- prefer explicit types over overly clever generics
- prefer `struct` + clear fields over ad hoc maps
- keep domain types serde-friendly and stable
- use `thiserror` for typed core errors
- use `anyhow` only at shell boundaries when needed
- keep modules small and table-oriented
- avoid async in core unless there is a demonstrated need

### Database standards

- migrations are append-only
- do not rewrite past migrations after merge
- keep SQL readable and explicit
- index only for known query paths
- do not hide major schema changes behind helper magic

### Frontend standards

- treat core DTOs as source of truth
- keep tables and filtering straightforward
- do not introduce client-only schema drift
- prefer explainable charts over flashy charts

## Testing contract

Every feature PR should extend tests at the correct layer.

### Minimum expected test categories

#### Unit tests
For:

- manifest validation
- scope hash stability
- metric diff logic
- artifact store path generation

#### Integration tests
For:

- adapter ingest on fixture directories
- manual record flow
- duplicate ingest idempotency
- baseline replacement in same scope
- compare report regression flag generation
- migration bootstrap on empty DB

#### Golden tests
Use fixture artifact directories and assert the exact canonical `run.json` shape where useful.

## Determinism and reproducibility rules

RunScope is not a full deterministic replay engine in v1, but it must preserve enough provenance to support manual reproduction.

Store when available:

- argv
- display command
- cwd
- env snapshot reference
- source artifact paths
- source hash
- commit SHA
- branch
- machine / backend / model / precision context

Do not claim more replay guarantees than the system can actually provide.

## Security and safety rules

### Environment snapshots
Do not assume raw environment variables are safe to store.

Default behavior should prefer:

- redacted env snapshot artifacts
- allowlist or denylist filtering
- hashes or references when full persistence is unsafe

### Path handling

- never store absolute artifact paths inside canonical artifact records
- reject path traversal in copied artifact destinations
- keep all managed files under the configured data directory

## Recommended PR order

### PR1
- workspace scaffold
- core domain types
- `run.json` schema generation
- SQLite migration bootstrap
- artifact store layout
- LocalAgent adapter
- `runscope ingest`

### PR2
- VideoForge adapter
- faceapp adapter
- manual record flow

### PR3
- list/show queries
- notes and tags
- detail DTOs

### PR4
- compare service
- baseline bindings
- regression rule evaluation
- baseline CLI

### PR5
- Tauri wiring
- runs list UI
- run detail UI
- compare UI

## Anti-patterns to avoid

Do not:

- turn this into a generic orchestration platform in v1
- mix UI state with persistent baseline/rule state
- let adapters write directly to SQLite
- keep critical queryable fields only inside opaque JSON blobs
- overload `status` with both execution result and human workflow meaning
- add watcher mode before manual ingest is stable
- promise replay determinism beyond stored metadata
- add cloud abstractions with no local use case pressure

## Good commit / PR shape

A good PR in this repo:

- changes one layer or one feature slice cleanly
- updates schema/migration/tests together when needed
- includes acceptance criteria in the PR description
- ships a small vertical slice when possible
- leaves the workspace easier to understand than before

## Suggested acceptance criteria style

Use concrete acceptance criteria such as:

- ingesting the same fixture twice returns the existing run ID and does not duplicate rows
- baseline replacement deactivates the previous active binding for the same scope
- compare reports show missing metrics explicitly rather than hiding them
- `run.json` generation matches committed schema and fixture goldens

## Final guidance for coding agents

When modifying this repo:

1. preserve the canonical `runscope.run.v1` contract
2. keep adapters parse-only
3. keep services as the orchestration boundary
4. store searchable metadata in SQLite and bytes on disk
5. keep baseline and regression semantics explicit
6. prefer boring correctness over framework cleverness
7. do not broaden scope unless the product loop clearly demands it

When in doubt, strengthen the core ingest -> normalize -> persist -> inspect -> compare loop instead of adding more surface area.
