import { startTransition, useDeferredValue, useEffect, useState, type ReactNode } from "react";
import {
  compareRuns,
  createRegressionRule,
  getRun,
  listBaselines,
  listRegressionRules,
  listRuns,
  openPath,
  revealPath,
  setActiveBaseline,
} from "./lib/api";
import {
  EMPTY_TOKEN,
  compactMetricLabel,
  formatDateTime,
  formatDuration,
  formatMetric,
  formatNumericDelta,
  formatOptionalText,
  formatPercentDelta,
  formatRelativeAge,
} from "./lib/format";
import type {
  BaselineBinding,
  CreateRegressionRuleRequest,
  CompareReport,
  ExecStatus,
  MetricRecord,
  RegressionComparator,
  RegressionRule,
  RunDetail,
  RunListFilter,
  RunListItem,
} from "./lib/types";

type FilterState = {
  queryText: string;
  project: string;
  execStatus: "" | ExecStatus;
  backend: string;
  model: string;
  precision: string;
};

type MetricSortMode = "severity" | "abs_delta" | "pct_delta" | "primary";
type CompactSortMode = "latest" | "warnings" | "duration" | "primary_metric";
type CompareSemantic = "improved" | "regressed" | "changed" | "unresolved" | "stable";
type CompareSemanticFilter = "all" | "regressed" | "improved" | "changed" | "unresolved";

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
  const [compareTargetId, setCompareTargetId] = useState<string | null>(null);
  const [detail, setDetail] = useState<RunDetail | null>(null);
  const [candidateDetail, setCandidateDetail] = useState<RunDetail | null>(null);
  const [compareReport, setCompareReport] = useState<CompareReport | null>(null);
  const [projectBaselines, setProjectBaselines] = useState<BaselineBinding[]>([]);
  const [regressionRules, setRegressionRules] = useState<RegressionRule[]>([]);
  const [loadingList, setLoadingList] = useState(true);
  const [loadingDetail, setLoadingDetail] = useState(false);
  const [loadingCompare, setLoadingCompare] = useState(false);
  const [savingBaseline, setSavingBaseline] = useState(false);
  const [savingRule, setSavingRule] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [ruleDraft, setRuleDraft] = useState<{
    metricKey: string;
    comparator: RegressionComparator;
    thresholdValue: string;
  }>({
    metricKey: "",
    comparator: "pct_drop_gt",
    thresholdValue: "5",
  });
  const [compactList, setCompactList] = useState(false);
  const [compactSortMode, setCompactSortMode] = useState<CompactSortMode>("latest");
  const [compactMetricKey, setCompactMetricKey] = useState("");
  const [showAllPrimaryMetrics, setShowAllPrimaryMetrics] = useState(false);
  const [pathNotice, setPathNotice] = useState<string | null>(null);
  const [metricSortMode, setMetricSortMode] = useState<MetricSortMode>("severity");
  const [primaryOnlyMetricDiffs, setPrimaryOnlyMetricDiffs] = useState(false);
  const [changedOnlyMetricDiffs, setChangedOnlyMetricDiffs] = useState(true);
  const [compareSemanticMap, setCompareSemanticMap] = useState<Record<string, CompareSemantic>>({});
  const [loadingCompareSemantics, setLoadingCompareSemantics] = useState(false);
  const [compareSemanticFilter, setCompareSemanticFilter] =
    useState<CompareSemanticFilter>("all");
  const [compareSemanticSort, setCompareSemanticSort] = useState(false);

  useEffect(() => {
    let cancelled = false;
    const run = async () => {
      setLoadingList(true);
      setError(null);
      try {
        const normalizedQuery = deferredQueryText.trim().toLowerCase();
        const specialQuery =
          normalizedQuery === "has:metrics" || normalizedQuery === "has:baseline"
            ? normalizedQuery
            : null;
        const filter: RunListFilter = {
          query_text: specialQuery ? undefined : deferredQueryText || undefined,
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
        const filteredItems = page.items.filter((item) => {
          if (specialQuery === "has:metrics") {
            return item.primary_metrics.length > 0;
          }
          if (specialQuery === "has:baseline") {
            return activeBaselineRunIds.has(item.run_id);
          }
          return true;
        });
        setItems(filteredItems);
        setTotal(filteredItems.length);
        startTransition(() => {
          setSelectedRunId((current) => {
            if (current && filteredItems.some((item) => item.run_id === current)) {
              return current;
            }
            return filteredItems[0]?.run_id ?? null;
          });
          setCompareTargetId((current) => {
            if (
              current &&
              current !== selectedRunId &&
              filteredItems.some((item) => item.run_id === current)
            ) {
              return current;
            }
            return null;
          });
        });
      } catch (caught) {
        if (!cancelled) {
          setError(String(caught));
          setItems([]);
          setTotal(0);
          setSelectedRunId(null);
          setCompareTargetId(null);
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
    projectBaselines,
    selectedRunId,
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

  useEffect(() => {
    if (!compareTargetId || compareTargetId === selectedRunId) {
      setCandidateDetail(null);
      return;
    }

    let cancelled = false;
    const run = async () => {
      try {
        const nextDetail = await getRun(compareTargetId);
        if (!cancelled) {
          setCandidateDetail(nextDetail);
        }
      } catch (caught) {
        if (!cancelled) {
          setError(String(caught));
          setCandidateDetail(null);
        }
      }
    };
    void run();
    return () => {
      cancelled = true;
    };
  }, [compareTargetId, selectedRunId]);

  useEffect(() => {
    if (!selectedRunId || !compareTargetId || selectedRunId === compareTargetId) {
      setCompareReport(null);
      return;
    }

    let cancelled = false;
    const run = async () => {
      setLoadingCompare(true);
      try {
        const nextReport = await compareRuns(selectedRunId, compareTargetId);
        if (!cancelled) {
          setCompareReport(nextReport);
        }
      } catch (caught) {
        if (!cancelled) {
          setError(String(caught));
          setCompareReport(null);
        }
      } finally {
        if (!cancelled) {
          setLoadingCompare(false);
        }
      }
    };
    void run();
    return () => {
      cancelled = true;
    };
  }, [compareTargetId, selectedRunId]);

  useEffect(() => {
    if (!selectedRunId || items.length === 0) {
      setCompareSemanticMap({});
      setLoadingCompareSemantics(false);
      return;
    }

    const candidateItems = items.filter((item) => item.run_id !== selectedRunId);
    if (candidateItems.length === 0) {
      setCompareSemanticMap({});
      setLoadingCompareSemantics(false);
      return;
    }

    let cancelled = false;
    const run = async () => {
      setLoadingCompareSemantics(true);
      try {
        const reports = await Promise.all(
          candidateItems.map(async (item) => ({
            runId: item.run_id,
            report: await compareRuns(selectedRunId, item.run_id),
          })),
        );
        if (cancelled) {
          return;
        }
        const nextMap = reports.reduce<Record<string, CompareSemantic>>((accumulator, entry) => {
          const candidateItem = candidateItems.find((item) => item.run_id === entry.runId);
          const baseItem = items.find((item) => item.run_id === selectedRunId);
          if (!candidateItem || !baseItem) {
            return accumulator;
          }
          accumulator[entry.runId] = deriveCompareSemanticFromReport(
            entry.report,
            baseItem,
            candidateItem,
          );
          return accumulator;
        }, {});
        setCompareSemanticMap(nextMap);
      } catch (caught) {
        if (!cancelled) {
          setError(String(caught));
          setCompareSemanticMap({});
        }
      } finally {
        if (!cancelled) {
          setLoadingCompareSemantics(false);
        }
      }
    };
    void run();
    return () => {
      cancelled = true;
    };
  }, [items, selectedRunId]);

  useEffect(() => {
    if (!detail) {
      setProjectBaselines([]);
      setRegressionRules([]);
      return;
    }

    let cancelled = false;
    const run = async () => {
      try {
        const baselines = await listBaselines(detail.manifest.project.slug);
        if (!cancelled) {
          setProjectBaselines(baselines);
        }
        const rules = await listRegressionRules(detail.manifest.project.slug);
        if (!cancelled) {
          setRegressionRules(rules);
        }
      } catch (caught) {
        if (!cancelled) {
          setError(String(caught));
        }
      }
    };
    void run();
    return () => {
      cancelled = true;
    };
  }, [detail?.manifest.project.slug]);

  useEffect(() => {
    if (!detail) {
      return;
    }
    setShowAllPrimaryMetrics(false);
    const defaultMetric =
      detail.manifest.metrics.find((metric) => metric.is_primary)?.key ??
      detail.manifest.metrics[0]?.key ??
      "";
    setRuleDraft((current) => ({
      ...current,
      metricKey: current.metricKey || defaultMetric,
    }));
  }, [detail]);

  const selectedSummary = items.find((item) => item.run_id === selectedRunId) ?? null;
  const compareSummary = items.find((item) => item.run_id === compareTargetId) ?? null;
  const primaryMetrics = detail?.manifest.metrics.filter((metric) => metric.is_primary) ?? [];
  const matchingBaselineForCompare =
    compareSummary && detail
      ? detail.active_baselines.find((baseline) => baseline.run_id === compareSummary.run_id)
      : null;
  const activeBaselineRunIds = new Set(projectBaselines.map((baseline) => baseline.run_id));

  const latestStartedAt = items.reduce<string | null>((latest, item) => {
    if (!item.started_at) {
      return latest;
    }
    if (!latest) {
      return item.started_at;
    }
    return new Date(item.started_at).getTime() > new Date(latest).getTime() ? item.started_at : latest;
  }, null);

  const passCount = items.filter((item) => item.exec_status === "pass").length;
  const failCount = items.filter(
    (item) => item.exec_status === "fail" || item.exec_status === "error",
  ).length;
  const unknownCount = items.filter((item) => item.exec_status === "unknown").length;
  const adapterCounts = items.reduce<Record<string, number>>((accumulator, item) => {
    accumulator[item.adapter] = (accumulator[item.adapter] ?? 0) + 1;
    return accumulator;
  }, {});
  const adapterSummary = Object.entries(adapterCounts)
    .sort((left, right) => right[1] - left[1])
    .map(([adapter, count]) => `${adapter} ${count}`)
    .join(" · ");
  const projectBaselineCount = new Set(projectBaselines.map((baseline) => baseline.run_id)).size;
  const baselineCoverageText = detail
    ? `${detail.active_baselines.length} matching · ${projectBaselineCount} active in project`
    : EMPTY_TOKEN;

  const selectedTitle =
    detail?.manifest.identity.label ??
    detail?.manifest.identity.scenario ??
    detail?.manifest.run_id ??
    EMPTY_TOKEN;
  const selectedContext = [
    detail?.manifest.project.display_name,
    detail?.manifest.identity.suite,
    detail?.manifest.identity.scenario,
  ]
    .filter((value): value is string => Boolean(value && value.trim().length > 0))
    .join(" · ");
  const selectedKicker = [
    detail?.manifest.project.slug ? `project ${detail.manifest.project.slug}` : null,
    selectedSummary?.started_at ? formatRelativeAge(selectedSummary.started_at) : null,
    detail?.manifest.runtime.duration_ms != null
      ? formatDuration(detail.manifest.runtime.duration_ms)
      : null,
  ]
    .filter((value): value is string => Boolean(value && value !== EMPTY_TOKEN))
    .join(" · ");
  const hasPrimaryMetricsCount = items.filter((item) => item.primary_metrics.length > 0).length;
  const baselineRunCount = items.filter((item) => activeBaselineRunIds.has(item.run_id)).length;
  const curatedPrimaryMetrics = selectCuratedMetrics(primaryMetrics);
  const visiblePrimaryMetrics = showAllPrimaryMetrics
    ? primaryMetrics
    : curatedPrimaryMetrics.slice(0, 8);
  const hiddenPrimaryMetricCount = Math.max(0, primaryMetrics.length - visiblePrimaryMetrics.length);
  const compareMode = Boolean(compareTargetId);
  const compareTriggeredFlags =
    compareReport?.regression_flags.filter((flag) => flag.status === "triggered") ?? [];
  const compareHighlights = selectCompareHighlights(compareReport, primaryMetrics);
  const warningGroups = categorizeWarnings(detail?.warnings ?? [], candidateDetail?.warnings ?? []);
  const compareStatusChanged =
    detail && candidateDetail
      ? detail.manifest.runtime.exec_status !== candidateDetail.manifest.runtime.exec_status
      : false;
  const warningDelta =
    detail && candidateDetail
      ? candidateDetail.manifest.summary.warning_count - detail.manifest.summary.warning_count
      : null;
  const artifactDelta =
    detail && candidateDetail
      ? candidateDetail.manifest.artifacts.length - detail.manifest.artifacts.length
      : null;
  const sortedMetricDiffs = sortMetricDiffs(
    compareReport?.metric_diffs ?? [],
    primaryMetrics,
    compareTriggeredFlags,
    metricSortMode,
    { primaryOnly: primaryOnlyMetricDiffs, changedOnly: changedOnlyMetricDiffs },
  );
  const candidatePrimaryMetric = compareSummary
    ? compareSummary.primary_metrics.find((metric) => metric.key === compactMetricKey) ?? null
    : null;
  const compareDecisionSummary =
    compareTriggeredFlags.length > 0
      ? `${compareTriggeredFlags.length} triggered regression flag${compareTriggeredFlags.length === 1 ? "" : "s"}`
      : compareStatusChanged
        ? "Execution status changed"
        : warningGroups.introduced.length > 0
          ? `${warningGroups.introduced.length} new warning${warningGroups.introduced.length === 1 ? "" : "s"} introduced`
          : compareHighlights.length > 0
            ? "Metric deltas detected"
            : "No high-signal changes detected";
  const compareDecisionDetail =
    compareTriggeredFlags.length > 0
      ? "Triggered rules should be reviewed before promoting this candidate."
      : compareStatusChanged
        ? "The candidate execution outcome differs from the base even without a triggered rule."
        : warningGroups.introduced.length > 0
          ? "Warning behavior changed even though no triggered regression rule fired."
        : compareHighlights.length > 0
            ? "Numeric metric movement exists but did not trigger a regression rule."
            : "No triggered regressions, warning shifts, or top-level metric movement were detected.";
  const semanticFlagKeys = new Set(compareTriggeredFlags.map((flag) => flag.metric_key));
  const rankedSemanticHighlights = compareHighlights.map((diff) => ({
    diff,
    semantic: classifyMetricDiffSemantic(diff, semanticFlagKeys.has(diff.key)),
  }));
  const compareSemantic =
    compareTriggeredFlags.length > 0
      ? "regressed"
      : compareStatusChanged || warningGroups.introduced.length > 0
        ? "changed"
        : rankedSemanticHighlights.some((entry) => entry.semantic === "regressed")
          ? "regressed"
          : rankedSemanticHighlights.some((entry) => entry.semantic === "improved")
            ? "improved"
            : compareReport && compareReport.metric_diffs.some((diff) => hasMeaningfulMetricChange(diff))
              ? "changed"
              : compareReport
                ? "stable"
                : "unresolved";
  const compareSemanticLabel = formatCompareSemantic(compareSemantic);
  const semanticCounts = Object.values(compareSemanticMap).reduce<Record<string, number>>(
    (accumulator, semantic) => {
      accumulator[semantic] = (accumulator[semantic] ?? 0) + 1;
      return accumulator;
    },
    {},
  );
  const filterChips = [
    {
      label: "LocalAgent",
      active: filters.project === "localagent",
      onClick: () =>
        setFilters((current) => ({
          ...current,
          project: current.project === "localagent" ? "" : "localagent",
        })),
    },
    {
      label: "VideoForge",
      active: filters.project === "videoforge",
      onClick: () =>
        setFilters((current) => ({
          ...current,
          project: current.project === "videoforge" ? "" : "videoforge",
        })),
    },
    {
      label: "Has metrics",
      active: filters.queryText === "has:metrics",
      onClick: () =>
        setFilters((current) => ({
          ...current,
          queryText: current.queryText === "has:metrics" ? "" : "has:metrics",
        })),
    },
    {
      label: "Has baseline",
      active: filters.queryText === "has:baseline",
      onClick: () =>
        setFilters((current) => ({
          ...current,
          queryText: current.queryText === "has:baseline" ? "" : "has:baseline",
        })),
    },
    {
      label: "Failed",
      active: filters.execStatus === "fail" || filters.execStatus === "error",
      onClick: () =>
        setFilters((current) => ({
          ...current,
          execStatus: current.execStatus === "fail" ? "" : "fail",
        })),
    },
  ];
  const compactMetricOptions = Array.from(
    new Set(
      items.flatMap((item) => item.primary_metrics.map((metric) => metric.key)),
    ),
  ).sort((left, right) => left.localeCompare(right));
  const sortedItems = sortRunsForCompactMode(items, compactSortMode, compactMetricKey);
  const compareFilteredItems = sortedItems
    .filter((item) => {
      if (item.run_id === selectedRunId || compareSemanticFilter === "all") {
        return true;
      }
      return compareSemanticMap[item.run_id] === compareSemanticFilter;
    })
    .sort((left, right) => {
      if (!compareSemanticSort) {
        return 0;
      }
      if (left.run_id === selectedRunId) {
        return -1;
      }
      if (right.run_id === selectedRunId) {
        return 1;
      }
      const leftRank = semanticSortRank(compareSemanticMap[left.run_id]);
      const rightRank = semanticSortRank(compareSemanticMap[right.run_id]);
      if (leftRank !== rightRank) {
        return rightRank - leftRank;
      }
      return 0;
    });

  useEffect(() => {
    if (!compactMetricKey && compactMetricOptions.length > 0) {
      setCompactMetricKey(compactMetricOptions[0]);
    }
  }, [compactMetricKey, compactMetricOptions]);

  async function refreshSelectedRun(runId: string) {
    const nextDetail = await getRun(runId);
    setDetail(nextDetail);
  }

  async function handleSetBaseline() {
    if (!detail) {
      return;
    }
    setSavingBaseline(true);
    setError(null);
    try {
      await setActiveBaseline({ run_id: detail.manifest.run_id, label: "default" });
      await refreshSelectedRun(detail.manifest.run_id);
      const baselines = await listBaselines(detail.manifest.project.slug);
      setProjectBaselines(baselines);
    } catch (caught) {
      setError(String(caught));
    } finally {
      setSavingBaseline(false);
    }
  }

  async function handleCreateRule() {
    if (!detail) {
      return;
    }
    const thresholdValue = Number(ruleDraft.thresholdValue);
    if (!ruleDraft.metricKey || Number.isNaN(thresholdValue)) {
      setError("Regression rule requires a metric key and numeric threshold.");
      return;
    }
    setSavingRule(true);
    setError(null);
    try {
      const request: CreateRegressionRuleRequest = {
        run_id: detail.manifest.run_id,
        label: "default",
        metric_key: ruleDraft.metricKey,
        comparator: ruleDraft.comparator,
        threshold_value: thresholdValue,
      };
      await createRegressionRule(request);
      const rules = await listRegressionRules(detail.manifest.project.slug);
      setRegressionRules(rules);
      if (compareTargetId) {
        const nextReport = await compareRuns(detail.manifest.run_id, compareTargetId);
        setCompareReport(nextReport);
      }
    } catch (caught) {
      setError(String(caught));
    } finally {
      setSavingRule(false);
    }
  }

  async function handleOpenPath(path: string, reveal = false) {
    try {
      if (reveal) {
        await revealPath(path);
      } else {
        await openPath(path);
      }
    } catch (caught) {
      setError(String(caught));
    }
  }

  async function handleCopyPath(path: string) {
    try {
      await navigator.clipboard.writeText(path);
      setPathNotice("Path copied");
      window.setTimeout(() => setPathNotice(null), 1800);
    } catch (caught) {
      setError(String(caught));
    }
  }

  return (
    <main className="app-shell">
      <section className="hero">
        <div className="hero-copy">
          <p className="eyebrow">RunScope Dashboard</p>
          <h1>Local run history tuned for inspection, comparison, and baseline work.</h1>
          <p className="hero-subcopy">
            Keep the list dense, keep the selected run legible, and surface the metrics that
            actually matter first.
          </p>
        </div>
        <div className="hero-stats">
          <SummaryCard label="Visible runs" value={String(total)} detail="Current filter window" />
          <SummaryCard
            label="Latest activity"
            value={formatRelativeAge(latestStartedAt)}
            detail={formatDateTime(latestStartedAt)}
          />
          <SummaryCard
            label="Status split"
            value={`${passCount} pass`}
            detail={`${failCount} fail/error · ${unknownCount} unknown`}
          />
          <SummaryCard
            label="Adapters"
            value={Object.keys(adapterCounts).length > 0 ? adapterSummary : EMPTY_TOKEN}
            detail={
              selectedSummary ? `selected ${selectedSummary.adapter}` : "Select a run to inspect"
            }
          />
          <SummaryCard
            label="Baseline coverage"
            value={baselineCoverageText}
            detail={detail ? "Selected run scope and project baselines" : "No run selected"}
          />
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

      <section className="filter-chip-row">
        {filterChips.map((chip) => (
          <button
            key={chip.label}
            className={`filter-chip ${chip.active ? "active" : ""}`}
            onClick={chip.onClick}
            type="button"
          >
            {chip.label}
          </button>
        ))}
        <span className="quiet filter-chip-summary">
          {hasPrimaryMetricsCount} runs with primary metrics · {baselineRunCount} active baselines in current project view
        </span>
      </section>

      {error ? <div className="banner banner-error">{error}</div> : null}

      <section className="workspace">
        <div className="runs-panel">
          <div className="section-heading">
            <div>
              <p className="section-label">Runs</p>
              <h2>Latest local history</h2>
            </div>
            <div className="runs-toolbar">
              {loadingList ? <span className="quiet">Refreshing...</span> : null}
              <button
                className={`utility-button ${compactList ? "active" : ""}`}
                onClick={() => setCompactList((current) => !current)}
                type="button"
              >
                {compactList ? "Comfortable List" : "Compact List"}
              </button>
            </div>
          </div>
          <section className={`compare-mode-banner ${compareMode ? "active" : ""}`}>
            <div>
              <span className="section-label">Compare Mode</span>
              <strong>
                {compareMode
                  ? `${selectedSummary?.label ?? selectedSummary?.run_id ?? EMPTY_TOKEN} vs ${compareSummary?.label ?? compareSummary?.run_id ?? EMPTY_TOKEN}`
                  : "Choose a candidate from the list to compare against the selected base run."}
              </strong>
            </div>
            <div className="compare-mode-actions">
              {compareMode ? (
                <button
                  className="utility-button"
                  onClick={() => setCompareTargetId(null)}
                  type="button"
                >
                  Clear Compare
                </button>
              ) : null}
            </div>
          </section>
          {compactList ? (
            <section className="compact-sort-bar">
              <span className="section-label">Compact Sort</span>
              <div className="compact-sort-controls">
                <button
                  className={`filter-chip ${compactSortMode === "latest" ? "active" : ""}`}
                  onClick={() => setCompactSortMode("latest")}
                  type="button"
                >
                  Latest
                </button>
                <button
                  className={`filter-chip ${compactSortMode === "warnings" ? "active" : ""}`}
                  onClick={() => setCompactSortMode("warnings")}
                  type="button"
                >
                  Warnings
                </button>
                <button
                  className={`filter-chip ${compactSortMode === "duration" ? "active" : ""}`}
                  onClick={() => setCompactSortMode("duration")}
                  type="button"
                >
                  Duration
                </button>
                <button
                  className={`filter-chip ${compactSortMode === "primary_metric" ? "active" : ""}`}
                  onClick={() => setCompactSortMode("primary_metric")}
                  type="button"
                >
                  Primary Metric
                </button>
                {compactSortMode === "primary_metric" ? (
                  <label className="compact-metric-select">
                    Metric
                    <select
                      value={compactMetricKey}
                      onChange={(event) => setCompactMetricKey(event.target.value)}
                    >
                      {compactMetricOptions.map((key) => (
                        <option key={key} value={key}>
                          {compactMetricLabel(key)}
                        </option>
                      ))}
                    </select>
                  </label>
                ) : null}
              </div>
            </section>
          ) : null}
          {compareMode ? (
            <section className="compare-semantic-bar">
              <div className="compare-semantic-header">
                <span className="section-label">Compare Outcome</span>
                {loadingCompareSemantics ? <span className="quiet">Refreshing outcomes...</span> : null}
              </div>
              <div className="compare-semantic-controls">
                <button
                  className={`filter-chip ${compareSemanticSort ? "active" : ""}`}
                  onClick={() => setCompareSemanticSort((current) => !current)}
                  type="button"
                >
                  Outcome Rank
                </button>
                {(["all", "regressed", "improved", "changed", "unresolved"] as const).map((semantic) => (
                  <button
                    key={semantic}
                    className={`filter-chip ${compareSemanticFilter === semantic ? "active" : ""}`}
                    onClick={() => setCompareSemanticFilter(semantic)}
                    type="button"
                  >
                    {semantic === "all"
                      ? "All"
                      : `${formatCompareSemantic(semantic)} ${semanticCounts[semantic] ?? 0}`}
                  </button>
                ))}
              </div>
            </section>
          ) : null}
          <div className={`runs-list ${compactList ? "compact" : ""}`}>
            {items.length === 0 && !loadingList ? (
              <div className="empty-state">
                <h3>No runs matched the current filters.</h3>
                <p>Ingest a LocalAgent or VideoForge artifact directory to populate the dashboard.</p>
              </div>
            ) : null}
            {(compareMode ? compareFilteredItems : sortedItems).map((item) => {
              const isSelected = selectedRunId === item.run_id;
              const isCompareTarget = compareTargetId === item.run_id;
              const isActiveBaseline = activeBaselineRunIds.has(item.run_id);
              const hasTriggeredRegression =
                isCompareTarget && compareTriggeredFlags.length > 0;
              const compareSemanticForRow =
                !isSelected && compareMode ? compareSemanticMap[item.run_id] ?? null : null;
              const visibleMetrics = item.primary_metrics.slice(0, 3);
              const hiddenMetricCount = Math.max(0, item.primary_metrics.length - visibleMetrics.length);
              const runTitle = item.label ?? item.scenario ?? item.run_id;
              return (
                <button
                  key={item.run_id}
                  className={`run-card status-${item.exec_status} ${compactList ? "compact" : ""} ${isSelected ? "selected" : ""} ${isCompareTarget ? "compare-target" : ""}`}
                  onClick={() => setSelectedRunId(item.run_id)}
                  type="button"
                >
                  <div className="run-card-topline">
                    <span className={`status-pill ${item.exec_status}`}>{item.exec_status}</span>
                    {!compactList ? <span className="adapter-pill">{item.adapter}</span> : null}
                    {isActiveBaseline ? <span className="signal-pill baseline">baseline</span> : null}
                    {isCompareTarget ? <span className="signal-pill candidate">candidate</span> : null}
                    {compareSemanticForRow && compareSemanticForRow !== "stable" ? (
                      <span className={`signal-pill semantic ${compareSemanticForRow}`}>
                        {formatCompareSemantic(compareSemanticForRow)}
                      </span>
                    ) : null}
                    {hasTriggeredRegression ? (
                      <span className="signal-pill regression">
                        {compareTriggeredFlags.length} regression
                      </span>
                    ) : null}
                    {!compactList ? <span className="timestamp">{formatRelativeAge(item.started_at)}</span> : null}
                  </div>
                  <div className={`run-card-title ${compactList ? "compact" : ""}`}>
                    <strong>{runTitle}</strong>
                    {!compactList ? <span>{item.project_slug}</span> : null}
                  </div>
                  {!compactList ? (
                    <div className="run-card-identity">
                      <span>{formatOptionalText(item.suite)}</span>
                      <span>{formatOptionalText(item.scenario)}</span>
                    </div>
                  ) : null}
                  {compactList ? (
                    <div className="compact-inline-meta">
                      <span>{formatOptionalText(item.backend)}</span>
                      <span>{formatOptionalText(item.model)}</span>
                      <span
                        className={`active-triage-slot ${isEmptyCompactTriageSignal(item, compactSortMode, compactMetricKey) ? "empty" : ""}`}
                      >
                        {renderCompactTriageSignal(item, compactSortMode, compactMetricKey)}
                      </span>
                    </div>
                  ) : (
                    <div className="run-card-meta">
                      <span>{formatOptionalText(item.backend)}</span>
                      <span>{formatOptionalText(item.model)}</span>
                      <span>{formatOptionalText(item.precision)}</span>
                      <span>{formatDuration(item.duration_ms)}</span>
                    </div>
                  )}
                  {!compactList ? (
                    <div className="metric-row">
                      {visibleMetrics.length > 0 ? (
                        <>
                          {visibleMetrics.map((metric) => (
                            <div key={`${metric.group_name}:${metric.key}`} className="metric-chip">
                              <span>{compactMetricLabel(metric.key)}</span>
                              <strong>{formatMetric(metric)}</strong>
                            </div>
                          ))}
                          {hiddenMetricCount > 0 ? (
                            <div className="metric-chip more">
                              <span>More metrics</span>
                              <strong>+{hiddenMetricCount}</strong>
                            </div>
                          ) : null}
                        </>
                      ) : (
                        <span className="quiet">No primary metrics</span>
                      )}
                    </div>
                  ) : (
                    <div className="compact-summary-row">
                      <span>
                        {visibleMetrics[0]
                          ? `${compactMetricLabel(visibleMetrics[0].key)} ${formatMetric(visibleMetrics[0])}`
                          : "No primary metrics"}
                      </span>
                      <span className="compact-summary-right">
                        {hiddenMetricCount > 0 ? `+${hiddenMetricCount} more` : EMPTY_TOKEN}
                      </span>
                    </div>
                  )}
                  {!compactList && item.tags.length > 0 ? (
                    <div className="tag-row">
                      {item.tags.map((tag) => (
                        <span key={tag} className="tag-pill">
                          {tag}
                        </span>
                      ))}
                    </div>
                  ) : null}
                  <div className="card-actions">
                    <span className="quiet compact-card-state">
                      {isSelected ? "Base" : isCompareTarget ? "Candidate" : EMPTY_TOKEN}
                    </span>
                    <div className="card-action-buttons">
                      {!isSelected ? (
                        <button
                          className={`compare-button ${compareMode ? "mode-active" : "mode-idle"} ${isCompareTarget ? "active" : ""}`}
                          onClick={(event) => {
                            event.stopPropagation();
                            setCompareTargetId((current) => (current === item.run_id ? null : item.run_id));
                          }}
                          type="button"
                        >
                          {isCompareTarget ? "Candidate" : compareMode ? "Compare" : "Queue"}
                        </button>
                      ) : null}
                    </div>
                  </div>
                </button>
              );
            })}
          </div>
          <section className={`runs-footer-card ${compareMode ? "compare-active" : ""}`}>
            {compareMode ? (
              <>
                <div className="panel-heading compact">
                  <h3>Compare Summary</h3>
                  <p>Keep the left rail in compare mode so the workflow shift is obvious.</p>
                </div>
                <div className="footer-summary-grid">
                  <article className="footer-summary-item">
                    <span className="section-label">Base</span>
                    <strong>{selectedSummary?.label ?? selectedSummary?.run_id ?? EMPTY_TOKEN}</strong>
                    <small>{selectedSummary?.project_slug ?? EMPTY_TOKEN}</small>
                  </article>
                  <article className="footer-summary-item">
                    <span className="section-label">Candidate</span>
                    <strong>{compareSummary?.label ?? compareSummary?.run_id ?? EMPTY_TOKEN}</strong>
                    <small>{compareSummary?.project_slug ?? EMPTY_TOKEN}</small>
                  </article>
                  <article className="footer-summary-item">
                    <span className="section-label">Warnings</span>
                    <strong>{formatDeltaText(warningDelta, "warning")}</strong>
                    <small>
                      {detail?.manifest.summary.warning_count ?? 0} → {candidateDetail?.manifest.summary.warning_count ?? EMPTY_TOKEN}
                    </small>
                  </article>
                  <article className="footer-summary-item">
                    <span className="section-label">Rank Signal</span>
                    <strong>
                      {compactSortMode === "primary_metric" && candidatePrimaryMetric
                        ? formatMetric(candidatePrimaryMetric)
                        : compactSortMode === "warnings"
                          ? `${compareSummary?.warning_count ?? 0}`
                          : compactSortMode === "duration"
                            ? formatDuration(compareSummary?.duration_ms)
                            : formatRelativeAge(compareSummary?.started_at)}
                    </strong>
                    <small>
                      {compactSortMode === "primary_metric"
                        ? compactMetricLabel(compactMetricKey)
                        : compactSortMode.replace(/_/g, " ")}
                    </small>
                  </article>
                  <article className="footer-summary-item compare-focus">
                    <span className="section-label">Compare Focus</span>
                    <strong>{compareSemanticLabel}</strong>
                    <small>
                      {compareTriggeredFlags.length > 0
                        ? "Candidate needs review against the active baseline"
                        : compareStatusChanged
                          ? "Candidate execution differs from the base run"
                        : "Use metric and warning deltas to confirm stability"}
                    </small>
                  </article>
                  <article className="footer-summary-item">
                    <span className="section-label">Outcome Controls</span>
                    <strong>
                      {compareSemanticFilter === "all"
                        ? "All outcomes"
                        : formatCompareSemantic(compareSemanticFilter)}
                    </strong>
                    <small>
                      {compareSemanticSort ? "Outcome rank enabled" : "Outcome rank off"}
                    </small>
                  </article>
                </div>
              </>
            ) : (
              <>
                <div className="panel-heading compact">
                  <h3>Triage Legend</h3>
                  <p>Compact mode stays decision-first when the signal slot mirrors the active sort.</p>
                </div>
                <div className="footer-summary-grid">
                  <article className="footer-summary-item">
                    <span className="section-label">Sort Mode</span>
                    <strong>{compactSortMode.replace(/_/g, " ")}</strong>
                    <small>
                      {compactSortMode === "primary_metric"
                        ? compactMetricLabel(compactMetricKey)
                        : "Active triage slot mirrors this ordering"}
                    </small>
                  </article>
                  <article className="footer-summary-item">
                    <span className="section-label">Baselines</span>
                    <strong>{baselineRunCount}</strong>
                    <small>Active baseline runs in this filtered view</small>
                  </article>
                  <article className="footer-summary-item">
                    <span className="section-label">Metric Coverage</span>
                    <strong>{hasPrimaryMetricsCount}</strong>
                    <small>Runs with primary metrics available for triage</small>
                  </article>
                  <article className="footer-summary-item">
                    <span className="section-label">Candidate Flow</span>
                    <strong>{compareMode ? "active" : "ready"}</strong>
                    <small>Pick any non-base row to begin compare mode</small>
                  </article>
                </div>
              </>
            )}
          </section>
        </div>

        <div className="detail-panel">
          <div className="section-heading">
            <div>
              <p className="section-label">Inspection</p>
              <h2>{compareTargetId ? "Compare Workspace" : "Run inspection"}</h2>
            </div>
            {loadingDetail || loadingCompare ? (
              <span className="quiet">{loadingCompare ? "Comparing..." : "Loading..."}</span>
            ) : null}
          </div>

          {!detail ? (
            <div className="empty-state detail-empty">
              <h3>Select a run to inspect its manifest, metrics, and artifacts.</h3>
              <p>The detail panel uses the stored canonical run.json plus notes, warnings, and tags.</p>
            </div>
          ) : (
            <div className="detail-scroll">
              {compareSummary ? (
                <section className="compare-workspace-hero">
                  <div className="compare-workspace-heading">
                    <div className="detail-title-row">
                      <span className="compare-badge prominent">Base vs Candidate</span>
                      <span className="adapter-pill">{detail.manifest.source.adapter}</span>
                      {matchingBaselineForCompare ? (
                        <span className="signal-pill baseline">matches baseline</span>
                      ) : null}
                    </div>
                    <h3>Compare the selected base run against the queued candidate.</h3>
                    <p>Lead with deltas and regressions first. Deeper run detail stays available below.</p>
                  </div>
                  <section className="compare-identity-ribbon">
                    <CompareRunCard title="Base" item={selectedSummary} />
                    <CompareRunCard title="Candidate" item={compareSummary} />
                  </section>
                  <section className="compare-overview-grid">
                    <article className={`overview-card ${compareStatusChanged ? "changed" : ""}`}>
                      <span className="section-label">Status</span>
                      <strong>
                        {detail.manifest.runtime.exec_status} →{" "}
                        {candidateDetail?.manifest.runtime.exec_status ?? EMPTY_TOKEN}
                      </strong>
                      <small>
                        {compareStatusChanged ? "Execution state changed" : "Execution state unchanged"}
                      </small>
                    </article>
                    <article className={`overview-card ${warningDelta && warningDelta !== 0 ? "changed" : ""}`}>
                      <span className="section-label">Warnings</span>
                      <strong>
                        {detail.manifest.summary.warning_count} →{" "}
                        {candidateDetail?.manifest.summary.warning_count ?? EMPTY_TOKEN}
                      </strong>
                      <small>{formatDeltaText(warningDelta, "warning")}</small>
                    </article>
                    <article className={`overview-card ${artifactDelta && artifactDelta !== 0 ? "changed" : ""}`}>
                      <span className="section-label">Artifacts</span>
                      <strong>
                        {detail.manifest.artifacts.length} →{" "}
                        {candidateDetail?.manifest.artifacts.length ?? EMPTY_TOKEN}
                      </strong>
                      <small>{formatDeltaText(artifactDelta, "artifact")}</small>
                    </article>
                    <article className={`overview-card ${compareTriggeredFlags.length > 0 ? "alert" : ""}`}>
                      <span className="section-label">Regression Flags</span>
                      <strong>{compareTriggeredFlags.length}</strong>
                      <small>
                        {compareTriggeredFlags.length > 0
                          ? "Triggered against the active baseline"
                          : "No triggered flags for this candidate"}
                      </small>
                    </article>
                  </section>
                  <div className="compare-workspace-actions">
                    <button className="baseline-button" onClick={handleSetBaseline} type="button">
                      {savingBaseline ? "Saving Baseline..." : "Set Base As Baseline"}
                    </button>
                    <button
                      className="utility-button"
                      onClick={() => handleOpenPath(detail.run_root)}
                      type="button"
                    >
                      Open Base Folder
                    </button>
                    {candidateDetail ? (
                      <button
                        className="utility-button"
                        onClick={() => handleOpenPath(candidateDetail.run_root)}
                        type="button"
                      >
                        Open Candidate Folder
                      </button>
                    ) : null}
                    <button
                      className="utility-button"
                      onClick={() => handleCopyPath(detail.run_root)}
                      type="button"
                    >
                      Copy Base Path
                    </button>
                  </div>
                  {pathNotice ? <small className="quiet">{pathNotice}</small> : null}
                </section>
              ) : (
                <section className="detail-hero">
                  <div className="detail-copy">
                    <div className="detail-title-row">
                      <span className={`status-pill ${detail.manifest.runtime.exec_status}`}>
                        {detail.manifest.runtime.exec_status}
                      </span>
                      <span className="adapter-pill">{detail.manifest.source.adapter}</span>
                    </div>
                    <h3>{selectedTitle}</h3>
                    <p>{selectedContext || EMPTY_TOKEN}</p>
                    <div className="detail-kicker-row">
                      <span>{selectedKicker || EMPTY_TOKEN}</span>
                    </div>
                  </div>
                  <div className="detail-root">
                    <div className="detail-actions">
                      <button className="baseline-button" onClick={handleSetBaseline} type="button">
                        {savingBaseline ? "Saving Baseline..." : "Set Active Baseline"}
                      </button>
                      <button
                        className="utility-button"
                        onClick={() => handleOpenPath(detail.run_root)}
                        type="button"
                      >
                        Open Folder
                      </button>
                      <button
                        className="utility-button"
                        onClick={() => handleCopyPath(detail.run_root)}
                        type="button"
                      >
                        Copy Path
                      </button>
                      <button
                        className="utility-button"
                        onClick={() => handleOpenPath(joinPath(detail.run_root, "run.json"))}
                        type="button"
                      >
                        Open run.json
                      </button>
                      <button
                        className="utility-button"
                        onClick={() => handleOpenPath(joinPath(detail.run_root, "run.json"), true)}
                        type="button"
                      >
                        Reveal run.json
                      </button>
                    </div>
                    <span>Run root</span>
                    <code>{detail.run_root}</code>
                    {pathNotice ? <small className="quiet">{pathNotice}</small> : null}
                  </div>
                </section>
              )}

              {compareSummary && compareReport ? (
                <>
                  <section className="compare-decision-stack">
                    <Panel
                      title="Regression Outcome"
                      subtitle="Start here: whether this candidate looks suspicious against the current baseline and compare scope."
                    >
                      <div className="compare-outcome-grid">
                        <article
                          className={`overview-card compare-decision-card ${
                            compareTriggeredFlags.length > 0
                              ? "alert"
                              : compareStatusChanged || warningGroups.introduced.length > 0
                                ? "changed"
                                : ""
                          }`}
                        >
                          <span className="section-label">Decision</span>
                          <strong>{compareDecisionSummary}</strong>
                          <small>{compareDecisionDetail}</small>
                        </article>
                        <article className={`overview-card ${compareHighlights.length > 0 ? "changed" : ""}`}>
                          <span className="section-label">Top Delta</span>
                          <strong>
                            {compareHighlights[0]
                              ? compactMetricLabel(compareHighlights[0].key)
                              : EMPTY_TOKEN}
                          </strong>
                          <small>
                            {compareHighlights[0]
                              ? `${formatNumericDelta(compareHighlights[0].abs_delta, compareHighlights[0].unit)} · ${formatPercentDelta(compareHighlights[0].pct_delta)}`
                              : compareDecisionSummary}
                          </small>
                          {compareHighlights[0] ? (
                            <span
                              className={`semantic-badge ${classifyMetricDiffSemantic(compareHighlights[0], semanticFlagKeys.has(compareHighlights[0].key))}`}
                            >
                              {formatCompareSemantic(
                                classifyMetricDiffSemantic(compareHighlights[0], semanticFlagKeys.has(compareHighlights[0].key)),
                              )}
                            </span>
                          ) : null}
                        </article>
                        <article className={`overview-card ${warningDelta && warningDelta !== 0 ? "changed" : ""}`}>
                          <span className="section-label">Warning Shift</span>
                          <strong>{formatDeltaText(warningDelta, "warning")}</strong>
                          <small>
                            {warningGroups.introduced.length > 0
                              ? `${warningGroups.introduced.length} new warning${warningGroups.introduced.length === 1 ? "" : "s"} introduced`
                              : "No newly introduced warnings"}
                          </small>
                        </article>
                      </div>
                    </Panel>

                    <Panel
                      title="Ranked Metric Deltas"
                      subtitle="Highest-value metric changes first, ordered by the active compare sort."
                    >
                      <div className="compare-highlight-grid ranked">
                        {compareHighlights.length > 0 ? (
                          rankedSemanticHighlights.map(({ diff, semantic }) => (
                            <article
                              className={`overview-card metric-delta semantic-${semantic}`}
                              key={`${diff.group_name}:${diff.key}`}
                            >
                              <span className="section-label">{compactMetricLabel(diff.key)}</span>
                              <strong>
                                {formatDiffMetric(diff.left_num, diff.left_text, diff.unit)} →{" "}
                                {formatDiffMetric(diff.right_num, diff.right_text, diff.unit)}
                              </strong>
                              <small>
                                {formatNumericDelta(diff.abs_delta, diff.unit)} · {formatPercentDelta(diff.pct_delta)}
                              </small>
                              <span className={`semantic-badge ${semantic}`}>
                                {formatCompareSemantic(semantic)}
                              </span>
                            </article>
                          ))
                        ) : (
                          <article className="overview-card compare-empty-state">
                            <span className="section-label">No Numeric Deltas</span>
                            <strong>{compareDecisionSummary}</strong>
                            <small>{compareDecisionDetail}</small>
                          </article>
                        )}
                      </div>
                    </Panel>
                  </section>

                  <Panel
                    title="Warning Delta"
                    subtitle="Shared, introduced, and resolved warnings split so newly suspicious runs stand out immediately."
                  >
                    <div className="warning-groups">
                      <WarningBucket title="Introduced" items={warningGroups.introduced} tone="alert" />
                      <WarningBucket title="Resolved" items={warningGroups.resolved} tone="ok" />
                      <WarningBucket title="Candidate Only" items={warningGroups.candidateOnly} />
                      <WarningBucket title="Base Only" items={warningGroups.baseOnly} />
                    </div>
                    {warningGroups.shared.length > 0 ? (
                      <div className="warning-shared">
                        <span className="section-label">Shared warnings</span>
                        <strong>{warningGroups.shared.length}</strong>
                      </div>
                    ) : (
                      <span className="quiet">No shared warnings across the two runs.</span>
                    )}
                  </Panel>

                  <Panel
                    title="Regression Flags"
                    subtitle="Candidate run evaluated against the active baseline for the same scope using stored regression rules."
                  >
                    {compareReport.regression_flags.length > 0 ? (
                      <div className="flag-list">
                        {compareReport.regression_flags.map((flag) => (
                          <article className={`flag-item ${flag.status}`} key={`${flag.label}:${flag.metric_key}:${flag.comparator}`}>
                            <div className="flag-topline">
                              <strong>{flag.metric_key}</strong>
                              <span className={`flag-status ${flag.status}`}>{flag.status}</span>
                            </div>
                            <span>
                              {flag.comparator} threshold {flag.threshold_value}
                            </span>
                            <span>
                              actual {flag.actual_value != null ? flag.actual_value.toFixed(2) : EMPTY_TOKEN} · baseline {flag.baseline_run_id}
                            </span>
                          </article>
                        ))}
                      </div>
                    ) : (
                      <span className="quiet">No regression rules matched the candidate baseline scope yet.</span>
                    )}
                  </Panel>

                  <section className="compare-secondary-stack">
                  <Panel title="Metadata Diffs" subtitle="Core-owned metadata comparisons across the normalized manifest.">
                    <table className="data-table">
                      <thead>
                        <tr>
                          <th>Field</th>
                          <th>Base</th>
                          <th>Candidate</th>
                        </tr>
                      </thead>
                      <tbody>
                        {compareReport.metadata_diffs.length > 0 ? (
                          compareReport.metadata_diffs.map((diff) => (
                            <tr key={diff.field}>
                              <td>{diff.field}</td>
                              <td>{formatOptionalText(diff.left)}</td>
                              <td>{formatOptionalText(diff.right)}</td>
                            </tr>
                          ))
                        ) : (
                          <tr>
                            <td colSpan={3} className="quiet">
                              No metadata differences across the tracked compare fields.
                            </td>
                          </tr>
                        )}
                      </tbody>
                    </table>
                  </Panel>

                  <Panel title="Metric Diffs" subtitle="Absolute and percent deltas are computed in core when both values are numeric.">
                    <div className="metric-diff-toolbar">
                      <div className="filter-chip-row metric-diff-filters">
                        <button
                          className={`filter-chip ${metricSortMode === "severity" ? "active" : ""}`}
                          onClick={() => setMetricSortMode("severity")}
                          type="button"
                        >
                          Severity
                        </button>
                        <button
                          className={`filter-chip ${metricSortMode === "abs_delta" ? "active" : ""}`}
                          onClick={() => setMetricSortMode("abs_delta")}
                          type="button"
                        >
                          Abs Delta
                        </button>
                        <button
                          className={`filter-chip ${metricSortMode === "pct_delta" ? "active" : ""}`}
                          onClick={() => setMetricSortMode("pct_delta")}
                          type="button"
                        >
                          Percent Delta
                        </button>
                        <button
                          className={`filter-chip ${metricSortMode === "primary" ? "active" : ""}`}
                          onClick={() => setMetricSortMode("primary")}
                          type="button"
                        >
                          Primary First
                        </button>
                        <button
                          className={`filter-chip ${primaryOnlyMetricDiffs ? "active" : ""}`}
                          onClick={() => setPrimaryOnlyMetricDiffs((current) => !current)}
                          type="button"
                        >
                          Primary Only
                        </button>
                        <button
                          className={`filter-chip ${changedOnlyMetricDiffs ? "active" : ""}`}
                          onClick={() => setChangedOnlyMetricDiffs((current) => !current)}
                          type="button"
                        >
                          Changed Only
                        </button>
                      </div>
                    </div>
                    <div className="table-scroll-shell metric-diffs-shell">
                      <table className="data-table">
                        <thead>
                          <tr>
                            <th>Metric</th>
                            <th>State</th>
                            <th>Base</th>
                            <th>Candidate</th>
                            <th>Abs Delta</th>
                            <th>Percent Delta</th>
                          </tr>
                        </thead>
                        <tbody>
                          {sortedMetricDiffs.length > 0 ? (
                            sortedMetricDiffs.map((diff) => {
                              const semantic = classifyMetricDiffSemantic(diff, semanticFlagKeys.has(diff.key));
                              return (
                            <tr key={`${diff.group_name}:${diff.key}`} className={`metric-diff-row semantic-${semantic}`}>
                              <td>
                                <div className="metric-name-cell">
                                  <strong>{compactMetricLabel(diff.key)}</strong>
                                  <span>{formatOptionalText(diff.group_name)}</span>
                                </div>
                              </td>
                              <td>
                                <span className={`semantic-badge ${semantic}`}>
                                  {formatCompareSemantic(semantic)}
                                </span>
                              </td>
                              <td>{formatDiffMetric(diff.left_num, diff.left_text, diff.unit)}</td>
                              <td>{formatDiffMetric(diff.right_num, diff.right_text, diff.unit)}</td>
                              <td>{formatNumericDelta(diff.abs_delta, diff.unit)}</td>
                              <td>{formatPercentDelta(diff.pct_delta)}</td>
                            </tr>
                              );
                            })
                          ) : (
                            <tr>
                              <td colSpan={6} className="quiet">
                                No metric diffs matched the current compare filters.
                              </td>
                            </tr>
                          )}
                        </tbody>
                      </table>
                    </div>
                  </Panel>

                  <Panel title="Artifact Diffs" subtitle="Artifact presence and managed relative path changes between the two runs.">
                    <table className="data-table">
                      <thead>
                        <tr>
                          <th>Role</th>
                          <th>Base Path</th>
                          <th>Candidate Path</th>
                        </tr>
                      </thead>
                      <tbody>
                        {compareReport.artifact_diffs.length > 0 ? (
                          compareReport.artifact_diffs.map((diff) => (
                            <tr key={diff.role}>
                              <td>{diff.role}</td>
                              <td>{formatOptionalText(diff.left_rel_path ?? "missing")}</td>
                              <td>{formatOptionalText(diff.right_rel_path ?? "missing")}</td>
                            </tr>
                          ))
                        ) : (
                          <tr>
                            <td colSpan={3} className="quiet">
                              No artifact path differences across matched roles.
                            </td>
                          </tr>
                        )}
                      </tbody>
                    </table>
                  </Panel>
                  </section>
                </>
              ) : null}

              {compareSummary ? (
                <div className="compare-reference-header">
                  <span className="section-label">Base Run Reference</span>
                  <p>Reference panels stay available below, but compare outcome and diffs lead the workflow.</p>
                </div>
              ) : null}
              <section className={`detail-grid ${compareSummary ? "compare-reference-grid" : ""}`}>
                <InfoCard title="Identity" subtle={Boolean(compareSummary)}>
                  <InfoRow label="Project" value={detail.manifest.project.slug} />
                  <InfoRow label="Suite" value={detail.manifest.identity.suite} />
                  <InfoRow label="Scenario" value={detail.manifest.identity.scenario} />
                  <InfoRow label="Label" value={detail.manifest.identity.label} />
                  <InfoRow label="Run ID" value={detail.manifest.run_id} />
                </InfoCard>
                <InfoCard title="Execution" subtle={Boolean(compareSummary)}>
                  <InfoRow label="Status" value={detail.manifest.runtime.exec_status} />
                  <InfoRow label="Started" value={formatDateTime(detail.manifest.runtime.started_at)} />
                  <InfoRow label="Finished" value={formatDateTime(detail.manifest.runtime.finished_at)} />
                  <InfoRow label="Duration" value={formatDuration(detail.manifest.runtime.duration_ms)} />
                  <InfoRow label="Warnings" value={detail.manifest.summary.warning_count.toString()} />
                </InfoCard>
                <InfoCard title="Environment" subtle={Boolean(compareSummary)}>
                  <InfoRow label="Backend" value={detail.manifest.environment?.backend} />
                  <InfoRow label="Model" value={detail.manifest.environment?.model} />
                  <InfoRow label="Precision" value={detail.manifest.environment?.precision} />
                  <InfoRow label="Machine" value={detail.manifest.environment?.machine_name} />
                  <InfoRow label="GPU" value={detail.manifest.environment?.gpu} />
                </InfoCard>
                <InfoCard title="Workload" subtle={Boolean(compareSummary)}>
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

              <Panel
                title="Key Metrics"
                subtitle={
                  hiddenPrimaryMetricCount > 0
                    ? "Curated top metrics first, with the remainder available on demand."
                    : "Primary metrics shown in the list for comparison readiness."
                }
              >
                <div className="metric-grid">
                  {visiblePrimaryMetrics.length > 0 ? (
                    visiblePrimaryMetrics.map((metric) => (
                      <div className="metric-tile" key={`${metric.group_name}:${metric.key}`}>
                        <span>{compactMetricLabel(metric.key)}</span>
                        <strong>{formatMetric(metric)}</strong>
                        <small>{metric.direction.replace(/_/g, " ")}</small>
                      </div>
                    ))
                  ) : (
                    <div className="quiet">No primary metrics were marked for this run.</div>
                  )}
                </div>
                {hiddenPrimaryMetricCount > 0 ? (
                  <div className="panel-actions">
                    <button
                      className="utility-button"
                      onClick={() => setShowAllPrimaryMetrics((current) => !current)}
                      type="button"
                    >
                      {showAllPrimaryMetrics
                        ? "Show Curated Set"
                        : `Show Remaining ${hiddenPrimaryMetricCount}`}
                    </button>
                  </div>
                ) : null}
              </Panel>

              <Panel
                title="All Metrics"
                subtitle="Reference-grade normalized metric records from the canonical manifest."
                className="subtle-panel"
              >
                <div className="table-scroll-shell metrics-table-shell">
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
                          <td>{formatOptionalText(metric.group_name)}</td>
                          <td>{formatMetric(metric)}</td>
                          <td>{metric.direction.replace(/_/g, " ")}</td>
                          <td>{metric.is_primary ? "yes" : EMPTY_TOKEN}</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </Panel>

              <Panel title="Artifacts" subtitle="Managed relative paths preserved under the local run root.">
                <table className="data-table">
                  <thead>
                    <tr>
                      <th>Role</th>
                      <th>Relative path</th>
                      <th>Media type</th>
                      <th>Size</th>
                      <th>Actions</th>
                    </tr>
                  </thead>
                  <tbody>
                    {detail.manifest.artifacts.map((artifact) => (
                      <tr key={`${artifact.role}:${artifact.rel_path}`}>
                        <td>{artifact.role}</td>
                        <td>
                          <code>{artifact.rel_path}</code>
                        </td>
                        <td>{formatOptionalText(artifact.media_type)}</td>
                        <td>{artifact.size_bytes != null ? `${artifact.size_bytes} B` : EMPTY_TOKEN}</td>
                        <td>
                          <div className="table-actions">
                            <button
                              className="utility-button inline"
                              onClick={() => handleOpenPath(joinPath(detail.run_root, artifact.rel_path))}
                              type="button"
                            >
                              Open
                            </button>
                            <button
                              className="utility-button inline"
                              onClick={() => handleOpenPath(joinPath(detail.run_root, artifact.rel_path), true)}
                              type="button"
                            >
                              Reveal
                            </button>
                            <button
                              className="utility-button inline"
                              onClick={() => handleCopyPath(joinPath(detail.run_root, artifact.rel_path))}
                              type="button"
                            >
                              Copy
                            </button>
                          </div>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </Panel>

              <div className="detail-grid secondary">
                <Panel title="Baselines" subtitle="Active scoped baselines matching this run and the wider project.">
                  {detail.active_baselines.length > 0 ? (
                    <div className="baseline-list">
                      {detail.active_baselines.map((baseline) => (
                        <article className="baseline-item" key={baseline.id}>
                          <strong>{baseline.label}</strong>
                          <span>Run {baseline.run_id}</span>
                          <span>
                            Branch {formatOptionalText(baseline.scope.branch)} · Backend{" "}
                            {formatOptionalText(baseline.scope.backend)}
                          </span>
                        </article>
                      ))}
                    </div>
                  ) : (
                    <span className="quiet">No active baseline currently matches this run scope.</span>
                  )}
                  {projectBaselines.length > 0 ? (
                    <div className="baseline-list project">
                      {projectBaselines.map((baseline) => (
                        <article className="baseline-item" key={`project-${baseline.id}`}>
                          <strong>{baseline.label}</strong>
                          <span>{baseline.run_id}</span>
                          <span>
                            {formatOptionalText(baseline.scope.suite)} · {formatOptionalText(baseline.scope.scenario)} ·{" "}
                            {formatOptionalText(baseline.scope.backend)}
                          </span>
                        </article>
                      ))}
                    </div>
                  ) : null}
                </Panel>
                <Panel title="Regression Rules" subtitle="Scope-aware rules evaluated when a candidate run has an active baseline for this scope.">
                  <div className="rule-form">
                    <label>
                      Metric
                      <select
                        value={ruleDraft.metricKey}
                        onChange={(event) =>
                          setRuleDraft((current) => ({ ...current, metricKey: event.target.value }))
                        }
                      >
                        {detail.manifest.metrics.map((metric) => (
                          <option key={metric.key} value={metric.key}>
                            {metric.key}
                          </option>
                        ))}
                      </select>
                    </label>
                    <label>
                      Comparator
                      <select
                        value={ruleDraft.comparator}
                        onChange={(event) =>
                          setRuleDraft((current) => ({
                            ...current,
                            comparator: event.target.value as RegressionComparator,
                          }))
                        }
                      >
                        <option value="pct_drop_gt">pct_drop_gt</option>
                        <option value="pct_increase_gt">pct_increase_gt</option>
                        <option value="abs_delta_gt">abs_delta_gt</option>
                        <option value="abs_delta_lt">abs_delta_lt</option>
                      </select>
                    </label>
                    <label>
                      Threshold
                      <input
                        value={ruleDraft.thresholdValue}
                        onChange={(event) =>
                          setRuleDraft((current) => ({
                            ...current,
                            thresholdValue: event.target.value,
                          }))
                        }
                        placeholder="5"
                      />
                    </label>
                    <button className="baseline-button" onClick={handleCreateRule} type="button">
                      {savingRule ? "Saving Rule..." : "Add Rule For This Scope"}
                    </button>
                  </div>
                  {regressionRules.length > 0 ? (
                    <div className="baseline-list project">
                      {regressionRules.map((rule) => (
                        <article className="baseline-item" key={`rule-${rule.id}`}>
                          <strong>{rule.metric_key}</strong>
                          <span>
                            {rule.comparator} · threshold {rule.threshold_value}
                          </span>
                          <span>
                            {formatOptionalText(rule.scope.suite)} · {formatOptionalText(rule.scope.scenario)} ·{" "}
                            {formatOptionalText(rule.scope.backend)}
                          </span>
                        </article>
                      ))}
                    </div>
                  ) : (
                    <span className="quiet">No regression rules defined for this project yet.</span>
                  )}
                </Panel>
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

function formatDiffMetric(valueNum: number | null, valueText: string | null, unit: string | null): string {
  if (valueNum != null) {
    const metric: MetricRecord = {
      key: "",
      group_name: "",
      value_num: valueNum,
      value_text: null,
      unit,
      direction: "none",
      is_primary: false,
    };
    return formatMetric(metric);
  }
  return formatOptionalText(valueText);
}

function sortMetricDiffs(
  metricDiffs: CompareReport["metric_diffs"],
  primaryMetrics: MetricRecord[],
  triggeredFlags: CompareReport["regression_flags"],
  sortMode: MetricSortMode,
  filters: { primaryOnly: boolean; changedOnly: boolean },
) {
  const primaryKeys = new Set(primaryMetrics.map((metric) => metric.key));
  const triggeredKeys = new Set(
    triggeredFlags.filter((flag) => flag.status === "triggered").map((flag) => flag.metric_key),
  );

  return [...metricDiffs]
    .filter((diff) => {
      const changed =
        diff.abs_delta != null ||
        diff.pct_delta != null ||
        diff.left_text !== diff.right_text ||
        diff.left_num !== diff.right_num;
      if (filters.primaryOnly && !primaryKeys.has(diff.key)) {
        return false;
      }
      if (filters.changedOnly && !changed) {
        return false;
      }
      return true;
    })
    .sort((left, right) => {
      const leftPrimary = primaryKeys.has(left.key) ? 1 : 0;
      const rightPrimary = primaryKeys.has(right.key) ? 1 : 0;
      const leftTriggered = triggeredKeys.has(left.key) ? 1 : 0;
      const rightTriggered = triggeredKeys.has(right.key) ? 1 : 0;

      if (sortMode === "severity") {
        if (leftTriggered !== rightTriggered) {
          return rightTriggered - leftTriggered;
        }
        if (leftPrimary !== rightPrimary) {
          return rightPrimary - leftPrimary;
        }
        return Math.abs(right.abs_delta ?? 0) - Math.abs(left.abs_delta ?? 0);
      }
      if (sortMode === "abs_delta") {
        return Math.abs(right.abs_delta ?? 0) - Math.abs(left.abs_delta ?? 0);
      }
      if (sortMode === "pct_delta") {
        return Math.abs(right.pct_delta ?? 0) - Math.abs(left.pct_delta ?? 0);
      }
      if (leftPrimary !== rightPrimary) {
        return rightPrimary - leftPrimary;
      }
      return Math.abs(right.abs_delta ?? 0) - Math.abs(left.abs_delta ?? 0);
    });
}

function classifyMetricDiffSemantic(
  diff: CompareReport["metric_diffs"][number],
  hasTriggeredRegression: boolean,
): CompareSemantic {
  if (hasTriggeredRegression) {
    return "regressed";
  }

  const hasNumeric = diff.left_num != null && diff.right_num != null;
  const hasTextChange = diff.left_text !== diff.right_text;
  const hasMissingSide =
    (diff.left_num == null && diff.left_text == null) || (diff.right_num == null && diff.right_text == null);

  if (hasMissingSide) {
    return hasMeaningfulMetricChange(diff) ? "unresolved" : "stable";
  }

  if (hasNumeric) {
    const delta = (diff.right_num ?? 0) - (diff.left_num ?? 0);
    if (delta === 0) {
      return "stable";
    }
    if (diff.direction === "higher_is_better") {
      return delta > 0 ? "improved" : "regressed";
    }
    if (diff.direction === "lower_is_better") {
      return delta < 0 ? "improved" : "regressed";
    }
    if (diff.direction === "none") {
      return "changed";
    }
    return "unresolved";
  }

  if (hasTextChange) {
    return "changed";
  }

  return "stable";
}

function hasMeaningfulMetricChange(diff: CompareReport["metric_diffs"][number]): boolean {
  return (
    diff.abs_delta != null ||
    diff.pct_delta != null ||
    diff.left_text !== diff.right_text ||
    diff.left_num !== diff.right_num
  );
}

function formatCompareSemantic(semantic: CompareSemantic): string {
  if (semantic === "improved") {
    return "Improved";
  }
  if (semantic === "regressed") {
    return "Regressed";
  }
  if (semantic === "changed") {
    return "Changed";
  }
  if (semantic === "unresolved") {
    return "Unresolved";
  }
  return "Stable";
}

function deriveCompareSemanticFromReport(
  report: CompareReport,
  baseItem: RunListItem,
  candidateItem: RunListItem,
): CompareSemantic {
  const triggeredKeys = new Set(
    report.regression_flags.filter((flag) => flag.status === "triggered").map((flag) => flag.metric_key),
  );
  if (triggeredKeys.size > 0) {
    return "regressed";
  }
  if (baseItem.exec_status !== candidateItem.exec_status) {
    return "changed";
  }
  if (baseItem.warning_count !== candidateItem.warning_count) {
    return "changed";
  }

  const highlightedSemantics = selectCompareHighlights(report, baseItem.primary_metrics).map((diff) =>
    classifyMetricDiffSemantic(diff, triggeredKeys.has(diff.key)),
  );
  if (highlightedSemantics.includes("regressed")) {
    return "regressed";
  }
  if (highlightedSemantics.includes("improved")) {
    return "improved";
  }
  if (highlightedSemantics.includes("changed")) {
    return "changed";
  }
  if (highlightedSemantics.includes("unresolved")) {
    return "unresolved";
  }
  if (report.metric_diffs.some((diff) => hasMeaningfulMetricChange(diff))) {
    return "changed";
  }
  return "stable";
}

function semanticSortRank(semantic: CompareSemantic | undefined): number {
  if (semantic === "regressed") {
    return 5;
  }
  if (semantic === "unresolved") {
    return 4;
  }
  if (semantic === "changed") {
    return 3;
  }
  if (semantic === "improved") {
    return 2;
  }
  if (semantic === "stable") {
    return 1;
  }
  return 0;
}

function sortRunsForCompactMode(
  items: RunListItem[],
  sortMode: CompactSortMode,
  metricKey: string,
) {
  return [...items].sort((left, right) => {
    if (sortMode === "latest") {
      return compareNullableDate(right.started_at, left.started_at);
    }
    if (sortMode === "warnings") {
      if (right.warning_count !== left.warning_count) {
        return right.warning_count - left.warning_count;
      }
      return compareNullableDate(right.started_at, left.started_at);
    }
    if (sortMode === "duration") {
      const durationDelta = (right.duration_ms ?? -1) - (left.duration_ms ?? -1);
      if (durationDelta !== 0) {
        return durationDelta;
      }
      return compareNullableDate(right.started_at, left.started_at);
    }

    const leftMetric = extractPrimaryMetricValue(left, metricKey);
    const rightMetric = extractPrimaryMetricValue(right, metricKey);
    if (rightMetric !== leftMetric) {
      return rightMetric - leftMetric;
    }
    return compareNullableDate(right.started_at, left.started_at);
  });
}

function extractPrimaryMetricValue(item: RunListItem, metricKey: string): number {
  const metric = item.primary_metrics.find((entry) => entry.key === metricKey);
  return metric?.value_num ?? Number.NEGATIVE_INFINITY;
}

function compareNullableDate(left: string | null, right: string | null): number {
  const leftValue = left ? new Date(left).getTime() : Number.NEGATIVE_INFINITY;
  const rightValue = right ? new Date(right).getTime() : Number.NEGATIVE_INFINITY;
  return leftValue - rightValue;
}

function renderCompactTriageSignal(
  item: RunListItem,
  sortMode: CompactSortMode,
  metricKey: string,
) {
  if (sortMode === "warnings") {
    return (
      <>
        <strong>{item.warning_count}</strong>
        <span>warnings</span>
      </>
    );
  }

  if (sortMode === "duration") {
    return (
      <>
        <strong>{formatDuration(item.duration_ms)}</strong>
        <span>duration</span>
      </>
    );
  }

  if (sortMode === "primary_metric") {
    const metric = item.primary_metrics.find((entry) => entry.key === metricKey);
    return (
      <>
        <strong>{metric ? formatMetric(metric) : EMPTY_TOKEN}</strong>
        <span>
          {metric
            ? `${compactMetricLabel(metric.key)} · ${metric.direction.replace(/_/g, " ")}`
            : compactMetricLabel(metricKey)}
        </span>
      </>
    );
  }

  return (
    <>
      <strong>{formatRelativeAge(item.started_at)}</strong>
      <span>{formatDateTime(item.started_at)}</span>
    </>
  );
}

function isEmptyCompactTriageSignal(
  item: RunListItem,
  sortMode: CompactSortMode,
  metricKey: string,
) {
  if (sortMode === "warnings") {
    return item.warning_count === 0;
  }
  if (sortMode === "duration") {
    return item.duration_ms == null;
  }
  if (sortMode === "primary_metric") {
    return !item.primary_metrics.some((entry) => entry.key === metricKey && entry.value_num != null);
  }
  return !item.started_at;
}

function categorizeWarnings(baseWarnings: RunDetail["warnings"], candidateWarnings: RunDetail["warnings"]) {
  const baseMap = new Map(baseWarnings.map((warning) => [warningIdentity(warning), warning]));
  const candidateMap = new Map(
    candidateWarnings.map((warning) => [warningIdentity(warning), warning]),
  );
  const shared: RunDetail["warnings"] = [];
  const baseOnly: RunDetail["warnings"] = [];
  const candidateOnly: RunDetail["warnings"] = [];

  baseWarnings.forEach((warning) => {
    const key = warningIdentity(warning);
    if (candidateMap.has(key)) {
      shared.push(warning);
    } else {
      baseOnly.push(warning);
    }
  });

  candidateWarnings.forEach((warning) => {
    const key = warningIdentity(warning);
    if (!baseMap.has(key)) {
      candidateOnly.push(warning);
    }
  });

  return {
    shared,
    baseOnly,
    candidateOnly,
    introduced: candidateOnly,
    resolved: baseOnly,
  };
}

function warningIdentity(warning: RunDetail["warnings"][number]): string {
  return `${warning.code}:${warning.message}`;
}

function selectCuratedMetrics(metrics: MetricRecord[]): MetricRecord[] {
  return [...metrics].sort((left, right) => {
    const leftIsGeneric = left.key.includes("by_model") ? 1 : 0;
    const rightIsGeneric = right.key.includes("by_model") ? 1 : 0;
    if (leftIsGeneric !== rightIsGeneric) {
      return leftIsGeneric - rightIsGeneric;
    }
    return left.key.length - right.key.length;
  });
}

function selectCompareHighlights(
  report: CompareReport | null,
  primaryMetrics: MetricRecord[],
) {
  if (!report) {
    return [];
  }
  const primaryKeys = new Set(primaryMetrics.map((metric) => metric.key));
  return [...report.metric_diffs]
    .filter((diff) => diff.abs_delta != null || diff.pct_delta != null)
    .sort((left, right) => {
      const leftPrimary = primaryKeys.has(left.key) ? 1 : 0;
      const rightPrimary = primaryKeys.has(right.key) ? 1 : 0;
      if (leftPrimary !== rightPrimary) {
        return rightPrimary - leftPrimary;
      }
      return Math.abs(right.abs_delta ?? 0) - Math.abs(left.abs_delta ?? 0);
    })
    .slice(0, 6);
}

function formatDeltaText(delta: number | null, noun: string): string {
  if (delta == null || delta === 0) {
    return `No ${noun} count change`;
  }
  const prefix = delta > 0 ? "+" : "";
  return `${prefix}${delta} ${noun}${Math.abs(delta) === 1 ? "" : "s"}`;
}

function joinPath(base: string, relPath: string): string {
  const normalizedBase = base.replace(/[\\/]+$/, "");
  const normalizedRel = relPath.replace(/[\\/]+/g, "\\");
  return `${normalizedBase}\\${normalizedRel}`;
}

function SummaryCard(props: { label: string; value: string; detail: string }) {
  return (
    <div className="summary-card">
      <span>{props.label}</span>
      <strong>{props.value}</strong>
      <small>{props.detail}</small>
    </div>
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

function Panel(props: { title: string; subtitle: string; children: ReactNode; className?: string }) {
  return (
    <section className={`panel ${props.className ?? ""}`.trim()}>
      <div className="panel-heading">
        <h3>{props.title}</h3>
        <p>{props.subtitle}</p>
      </div>
      {props.children}
    </section>
  );
}

function InfoCard(props: { title: string; children: ReactNode; subtle?: boolean }) {
  return (
    <section className={`info-card ${props.subtle ? "subtle-panel" : ""}`.trim()}>
      <h3>{props.title}</h3>
      <div className="info-rows">{props.children}</div>
    </section>
  );
}

function InfoRow(props: { label: string; value: string | null | undefined }) {
  const formattedValue = formatOptionalText(props.value);
  return (
    <div className={`info-row ${formattedValue === EMPTY_TOKEN ? "is-empty" : ""}`}>
      <span>{props.label}</span>
      <strong className={formattedValue === EMPTY_TOKEN ? "empty-token" : undefined}>
        {formattedValue}
      </strong>
    </div>
  );
}

function CompareRunCard(props: { title: string; item: RunListItem | null }) {
  return (
    <article className="compare-run-card">
      <span className="section-label">{props.title}</span>
      {props.item ? (
        <>
          <strong>{props.item.label ?? props.item.run_id}</strong>
          <span>{props.item.project_slug}</span>
          <span>
            {formatOptionalText(props.item.backend)} · {formatOptionalText(props.item.model)} ·{" "}
            {formatOptionalText(props.item.precision)}
          </span>
        </>
      ) : (
        <span className="quiet">No run selected</span>
      )}
    </article>
  );
}

function WarningBucket(props: {
  title: string;
  items: RunDetail["warnings"];
  tone?: "alert" | "ok";
}) {
  return (
    <article className={`warning-bucket ${props.tone ?? ""}`}>
      <div className="warning-bucket-header">
        <span className="section-label">{props.title}</span>
        <strong>{props.items.length}</strong>
      </div>
      {props.items.length > 0 ? (
        props.items.slice(0, 3).map((warning) => (
          <div className="warning-bucket-item" key={`${warning.created_at}:${warning.code}:${warning.message}`}>
            <strong>{warning.code}</strong>
            <span>{warning.message}</span>
          </div>
        ))
      ) : (
        <span className="quiet">None</span>
      )}
    </article>
  );
}
