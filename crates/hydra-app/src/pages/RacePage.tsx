import React, { useState } from "react";
import { StatusBadge } from "../components/StatusBadge";
import { startRace } from "../ipc";
import type { RaceResult } from "../types";

export const RacePage: React.FC = () => {
  const [agent, setAgent] = useState("claude");
  const [prompt, setPrompt] = useState("");
  const [running, setRunning] = useState(false);
  const [result, setResult] = useState<RaceResult | null>(null);
  const [error, setError] = useState<string | null>(null);

  const handleStart = async () => {
    if (!prompt.trim()) return;
    setRunning(true);
    setError(null);
    setResult(null);
    try {
      const r = await startRace(agent, prompt);
      setResult(r);
    } catch (e) {
      setError(String(e));
    } finally {
      setRunning(false);
    }
  };

  return (
    <div>
      <h2 style={{ marginTop: 0 }}>Start Race</h2>

      <div style={{ marginBottom: "16px" }}>
        <label
          htmlFor="agent-select"
          style={{ display: "block", marginBottom: "4px", fontWeight: 600 }}
        >
          Agent
        </label>
        <select
          id="agent-select"
          value={agent}
          onChange={(e) => setAgent(e.target.value)}
          style={{ padding: "6px 12px", borderRadius: "4px", border: "1px solid #d1d5db" }}
        >
          <option value="claude">Claude (Tier 1)</option>
          <option value="codex">Codex (Tier 1)</option>
          <option value="cursor-agent">Cursor (Experimental)</option>
        </select>
      </div>

      <div style={{ marginBottom: "16px" }}>
        <label
          htmlFor="prompt-input"
          style={{ display: "block", marginBottom: "4px", fontWeight: 600 }}
        >
          Task Prompt
        </label>
        <textarea
          id="prompt-input"
          value={prompt}
          onChange={(e) => setPrompt(e.target.value)}
          placeholder="Describe the task for the agent..."
          rows={4}
          style={{
            width: "100%",
            padding: "8px 12px",
            borderRadius: "4px",
            border: "1px solid #d1d5db",
            fontFamily: "inherit",
            resize: "vertical",
            boxSizing: "border-box",
          }}
        />
      </div>

      <button
        onClick={handleStart}
        disabled={running || !prompt.trim()}
        style={{
          padding: "8px 24px",
          backgroundColor: running ? "#9ca3af" : "#2563eb",
          color: "white",
          border: "none",
          borderRadius: "6px",
          cursor: running ? "not-allowed" : "pointer",
          fontWeight: 600,
        }}
      >
        {running ? "Running..." : "Start Race"}
      </button>

      {error && (
        <div
          style={{
            marginTop: "16px",
            padding: "12px",
            backgroundColor: "#fef2f2",
            borderRadius: "6px",
            color: "#991b1b",
          }}
        >
          {error}
        </div>
      )}

      {result && (
        <div
          style={{
            marginTop: "16px",
            padding: "16px",
            border: "1px solid #e5e7eb",
            borderRadius: "8px",
          }}
        >
          <h3 style={{ marginTop: 0 }}>Race Result</h3>
          <div
            style={{
              display: "grid",
              gridTemplateColumns: "auto 1fr",
              gap: "8px",
            }}
          >
            <span style={{ fontWeight: 600 }}>Run ID:</span>
            <span style={{ fontFamily: "monospace", fontSize: "13px" }}>
              {result.run_id}
            </span>
            <span style={{ fontWeight: 600 }}>Agent:</span>
            <span>{result.agent_key}</span>
            <span style={{ fontWeight: 600 }}>Status:</span>
            <StatusBadge status={result.status.toLowerCase()} />
            <span style={{ fontWeight: 600 }}>Artifacts:</span>
            <span style={{ fontFamily: "monospace", fontSize: "13px" }}>
              {result.artifact_dir}
            </span>
          </div>
        </div>
      )}
    </div>
  );
};
