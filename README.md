# RunScope

**Local-first run history, benchmark, and reproducibility dashboard for engineering projects.**

See `docs/runscope-v1-pr-ready-spec.md` for the full architecture, schema, SQLite DDL, and CLI contract.

RunScope ingests artifact folders and manual test runs, normalizes them into a searchable `run.json` manifest, stores metadata in SQLite, preserves raw artifacts on disk, and makes it easy to compare runs, track regressions, manage baselines, and inspect the exact command and environment behind every result.

## Status

Early project scaffold and spec phase.

The initial target is a local-first MVP focused on:

- artifact ingestion
- normalized run manifests
- SQLite-backed metadata
- artifact browsing
- run comparison
- scoped baselines and regression flags

## Why RunScope

Engineering projects generate runs constantly: benchmarks, smoke tests, evals, manual experiments, and replay artifacts. The raw outputs usually end up scattered across folders, logs, screenshots, JSON reports, and ad hoc notes.

RunScope gives those runs a consistent home.

Instead of hunting through directories and trying to remember which command, model, backend, branch, or machine produced a result, RunScope turns every run into a structured record that you can browse, compare, filter, and revisit later.

## Core use cases

- Ingest run artifacts from projects like `LocalAgent`, `VideoForge`, and `faceapp`
- Record manual test sessions with commit SHA, machine, GPU, command, flags, model, dataset or input, and notes
- Compare two runs or compare a run against a scoped baseline
- Track trend lines over time by project, backend, model, precision, or branch
- Flag regressions when metrics cross explicit thresholds
- Store enough provenance to reproduce a run later

## MVP

- Define one normalized `run.json` manifest schema
- Build a CLI:
  - `runscope ingest <artifact_dir>`
  - `runscope record ...`
- Support three adapters:
  - `LocalAgent` eval and replay artifacts
  - `VideoForge` smoke and perf reports
  - `faceapp` backend benchmark reports
- Store metadata in SQLite
- Preserve logs, JSON, screenshots, videos, and reports in a managed artifact store on disk
- Ship a local desktop UI with:
  - runs list
  - run detail page
  - compare view
  - simple trend charts
- Let users mark and organize runs with tags like:
  - `baseline`
  - `pass`
  - `fail`
  - `investigate`
  - `regressed`
  - `improved`
- Show replay metadata:
  - original command
  - cwd
  - env snapshot reference
  - artifact paths

## Non-goals for v1

- multi-user collaboration
- cloud sync
- distributed execution
- full test orchestration
- video or image diffing beyond artifact linking
- guaranteed deterministic replay execution

## How it works

### 1. Producers
Each project emits an artifact folder plus raw logs, reports, and outputs.

### 2. Ingest layer
Per-project adapters parse those raw outputs and convert them into one normalized `run.json` manifest.

### 3. Storage layer
RunScope stores structured metadata in SQLite and keeps raw artifacts on disk in a managed artifact store.

### 4. Query and comparison layer
Runs can be filtered, compared, tagged, attached to scoped baselines, and checked against threshold-based regression rules.

### 5. UI layer
A local dashboard makes it easy to browse runs, inspect details, compare results, and review provenance.

## Normalized run model

Every ingested or recorded run is represented by a canonical `run.json` manifest.

The normalized model captures:

- project identity
- adapter and source metadata
- suite, scenario, and label
- git context
- runtime status and timing
- environment details such as machine, OS, CPU, GPU, backend, model, and precision
- workload metadata such as dataset, input count, command, cwd, and env snapshot reference
- metrics
- artifact references
- adapter-specific payloads when needed

This keeps cross-project queries stable while preserving source-specific details.

## Example use cases

### Compare performance between two backend runs
See whether a TensorRT run regressed against the active FP16 baseline on the same model and dataset.

### Revisit a manual debugging session
Open the exact command, cwd, env snapshot reference, notes, and logs tied to a failure that happened two weeks ago.

### Track long-term trends
Filter by project, branch, backend, or model to see whether throughput or latency is moving in the right direction over time.

## Planned architecture

### Stack

- **Tauri v2** for the local desktop shell
- **Rust** for core domain logic, adapters, CLI, storage, and compare services
- **React + TypeScript + Vite** for the frontend
- **SQLite** for metadata
- **serde** for schema and manifest parsing
- **rusqlite** for DB access
- **Recharts** for trend and compare graphs
- **TanStack Table** for sortable and filterable run tables

### Core layers

- **Domain**: canonical run manifest, scope, compare DTOs
- **Adapters**: project-specific parsers for producer artifacts
- **Artifact store**: on-disk managed storage for copied run files
- **DB**: SQLite schema, migrations, repositories
- **Services**: ingest, record, query, compare, baseline, notes, tags
- **Shells**: CLI and Tauri desktop app

## Repository goals

RunScope is meant to reinforce a strong local-first Rust tooling story:

- deterministic ingest pipeline
- explicit normalized schema
- clear provenance and auditability
- portable local storage
- real engineering utility across multiple projects

## Planned CLI surface

```bash
runscope ingest <artifact_dir> [--adapter auto|localagent|videoforge|faceapp]
runscope record --project <slug> --status <pass|fail|error|unknown> [options]
runscope list [filters]
runscope show <run_id>
runscope compare <left_run_id> <right_run_id>
runscope baseline set <run_id> [scope options]
runscope baseline list --project <slug>
runscope note add <run_id> --text "..."
runscope tag add <run_id> <tag>...
runscope tag remove <run_id> <tag>...
```

## Planned artifact layout

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

## Roadmap

### Phase 1
- canonical `run.json` schema
- SQLite schema and migrations
- artifact store layout
- `runscope ingest`
- LocalAgent adapter

### Phase 2
- VideoForge adapter
- faceapp adapter
- `runscope record`
- runs list and run detail query path

### Phase 3
- compare service
- scoped baselines
- regression rules
- notes and tags

### Phase 4
- Tauri UI
- trend charts
- docs, fixtures, and smoke tests

## Design principles

- **Local first**: runs live on your machine, with SQLite metadata and artifact files on disk
- **Schema first**: every run normalizes into a stable manifest contract
- **Provenance matters**: commands, cwd, env references, logs, and artifacts stay attached to the run
- **Explicit comparisons**: baselines and regression rules are scoped and explainable
- **Portable and inspectable**: raw artifacts remain accessible outside the app

## Who this is for

RunScope is for engineers building and testing systems that emit artifacts over time, especially when they need to answer questions like:

- What changed between these two runs?
- Which run is the current baseline for this model and backend?
- When did performance regress?
- What exact command and environment produced this output?
- Can I inspect the raw logs and reports behind this result?

## Future directions

After the MVP is solid, future work may include:

- watcher mode for auto-ingesting artifact folders
- import and export of normalized manifests
- richer trend and baseline dashboards
- replay script export
- more adapters for additional engineering workflows

## License

TBD
