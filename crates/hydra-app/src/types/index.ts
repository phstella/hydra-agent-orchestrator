// TypeScript types matching hydra-core Rust types for IPC.

// -- Race --

export interface RaceResult {
  run_id: string;
  agent_key: string;
  status: string;
  artifact_dir: string;
}

// -- Doctor --

export interface DoctorReport {
  git: GitCheck;
  adapters: ProbeReport;
  overall_ready: boolean;
}

export interface GitCheck {
  git_available: boolean;
  git_version: string | null;
  in_git_repo: boolean;
  repo_root: string | null;
}

export interface ProbeReport {
  adapters: ProbeResult[];
  tier1_ready: boolean;
}

export interface ProbeResult {
  adapter_key: string;
  tier: "tier1" | "experimental";
  status: "ready" | "missing" | "blocked" | "experimental_ready";
  binary_path: string | null;
  version: string | null;
  capabilities: CapabilitySet;
  confidence: "high" | "medium" | "low" | "unknown";
  message: string | null;
}

export interface CapabilitySet {
  print_mode: boolean;
  json_output: boolean;
  streaming: boolean;
  resume: boolean;
  sandbox: boolean;
}

// -- Artifact --

export interface RunManifest {
  schema_version: string;
  run_id: string;
  repo_root: string;
  base_ref: string;
  task_prompt_hash: string;
  started_at: string;
  completed_at: string | null;
  status: RunStatus;
  agents: AgentManifest[];
}

export type RunStatus =
  | "running"
  | "completed"
  | "failed"
  | "cancelled"
  | "timed_out";

export interface AgentManifest {
  agent_key: string;
  adapter_version: string | null;
  worktree_path: string;
  branch: string;
  started_at: string;
  completed_at: string | null;
  status: AgentStatus;
  token_usage: TokenUsage | null;
  cost_estimate_usd: number | null;
}

export type AgentStatus =
  | "running"
  | "completed"
  | "failed"
  | "timed_out"
  | "cancelled";

export interface TokenUsage {
  input_tokens: number;
  output_tokens: number;
}

// -- Events --

export interface RunEvent {
  timestamp: string;
  run_id: string;
  event_type: string;
  agent_key: string | null;
  data: unknown;
}

// -- Merge --

export interface MergeReport {
  source_branch: string;
  target_branch: string;
  dry_run: boolean;
  can_merge: boolean;
  conflicts: ConflictFile[];
  files_changed: number;
  insertions: number;
  deletions: number;
}

export interface ConflictFile {
  path: string;
  conflict_type: string;
}

// -- Scoring --

export interface AgentScore {
  agent_key: string;
  total: number;
  breakdown: ScoreBreakdown;
  mergeable: boolean;
  gate_failures: string[];
}

export interface ScoreBreakdown {
  build: number | null;
  tests: number | null;
  lint: number | null;
  diff_scope: number | null;
  speed: number | null;
}

export interface RankingResult {
  run_id: string;
  rankings: AgentScore[];
}

// -- Config --

export interface HydraConfig {
  general: GeneralConfig;
  scoring: ScoringConfig;
  adapters: AdaptersConfig;
  retention: RetentionConfig;
  budget: BudgetConfig;
}

export interface GeneralConfig {
  default_timeout_seconds: number;
  hard_timeout_seconds: number;
  idle_timeout_seconds: number;
  allow_experimental_adapters: boolean;
  unsafe_mode: boolean;
}

export interface ScoringConfig {
  profile: string;
  timeout_per_check_seconds: number;
  weights: ScoringWeights;
  gates: ScoringGates;
}

export interface ScoringWeights {
  build: number;
  tests: number;
  lint: number;
  diff_scope: number;
  speed: number;
}

export interface ScoringGates {
  require_build_pass: boolean;
  max_test_regression_percent: number;
}

export interface AdaptersConfig {
  claude: AdapterConfig;
  codex: AdapterConfig;
  cursor: AdapterConfig;
}

export interface AdapterConfig {
  enabled: boolean | null;
  binary_path: string | null;
  extra_args: string[];
}

export interface RetentionConfig {
  policy: string;
  max_age_days: number | null;
}

export interface BudgetConfig {
  max_tokens_total: number | null;
  max_cost_usd: number | null;
}
