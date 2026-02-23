import React, { useState } from "react";
import { DiffPage } from "./pages/DiffPage";
import { DoctorPage } from "./pages/DoctorPage";
import { RacePage } from "./pages/RacePage";
import { ResultsPage } from "./pages/ResultsPage";

type Page = "doctor" | "race" | "results" | "diff";

const NAV_ITEMS: { key: Page; label: string }[] = [
  { key: "doctor", label: "Health" },
  { key: "race", label: "Race" },
  { key: "results", label: "Results" },
  { key: "diff", label: "Diff & Merge" },
];

export const App: React.FC = () => {
  const [page, setPage] = useState<Page>("doctor");

  return (
    <div style={{ display: "flex", minHeight: "100vh" }}>
      {/* Sidebar */}
      <nav
        style={{
          width: "200px",
          backgroundColor: "#111827",
          color: "#f9fafb",
          padding: "16px 0",
          flexShrink: 0,
        }}
      >
        <div
          style={{
            padding: "0 16px 16px",
            borderBottom: "1px solid #374151",
            marginBottom: "8px",
          }}
        >
          <h1
            style={{
              fontSize: "18px",
              fontWeight: 700,
              margin: 0,
              letterSpacing: "0.05em",
            }}
          >
            HYDRA
          </h1>
          <div style={{ fontSize: "11px", color: "#9ca3af", marginTop: "2px" }}>
            Agent Orchestrator
          </div>
        </div>
        {NAV_ITEMS.map((item) => (
          <button
            key={item.key}
            onClick={() => setPage(item.key)}
            style={{
              display: "block",
              width: "100%",
              padding: "8px 16px",
              background:
                page === item.key ? "#1f2937" : "transparent",
              color: page === item.key ? "#60a5fa" : "#d1d5db",
              border: "none",
              textAlign: "left",
              cursor: "pointer",
              fontSize: "14px",
              fontWeight: page === item.key ? 600 : 400,
              borderLeft:
                page === item.key
                  ? "3px solid #2563eb"
                  : "3px solid transparent",
            }}
          >
            {item.label}
          </button>
        ))}
      </nav>

      {/* Main content */}
      <main
        style={{
          flex: 1,
          padding: "24px 32px",
          backgroundColor: "#ffffff",
          overflow: "auto",
        }}
      >
        {page === "doctor" && <DoctorPage />}
        {page === "race" && <RacePage />}
        {page === "results" && <ResultsPage />}
        {page === "diff" && <DiffPage />}
      </main>
    </div>
  );
};

export default App;
