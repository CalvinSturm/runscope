import { invoke } from "@tauri-apps/api/core";
import type { RunDetail, RunListFilter, RunListPage } from "./types";

export async function listRuns(filter: RunListFilter): Promise<RunListPage> {
  return invoke<RunListPage>("list_runs", { filter });
}

export async function getRun(runId: string): Promise<RunDetail> {
  return invoke<RunDetail>("get_run", { runId });
}
