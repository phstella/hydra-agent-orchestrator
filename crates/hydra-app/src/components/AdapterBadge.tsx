import React from "react";
import type { ProbeResult } from "../types";
import { StatusBadge } from "./StatusBadge";

interface AdapterBadgeProps {
  adapter: ProbeResult;
}

export const AdapterBadge: React.FC<AdapterBadgeProps> = ({ adapter }) => {
  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        gap: "8px",
        padding: "8px 12px",
        border: "1px solid #e5e7eb",
        borderRadius: "6px",
      }}
    >
      <span style={{ fontWeight: 600 }}>{adapter.adapter_key}</span>
      <StatusBadge status={adapter.status} />
      {adapter.tier === "experimental" && (
        <span
          style={{
            fontSize: "11px",
            color: "#6b7280",
            fontStyle: "italic",
          }}
        >
          (experimental)
        </span>
      )}
      {adapter.version && (
        <span style={{ fontSize: "12px", color: "#6b7280" }}>
          v{adapter.version}
        </span>
      )}
    </div>
  );
};
