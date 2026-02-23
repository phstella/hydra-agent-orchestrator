import React, { useState } from "react";
import { ScoreCard } from "../components/ScoreCard";
import { getRunManifest } from "../ipc";
import type { AgentScore, RunManifest } from "../types";

// Placeholder scores for display until scoring is wired through IPC.
function placeholderScores(manifest: RunManifest): AgentScore[] {
  return manifest.agents.map((agent) => ({
    agent_key: agent.agent_key,
    total: agent.status === "completed" ? 85.0 : 0.0,
    breakdown: {
      build: agent.status === "completed" ? 100.0 : 0.0,
      tests: agent.status === "completed" ? 80.0 : null,
      lint: agent.status === "completed" ? 90.0 : null,
      diff_scope: agent.status === "completed" ? 75.0 : null,
      speed: agent.status === "completed" ? 70.0 : null,
    },
    mergeable: agent.status === "completed",
    gate_failures:
      agent.status === "completed" ? [] : ["build_failed"],
  }));
}

export const ResultsPage: React.FC = () => {
  const [runId, setRunId] = useState("");
  const [manifest, setManifest] = useState<RunManifest | null>(null);
  const [scores, setScores] = useState<AgentScore[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const handleLoad = async () => {
    if (!runId.trim()) return;
    setLoading(true);
    setError(null);
    try {
      const m = await getRunManifest(runId);
      setManifest(m);
      setScores(placeholderScores(m));
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  return (
    <div>
      <h2 style={{ marginTop: 0 }}>Results</h2>

      <div style={{ display: "flex", gap: "8px", marginBottom: "24px" }}>
        <input
          type="text"
          value={runId}
          onChange={(e) => setRunId(e.target.value)}
          placeholder="Enter run ID..."
          style={{
            flex: 1,
            padding: "6px 12px",
            borderRadius: "4px",
            border: "1px solid #d1d5db",
            fontFamily: "monospace",
          }}
        />
        <button onClick={handleLoad} disabled={loading || !runId.trim()}>
          {loading ? "Loading..." : "Load"}
        </button>
      </div>

      {error && (
        <div
          style={{
            padding: "12px",
            backgroundColor: "#fef2f2",
            borderRadius: "6px",
            color: "#991b1b",
            marginBottom: "16px",
          }}
        >
          {error}
        </div>
      )}

      {manifest && (
        <>
          <div
            style={{
              padding: "12px",
              backgroundColor: "#f9fafb",
              borderRadius: "6px",
              marginBottom: "16px",
              fontSize: "13px",
            }}
          >
            <div>
              <strong>Run:</strong>{" "}
              <span style={{ fontFamily: "monospace" }}>
                {manifest.run_id}
              </span>
            </div>
            <div>
              <strong>Status:</strong> {manifest.status}
            </div>
            <div>
              <strong>Started:</strong> {manifest.started_at}
            </div>
            {manifest.completed_at && (
              <div>
                <strong>Completed:</strong> {manifest.completed_at}
              </div>
            )}
          </div>

          <h3>Rankings</h3>
          {scores.map((score, i) => (
            <ScoreCard key={score.agent_key} score={score} rank={i + 1} />
          ))}
        </>
      )}
    </div>
  );
};
