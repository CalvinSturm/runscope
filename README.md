# RunScope

> Local-first run history, comparison, and provenance for engineering workflows.

RunScope turns evals, benchmarks, smoke tests, replay artifacts, and manual debugging sessions into structured runs you can inspect, compare, and revisit later.

It ingests artifact folders and manual runs, normalizes them into a canonical `run.json` manifest, stores structured metadata in SQLite, and preserves raw logs, reports, screenshots, videos, and outputs on disk.

## Why it exists

Engineering runs usually end up scattered across folders, logs, screenshots, JSON reports, and ad hoc notes.

RunScope makes those runs searchable, comparable, and easier to inspect later.

It is built to answer questions like:

- What changed between these two runs?
- Which run is the current baseline for this model, backend, or branch?
- When did latency, throughput, or quality regress?
- What exact command, machine, backend, or environment produced this output?
- Can I inspect the raw artifacts behind this result?

## What RunScope does

- ingests artifact folders from project-specific producers
- records manual test sessions with provenance
- normalizes runs into one canonical `run.json` model
- stores structured metadata in SQLite
- preserves raw artifacts on disk
- compares runs and scoped baselines
- flags regressions when metrics cross thresholds
- makes historical runs searchable and inspectable

## Core use cases

- ingest eval, replay, smoke, and benchmark artifacts from engineering projects
- compare two runs directly or against a scoped baseline
- track latency, throughput, or quality trends over time
- revisit a failure with the original command, cwd, env reference, logs, and attachments
- preserve enough provenance to make results easier to reproduce

## Architecture at a glance

RunScope uses per-project adapters to normalize raw outputs into a canonical `run.json` manifest. Structured metadata lives in SQLite, raw artifacts stay on disk, and the local desktop UI makes runs easy to browse, filter, compare, and inspect.

## Initial adapters

- `LocalAgent`
- `VideoForge`
- `faceapp`

## Status

Active MVP currently focused on:

- artifact ingestion
- normalized manifests
- SQLite-backed metadata
- raw artifact preservation
- run comparison
- scoped baselines
- regression flags
- local desktop inspection UI

## Why local-first

Engineering artifacts are often large, messy, and tied to local development environments. RunScope keeps metadata local, preserves raw artifacts on disk, and avoids requiring cloud infrastructure to get value from the tool.

## Documentation

For the full architecture, schema, SQLite DDL, and CLI contract, see:

- `docs/runscope-v1-pr-ready-spec.md`
