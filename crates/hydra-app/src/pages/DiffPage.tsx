import React, { useState } from "react";
import { StatusBadge } from "../components/StatusBadge";
import { mergeDryRun, mergeConfirm } from "../ipc";
import type { MergeReport } from "../types";

export const DiffPage: React.FC = () => {
  const [sourceBranch, setSourceBranch] = useState("");
  const [targetBranch, setTargetBranch] = useState("main");
  const [report, setReport] = useState<MergeReport | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const handleDryRun = async () => {
    if (!sourceBranch.trim()) return;
    setLoading(true);
    setError(null);
    try {
      const r = await mergeDryRun(sourceBranch, targetBranch);
      setReport(r);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  const handleMerge = async () => {
    if (!sourceBranch.trim()) return;
    setLoading(true);
    setError(null);
    try {
      const r = await mergeConfirm(sourceBranch, targetBranch);
      setReport(r);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  return (
    <div>
      <h2 style={{ marginTop: 0 }}>Diff &amp; Merge</h2>

      <div
        style={{
          display: "grid",
          gridTemplateColumns: "1fr 1fr",
          gap: "12px",
          marginBottom: "16px",
        }}
      >
        <div>
          <label
            htmlFor="source-branch"
            style={{ display: "block", marginBottom: "4px", fontWeight: 600 }}
          >
            Source Branch
          </label>
          <input
            id="source-branch"
            type="text"
            value={sourceBranch}
            onChange={(e) => setSourceBranch(e.target.value)}
            placeholder="e.g. hydra/run-abc/claude"
            style={{
              width: "100%",
              padding: "6px 12px",
              borderRadius: "4px",
              border: "1px solid #d1d5db",
              fontFamily: "monospace",
              boxSizing: "border-box",
            }}
          />
        </div>
        <div>
          <label
            htmlFor="target-branch"
            style={{ display: "block", marginBottom: "4px", fontWeight: 600 }}
          >
            Target Branch
          </label>
          <input
            id="target-branch"
            type="text"
            value={targetBranch}
            onChange={(e) => setTargetBranch(e.target.value)}
            style={{
              width: "100%",
              padding: "6px 12px",
              borderRadius: "4px",
              border: "1px solid #d1d5db",
              fontFamily: "monospace",
              boxSizing: "border-box",
            }}
          />
        </div>
      </div>

      <div style={{ display: "flex", gap: "8px", marginBottom: "24px" }}>
        <button
          onClick={handleDryRun}
          disabled={loading || !sourceBranch.trim()}
        >
          {loading ? "Checking..." : "Dry Run"}
        </button>
        <button
          onClick={handleMerge}
          disabled={loading || !sourceBranch.trim()}
          style={{
            backgroundColor: "#dc2626",
            color: "white",
            border: "none",
            borderRadius: "4px",
            padding: "6px 16px",
            cursor: loading ? "not-allowed" : "pointer",
          }}
        >
          Merge
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

      {report && (
        <div
          style={{
            border: "1px solid #e5e7eb",
            borderRadius: "8px",
            padding: "16px",
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
            <h3 style={{ margin: 0 }}>
              {report.dry_run ? "Dry Run" : "Merge"} Report
            </h3>
            <StatusBadge
              status={report.can_merge ? "completed" : "failed"}
            />
          </div>

          <div
            style={{
              display: "grid",
              gridTemplateColumns: "auto 1fr",
              gap: "8px",
              marginBottom: "12px",
              fontSize: "13px",
            }}
          >
            <span style={{ fontWeight: 600 }}>Source:</span>
            <span style={{ fontFamily: "monospace" }}>
              {report.source_branch}
            </span>
            <span style={{ fontWeight: 600 }}>Target:</span>
            <span style={{ fontFamily: "monospace" }}>
              {report.target_branch}
            </span>
            <span style={{ fontWeight: 600 }}>Files changed:</span>
            <span>{report.files_changed}</span>
            <span style={{ fontWeight: 600 }}>Changes:</span>
            <span>
              <span style={{ color: "#166534" }}>
                +{report.insertions}
              </span>{" "}
              <span style={{ color: "#991b1b" }}>
                -{report.deletions}
              </span>
            </span>
          </div>

          {report.conflicts.length > 0 && (
            <div>
              <h4 style={{ marginBottom: "8px" }}>
                Conflicts ({report.conflicts.length})
              </h4>
              <ul
                style={{
                  margin: 0,
                  padding: "0 0 0 20px",
                  fontFamily: "monospace",
                  fontSize: "13px",
                }}
              >
                {report.conflicts.map((c) => (
                  <li key={c.path}>
                    {c.path}{" "}
                    <span style={{ color: "#6b7280" }}>
                      ({c.conflict_type})
                    </span>
                  </li>
                ))}
              </ul>
            </div>
          )}
        </div>
      )}
    </div>
  );
};
