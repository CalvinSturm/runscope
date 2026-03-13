import { invoke } from "@tauri-apps/api/core";
import type {
  BaselineBinding,
  CreateRegressionRuleRequest,
  CompareReport,
  RegressionRule,
  RunDetail,
  RunListFilter,
  RunListPage,
  SetBaselineRequest,
} from "./types";

export async function listRuns(filter: RunListFilter): Promise<RunListPage> {
  return invoke<RunListPage>("list_runs", { filter });
}

export async function getRun(runId: string): Promise<RunDetail> {
  return invoke<RunDetail>("get_run", { runId });
}

export async function compareRuns(leftRunId: string, rightRunId: string): Promise<CompareReport> {
  return invoke<CompareReport>("compare_runs", { leftRunId, rightRunId });
}

export async function setActiveBaseline(req: SetBaselineRequest): Promise<BaselineBinding> {
  return invoke<BaselineBinding>("set_active_baseline", { req });
}

export async function listBaselines(projectSlug: string): Promise<BaselineBinding[]> {
  return invoke<BaselineBinding[]>("list_baselines", { projectSlug });
}

export async function createRegressionRule(
  req: CreateRegressionRuleRequest,
): Promise<RegressionRule> {
  return invoke<RegressionRule>("create_regression_rule", { req });
}

export async function listRegressionRules(projectSlug: string): Promise<RegressionRule[]> {
  return invoke<RegressionRule[]>("list_regression_rules", { projectSlug });
}

export async function openPath(path: string): Promise<void> {
  return invoke<void>("open_path", { path });
}

export async function revealPath(path: string): Promise<void> {
  return invoke<void>("reveal_path", { path });
}
