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

// ---------------------------------------------------------------------------
// IPC Error
// ---------------------------------------------------------------------------

export interface IpcError {
  code: string;
  message: string;
  details: string | null;
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
