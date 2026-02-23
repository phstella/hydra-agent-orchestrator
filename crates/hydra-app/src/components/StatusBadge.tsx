import React from "react";

interface StatusBadgeProps {
  status: string;
}

const STATUS_COLORS: Record<string, { bg: string; text: string }> = {
  running: { bg: "#dbeafe", text: "#1d4ed8" },
  completed: { bg: "#dcfce7", text: "#166534" },
  failed: { bg: "#fee2e2", text: "#991b1b" },
  timed_out: { bg: "#fef3c7", text: "#92400e" },
  cancelled: { bg: "#f3f4f6", text: "#374151" },
  ready: { bg: "#dcfce7", text: "#166534" },
  missing: { bg: "#fee2e2", text: "#991b1b" },
  blocked: { bg: "#fef3c7", text: "#92400e" },
  experimental_ready: { bg: "#e0e7ff", text: "#3730a3" },
};

export const StatusBadge: React.FC<StatusBadgeProps> = ({ status }) => {
  const colors = STATUS_COLORS[status] ?? { bg: "#f3f4f6", text: "#374151" };
  return (
    <span
      style={{
        display: "inline-block",
        padding: "2px 8px",
        borderRadius: "4px",
        fontSize: "12px",
        fontWeight: 600,
        backgroundColor: colors.bg,
        color: colors.text,
      }}
    >
      {status.replace(/_/g, " ")}
    </span>
  );
};
