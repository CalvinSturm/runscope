import { startTransition, useDeferredValue, useEffect, useState, type ReactNode } from "react";
import { getRun, listRuns } from "./lib/api";
import { formatDateTime, formatDuration, formatMetric } from "./lib/format";
import type { ExecStatus, RunDetail, RunListFilter, RunListItem } from "./lib/types";

type FilterState = {
  queryText: string;
  project: string;
  execStatus: "" | ExecStatus;
  backend: string;
  model: string;
  precision: string;
};

const initialFilters: FilterState = {
  queryText: "",
  project: "",
  execStatus: "",
  backend: "",
  model: "",
  precision: "",
};

export default function App() {
  const [filters, setFilters] = useState<FilterState>(initialFilters);
  const deferredQueryText = useDeferredValue(filters.queryText);
  const [items, setItems] = useState<RunListItem[]>([]);
  const [total, setTotal] = useState(0);
  const [selectedRunId, setSelectedRunId] = useState<string | null>(null);
  const [detail, setDetail] = useState<RunDetail | null>(null);
  const [loadingList, setLoadingList] = useState(true);
  const [loadingDetail, setLoadingDetail] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    const run = async () => {
      setLoadingList(true);
      setError(null);
      try {
        const filter: RunListFilter = {
          query_text: deferredQueryText || undefined,
          project: filters.project || undefined,
          exec_status: filters.execStatus || undefined,
          backend: filters.backend || undefined,
          model: filters.model || undefined,
          precision: filters.precision || undefined,
          limit: 100,
          offset: 0,
        };
        const page = await listRuns(filter);
        if (cancelled) {
          return;
        }
        setItems(page.items);
        setTotal(page.total);
        startTransition(() => {
          setSelectedRunId((current) => {
            if (current && page.items.some((item) => item.run_id === current)) {
              return current;
            }
            return page.items[0]?.run_id ?? null;
          });
        });
      } catch (caught) {
        if (!cancelled) {
          setError(String(caught));
          setItems([]);
          setTotal(0);
          setSelectedRunId(null);
        }
      } finally {
        if (!cancelled) {
          setLoadingList(false);
        }
      }
    };
    void run();
    return () => {
      cancelled = true;
    };
  }, [
    deferredQueryText,
    filters.backend,
    filters.execStatus,
    filters.model,
    filters.precision,
    filters.project,
  ]);

  useEffect(() => {
    if (!selectedRunId) {
      setDetail(null);
      return;
    }

    let cancelled = false;
    const run = async () => {
      setLoadingDetail(true);
      try {
        const nextDetail = await getRun(selectedRunId);
        if (!cancelled) {
          setDetail(nextDetail);
        }
      } catch (caught) {
        if (!cancelled) {
          setError(String(caught));
        }
      } finally {
        if (!cancelled) {
          setLoadingDetail(false);
        }
      }
    };
    void run();
    return () => {
      cancelled = true;
    };
  }, [selectedRunId]);

  const selectedSummary = items.find((item) => item.run_id === selectedRunId) ?? null;
  const primaryMetrics = detail?.manifest.metrics.filter((metric) => metric.is_primary) ?? [];

  return (
    <main className="app-shell">
      <section className="hero">
        <div>
          <p className="eyebrow">RunScope Dashboard</p>
          <h1>Local run history for engineering work you actually need to inspect.</h1>
        </div>
        <div className="hero-stats">
          <div>
            <span>Total visible runs</span>
            <strong>{total}</strong>
          </div>
          <div>
            <span>Selected adapter</span>
            <strong>{selectedSummary?.adapter ?? "n/a"}</strong>
          </div>
        </div>
      </section>

      <section className="filters-panel">
        <FilterField
          label="Search"
          value={filters.queryText}
          onChange={(value) => setFilters((current) => ({ ...current, queryText: value }))}
          placeholder="run id, label, scenario, backend, model"
        />
        <FilterField
          label="Project"
          value={filters.project}
          onChange={(value) => setFilters((current) => ({ ...current, project: value }))}
          placeholder="videoforge or localagent"
        />
        <label>
          Status
          <select
            value={filters.execStatus}
            onChange={(event) =>
              setFilters((current) => ({
                ...current,
                execStatus: event.target.value as FilterState["execStatus"],
              }))
            }
          >
            <option value="">all</option>
            <option value="pass">pass</option>
            <option value="fail">fail</option>
            <option value="error">error</option>
            <option value="unknown">unknown</option>
          </select>
        </label>
        <FilterField
          label="Backend"
          value={filters.backend}
          onChange={(value) => setFilters((current) => ({ ...current, backend: value }))}
          placeholder="cuda, tensorrt, cpu"
        />
        <FilterField
          label="Model"
          value={filters.model}
          onChange={(value) => setFilters((current) => ({ ...current, model: value }))}
          placeholder="model id"
        />
        <FilterField
          label="Precision"
          value={filters.precision}
          onChange={(value) => setFilters((current) => ({ ...current, precision: value }))}
          placeholder="fp16, int8, bf16"
        />
      </section>

      {error ? <div className="banner banner-error">{error}</div> : null}

      <section className="workspace">
        <div className="runs-panel">
          <div className="section-heading">
            <div>
              <p className="section-label">Runs</p>
              <h2>Latest local history</h2>
            </div>
            {loadingList ? <span className="quiet">Refreshing...</span> : null}
          </div>
          <div className="runs-list">
            {items.length === 0 && !loadingList ? (
              <div className="empty-state">
                <h3>No runs matched the current filters.</h3>
                <p>Ingest a LocalAgent or VideoForge artifact directory to populate the dashboard.</p>
              </div>
            ) : null}
            {items.map((item) => (
              <button
                key={item.run_id}
                className={`run-card ${selectedRunId === item.run_id ? "selected" : ""}`}
                onClick={() => setSelectedRunId(item.run_id)}
                type="button"
              >
                <div className="run-card-topline">
                  <span className={`status-pill ${item.exec_status}`}>{item.exec_status}</span>
                  <span className="adapter-pill">{item.adapter}</span>
                  <span className="timestamp">{formatDateTime(item.started_at)}</span>
                </div>
                <div className="run-card-title">
                  <strong>{item.project_slug}</strong>
                  <span>{item.label ?? item.scenario ?? item.run_id}</span>
                </div>
                <div className="run-card-identity">
                  <span>{item.suite ?? "no suite"}</span>
                  <span>{item.scenario ?? "no scenario"}</span>
                </div>
                <div className="run-card-meta">
                  <span>{item.backend ?? "backend n/a"}</span>
                  <span>{item.model ?? "model n/a"}</span>
                  <span>{item.precision ?? "precision n/a"}</span>
                  <span>{formatDuration(item.duration_ms)}</span>
                </div>
                <div className="metric-row">
                  {item.primary_metrics.length > 0 ? (
                    item.primary_metrics.map((metric) => (
                      <div key={`${metric.group_name}:${metric.key}`} className="metric-chip">
                        <span>{metric.key}</span>
                        <strong>{formatMetric(metric)}</strong>
                      </div>
                    ))
                  ) : (
                    <span className="quiet">No primary metrics</span>
                  )}
                </div>
                {item.tags.length > 0 ? (
                  <div className="tag-row">
                    {item.tags.map((tag) => (
                      <span key={tag} className="tag-pill">
                        {tag}
                      </span>
                    ))}
                  </div>
                ) : null}
              </button>
            ))}
          </div>
        </div>

        <div className="detail-panel">
          <div className="section-heading">
            <div>
              <p className="section-label">Detail</p>
              <h2>Run inspection</h2>
            </div>
            {loadingDetail ? <span className="quiet">Loading...</span> : null}
          </div>

          {!detail ? (
            <div className="empty-state detail-empty">
              <h3>Select a run to inspect its manifest, metrics, and artifacts.</h3>
              <p>The detail panel uses the stored canonical run.json plus notes, warnings, and tags.</p>
            </div>
          ) : (
            <div className="detail-scroll">
              <section className="detail-hero">
                <div>
                  <div className="detail-title-row">
                    <span className={`status-pill ${detail.manifest.runtime.exec_status}`}>
                      {detail.manifest.runtime.exec_status}
                    </span>
                    <span className="adapter-pill">{detail.manifest.source.adapter}</span>
                  </div>
                  <h3>{detail.manifest.identity.label ?? detail.manifest.run_id}</h3>
                  <p>
                    {detail.manifest.project.display_name} · {detail.manifest.identity.suite ?? "no suite"} ·{" "}
                    {detail.manifest.identity.scenario ?? "no scenario"}
                  </p>
                </div>
                <div className="detail-root">
                  <span>Run root</span>
                  <code>{detail.run_root}</code>
                </div>
              </section>

              <section className="detail-grid">
                <InfoCard title="Identity">
                  <InfoRow label="Project" value={detail.manifest.project.slug} />
                  <InfoRow label="Suite" value={detail.manifest.identity.suite} />
                  <InfoRow label="Scenario" value={detail.manifest.identity.scenario} />
                  <InfoRow label="Label" value={detail.manifest.identity.label} />
                  <InfoRow label="Run ID" value={detail.manifest.run_id} />
                </InfoCard>
                <InfoCard title="Execution">
                  <InfoRow label="Status" value={detail.manifest.runtime.exec_status} />
                  <InfoRow label="Started" value={formatDateTime(detail.manifest.runtime.started_at)} />
                  <InfoRow label="Finished" value={formatDateTime(detail.manifest.runtime.finished_at)} />
                  <InfoRow label="Duration" value={formatDuration(detail.manifest.runtime.duration_ms)} />
                  <InfoRow label="Warnings" value={detail.manifest.summary.warning_count.toString()} />
                </InfoCard>
                <InfoCard title="Environment">
                  <InfoRow label="Backend" value={detail.manifest.environment?.backend} />
                  <InfoRow label="Model" value={detail.manifest.environment?.model} />
                  <InfoRow label="Precision" value={detail.manifest.environment?.precision} />
                  <InfoRow label="Machine" value={detail.manifest.environment?.machine_name} />
                  <InfoRow label="GPU" value={detail.manifest.environment?.gpu} />
                </InfoCard>
                <InfoCard title="Workload">
                  <InfoRow label="Dataset" value={detail.manifest.workload?.dataset} />
                  <InfoRow
                    label="Input count"
                    value={
                      detail.manifest.workload?.input_count != null
                        ? String(detail.manifest.workload.input_count)
                        : null
                    }
                  />
                  <InfoRow label="CWD" value={detail.manifest.workload?.cwd} />
                  <InfoRow label="Env snapshot" value={detail.manifest.workload?.env_snapshot_ref} />
                </InfoCard>
              </section>

              <Panel title="Key Metrics" subtitle="Primary metrics shown in the list for comparison readiness.">
                <div className="metric-grid">
                  {primaryMetrics.length > 0 ? (
                    primaryMetrics.map((metric) => (
                      <div className="metric-tile" key={`${metric.group_name}:${metric.key}`}>
                        <span>{metric.key}</span>
                        <strong>{formatMetric(metric)}</strong>
                        <small>{metric.direction.replace(/_/g, " ")}</small>
                      </div>
                    ))
                  ) : (
                    <div className="quiet">No primary metrics were marked for this run.</div>
                  )}
                </div>
              </Panel>

              <Panel title="All Metrics" subtitle="Normalized metric records from the canonical manifest.">
                <table className="data-table">
                  <thead>
                    <tr>
                      <th>Key</th>
                      <th>Group</th>
                      <th>Value</th>
                      <th>Direction</th>
                      <th>Primary</th>
                    </tr>
                  </thead>
                  <tbody>
                    {detail.manifest.metrics.map((metric) => (
                      <tr key={`${metric.group_name}:${metric.key}`}>
                        <td>{metric.key}</td>
                        <td>{metric.group_name || "default"}</td>
                        <td>{formatMetric(metric)}</td>
                        <td>{metric.direction.replace(/_/g, " ")}</td>
                        <td>{metric.is_primary ? "yes" : "no"}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </Panel>

              <Panel title="Artifacts" subtitle="Managed relative paths preserved under the local run root.">
                <table className="data-table">
                  <thead>
                    <tr>
                      <th>Role</th>
                      <th>Relative path</th>
                      <th>Media type</th>
                      <th>Size</th>
                    </tr>
                  </thead>
                  <tbody>
                    {detail.manifest.artifacts.map((artifact) => (
                      <tr key={`${artifact.role}:${artifact.rel_path}`}>
                        <td>{artifact.role}</td>
                        <td>
                          <code>{artifact.rel_path}</code>
                        </td>
                        <td>{artifact.media_type}</td>
                        <td>{artifact.size_bytes != null ? `${artifact.size_bytes} B` : "n/a"}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </Panel>

              <div className="detail-grid secondary">
                <Panel title="Tags" subtitle="Operator labels kept separate from execution status.">
                  <div className="tag-row">
                    {detail.tags.length > 0 ? (
                      detail.tags.map((tag) => (
                        <span key={tag} className="tag-pill">
                          {tag}
                        </span>
                      ))
                    ) : (
                      <span className="quiet">No tags</span>
                    )}
                  </div>
                </Panel>
                <Panel title="Notes" subtitle="Persistent operator notes for this run.">
                  {detail.notes.length > 0 ? (
                    detail.notes.map((note) => (
                      <article key={note.id} className="stacked-note">
                        <strong>{formatDateTime(note.created_at)}</strong>
                        <p>{note.body}</p>
                      </article>
                    ))
                  ) : (
                    <span className="quiet">No notes</span>
                  )}
                </Panel>
              </div>

              {detail.warnings.length > 0 ? (
                <Panel title="Warnings" subtitle="Adapter warnings captured during ingest.">
                  {detail.warnings.map((warning) => (
                    <article key={`${warning.created_at}:${warning.code}`} className="warning-item">
                      <strong>{warning.code}</strong>
                      <p>{warning.message}</p>
                    </article>
                  ))}
                </Panel>
              ) : null}
            </div>
          )}
        </div>
      </section>
    </main>
  );
}

function FilterField(props: {
  label: string;
  value: string;
  onChange: (value: string) => void;
  placeholder: string;
}) {
  return (
    <label>
      {props.label}
      <input
        value={props.value}
        onChange={(event) => props.onChange(event.target.value)}
        placeholder={props.placeholder}
      />
    </label>
  );
}

function Panel(props: { title: string; subtitle: string; children: ReactNode }) {
  return (
    <section className="panel">
      <div className="panel-heading">
        <h3>{props.title}</h3>
        <p>{props.subtitle}</p>
      </div>
      {props.children}
    </section>
  );
}

function InfoCard(props: { title: string; children: ReactNode }) {
  return (
    <section className="info-card">
      <h3>{props.title}</h3>
      <div className="info-rows">{props.children}</div>
    </section>
  );
}

function InfoRow(props: { label: string; value: string | null | undefined }) {
  return (
    <div className="info-row">
      <span>{props.label}</span>
      <strong>{props.value && props.value.length > 0 ? props.value : "n/a"}</strong>
    </div>
  );
}
