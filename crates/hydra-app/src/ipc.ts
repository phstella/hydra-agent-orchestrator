import { invoke } from "@tauri-apps/api/core";
import type {
  DoctorReport,
  HydraConfig,
  MergeReport,
  RaceResult,
  RunEvent,
  RunManifest,
} from "./types";

export async function startRace(
  agent: string,
  prompt: string,
): Promise<RaceResult> {
  return invoke<RaceResult>("start_race", { agent, prompt });
}

export async function getDoctorReport(): Promise<DoctorReport> {
  return invoke<DoctorReport>("get_doctor_report");
}

export async function getRunManifest(runId: string): Promise<RunManifest> {
  return invoke<RunManifest>("get_run_manifest", { runId });
}

export async function getRunEvents(runId: string): Promise<RunEvent[]> {
  return invoke<RunEvent[]>("get_run_events", { runId });
}

export async function mergeDryRun(
  sourceBranch: string,
  targetBranch: string,
): Promise<MergeReport> {
  return invoke<MergeReport>("merge_dry_run", { sourceBranch, targetBranch });
}

export async function mergeConfirm(
  sourceBranch: string,
  targetBranch: string,
): Promise<MergeReport> {
  return invoke<MergeReport>("merge_confirm", { sourceBranch, targetBranch });
}

export async function getConfig(): Promise<HydraConfig> {
  return invoke<HydraConfig>("get_config");
}
