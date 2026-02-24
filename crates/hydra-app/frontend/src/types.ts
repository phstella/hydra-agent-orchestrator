/**
 * IPC types shared between Tauri backend and React frontend.
 * These mirror the Rust `ipc_types` module 1:1.
 */

import type {
  AdapterTier,
  CapabilityConfidence,
  CheckStatus,
  DetectStatus,
} from './generated/rust-enums';

export type { AdapterTier, CapabilityConfidence, CheckStatus, DetectStatus };

// ---------------------------------------------------------------------------
// Adapter types
// ---------------------------------------------------------------------------

export interface CapabilityEntry {
  supported: boolean;
  confidence: CapabilityConfidence;
}

export interface CapabilitySet {
  json_stream: CapabilityEntry;
  plain_text: CapabilityEntry;
  force_edit_mode: CapabilityEntry;
  sandbox_controls: CapabilityEntry;
  approval_controls: CapabilityEntry;
  session_resume: CapabilityEntry;
  emits_usage: CapabilityEntry;
}

export interface AdapterInfo {
  key: string;
  tier: AdapterTier;
  status: DetectStatus;
  version: string | null;
  confidence: CapabilityConfidence;
  capabilities: CapabilitySet;
}

// ---------------------------------------------------------------------------
// Preflight / Doctor
// ---------------------------------------------------------------------------

export interface DiagnosticCheck {
  name: string;
  description: string;
  status: CheckStatus;
  evidence: string | null;
}

export interface PreflightResult {
  systemReady: boolean;
  allTier1Ready: boolean;
  passedCount: number;
  failedCount: number;
  totalCount: number;
  healthScore: number;
  checks: DiagnosticCheck[];
  adapters: AdapterInfo[];
  warnings: string[];
}

// ---------------------------------------------------------------------------
// Race
// ---------------------------------------------------------------------------

export interface RaceRequest {
  taskPrompt: string;
  agents: string[];
  allowExperimental: boolean;
}

export interface RaceStarted {
  runId: string;
  agents: string[];
}

export interface AgentStreamEvent {
  runId: string;
  agentKey: string;
  eventType: string;
  data: unknown;
  timestamp: string;
}

export interface DimensionScore {
  name: string;
  score: number;
  evidence: unknown;
}

export interface AgentResult {
  agentKey: string;
  status: string;
  durationMs: number | null;
  score: number | null;
  mergeable: boolean | null;
  gateFailures: string[];
  dimensions: DimensionScore[];
}

export interface RaceResult {
  runId: string;
  status: string;
  agents: AgentResult[];
  durationMs: number | null;
  totalCost: number | null;
}

export interface RaceEventBatch {
  runId: string;
  events: AgentStreamEvent[];
  nextCursor: number;
  done: boolean;
  status: string;
  error: string | null;
}

export interface WorkingTreeStatus {
  clean: boolean;
  message: string | null;
}

// ---------------------------------------------------------------------------
// Diff / Merge (P3-UI-05)
// ---------------------------------------------------------------------------

export interface DiffFile {
  path: string;
  added: number;
  removed: number;
}

export interface CandidateDiffPayload {
  runId: string;
  agentKey: string;
  baseRef: string;
  branch: string | null;
  mergeable: boolean | null;
  gateFailures: string[];
  diffText: string;
  files: DiffFile[];
  diffAvailable: boolean;
  source: 'artifact' | 'git' | 'none';
  warning: string | null;
}

export interface MergePreviewPayload {
  agentKey: string;
  branch: string;
  success: boolean;
  hasConflicts: boolean;
  stdout: string;
  stderr: string;
  reportPath: string | null;
}

export interface MergeExecutionPayload {
  agentKey: string;
  branch: string;
  success: boolean;
  message: string;
  stdout: string | null;
  stderr: string | null;
}

// ---------------------------------------------------------------------------
// IPC Error
// ---------------------------------------------------------------------------

export interface IpcError {
  code: string;
  message: string;
  details: string | null;
}

// ---------------------------------------------------------------------------
// Interactive Session (M4.2)
// ---------------------------------------------------------------------------

export interface InteractiveSessionRequest {
  agentKey: string;
  taskPrompt: string;
  allowExperimental: boolean;
  unsafeMode: boolean;
  cwd: string | null;
  cols: number | null;
  rows: number | null;
}

export interface InteractiveSessionStarted {
  sessionId: string;
  agentKey: string;
  status: string;
  startedAt: string;
}

export interface InteractiveStreamEvent {
  sessionId: string;
  agentKey: string;
  eventType: string;
  data: unknown;
  timestamp: string;
}

export interface InteractiveEventBatch {
  sessionId: string;
  events: InteractiveStreamEvent[];
  nextCursor: number;
  done: boolean;
  status: string;
  error: string | null;
}

export interface InteractiveWriteAck {
  sessionId: string;
  success: boolean;
  error: string | null;
}

export interface InteractiveResizeAck {
  sessionId: string;
  success: boolean;
  cols: number;
  rows: number;
  error: string | null;
}

export interface InteractiveStopResult {
  sessionId: string;
  status: string;
  wasRunning: boolean;
}

export interface InteractiveSessionSummary {
  sessionId: string;
  agentKey: string;
  status: string;
  startedAt: string;
  eventCount: number;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

export function isAdapterAvailable(status: DetectStatus): boolean {
  return status === 'ready' || status === 'experimental_ready';
}

export function isTier1(adapter: AdapterInfo): boolean {
  return adapter.tier === 'tier1';
}

export function isExperimental(adapter: AdapterInfo): boolean {
  return adapter.tier === 'experimental';
}

export function statusToVariant(status: CheckStatus): 'success' | 'danger' | 'warning' | 'info' {
  switch (status) {
    case 'passed': return 'success';
    case 'failed': return 'danger';
    case 'warning': return 'warning';
    case 'running': return 'info';
  }
}
