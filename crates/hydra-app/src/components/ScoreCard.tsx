import React from "react";
import type { AgentScore } from "../types";
import { StatusBadge } from "./StatusBadge";

interface ScoreCardProps {
  score: AgentScore;
  rank: number;
}

function formatScore(value: number | null): string {
  if (value === null) return "--";
  return value.toFixed(1);
}

export const ScoreCard: React.FC<ScoreCardProps> = ({ score, rank }) => {
  return (
    <div
      style={{
        border: "1px solid #e5e7eb",
        borderRadius: "8px",
        padding: "16px",
        marginBottom: "12px",
      }}
    >
      <div
        style={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
          marginBottom: "12px",
        }}
      >
        <div>
          <span
            style={{
              fontSize: "14px",
              color: "#6b7280",
              marginRight: "8px",
            }}
          >
            #{rank}
          </span>
          <span style={{ fontWeight: 700, fontSize: "16px" }}>
            {score.agent_key}
          </span>
        </div>
        <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
          <span style={{ fontWeight: 700, fontSize: "20px" }}>
            {score.total.toFixed(1)}
          </span>
          <StatusBadge
            status={score.mergeable ? "completed" : "failed"}
          />
        </div>
      </div>

      <div
        style={{
          display: "grid",
          gridTemplateColumns: "repeat(5, 1fr)",
          gap: "8px",
          fontSize: "13px",
        }}
      >
        {(
          [
            ["Build", score.breakdown.build],
            ["Tests", score.breakdown.tests],
            ["Lint", score.breakdown.lint],
            ["Diff", score.breakdown.diff_scope],
            ["Speed", score.breakdown.speed],
          ] as [string, number | null][]
        ).map(([label, value]) => (
          <div
            key={label}
            style={{
              textAlign: "center",
              padding: "4px",
              backgroundColor: "#f9fafb",
              borderRadius: "4px",
            }}
          >
            <div style={{ color: "#6b7280", marginBottom: "2px" }}>
              {label}
            </div>
            <div style={{ fontWeight: 600 }}>{formatScore(value)}</div>
          </div>
        ))}
      </div>

      {score.gate_failures.length > 0 && (
        <div
          style={{
            marginTop: "8px",
            padding: "6px 8px",
            backgroundColor: "#fef2f2",
            borderRadius: "4px",
            fontSize: "12px",
            color: "#991b1b",
          }}
        >
          Gate failures: {score.gate_failures.join(", ")}
        </div>
      )}
    </div>
  );
};
