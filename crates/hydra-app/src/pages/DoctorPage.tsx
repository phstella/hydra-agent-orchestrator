import React, { useCallback, useEffect, useState } from "react";
import { AdapterBadge } from "../components/AdapterBadge";
import { StatusBadge } from "../components/StatusBadge";
import { getDoctorReport } from "../ipc";
import type { DoctorReport } from "../types";

export const DoctorPage: React.FC = () => {
  const [report, setReport] = useState<DoctorReport | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const runCheck = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const r = await getDoctorReport();
      setReport(r);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    runCheck();
  }, [runCheck]);

  return (
    <div>
      <div
        style={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
          marginBottom: "24px",
        }}
      >
        <h2 style={{ margin: 0 }}>System Health</h2>
        <button onClick={runCheck} disabled={loading}>
          {loading ? "Checking..." : "Re-check"}
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
        <>
          <div
            style={{
              padding: "12px",
              backgroundColor: report.overall_ready ? "#dcfce7" : "#fef2f2",
              borderRadius: "6px",
              marginBottom: "24px",
              fontWeight: 600,
            }}
          >
            Overall: {report.overall_ready ? "Ready" : "Not Ready"}
          </div>

          <h3>Git</h3>
          <div
            style={{
              display: "grid",
              gridTemplateColumns: "auto 1fr",
              gap: "8px",
              marginBottom: "24px",
            }}
          >
            <span>Available:</span>
            <StatusBadge
              status={report.git.git_available ? "ready" : "missing"}
            />
            <span>Version:</span>
            <span>{report.git.git_version ?? "N/A"}</span>
            <span>In Repo:</span>
            <StatusBadge
              status={report.git.in_git_repo ? "ready" : "missing"}
            />
            <span>Root:</span>
            <span style={{ fontFamily: "monospace", fontSize: "13px" }}>
              {report.git.repo_root ?? "N/A"}
            </span>
          </div>

          <h3>Adapters</h3>
          <div
            style={{
              display: "flex",
              flexDirection: "column",
              gap: "8px",
            }}
          >
            {report.adapters.adapters.map((a) => (
              <AdapterBadge key={a.adapter_key} adapter={a} />
            ))}
          </div>
        </>
      )}
    </div>
  );
};
