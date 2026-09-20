import React from "react";
import { LucideIcon } from "lucide-react";

interface SensorBentoCardProps {
  title: string;
  value: number | string | null;
  unit?: string;
  icon: LucideIcon | React.ElementType;
  theme: "blue" | "fuchsia" | "orange" | "cyan" | "rose" | "emerald";
  statusLabel?: string;
  statusTone?: "good" | "warn" | "danger" | "info";
  rangeLabel?: string;
  description?: string;
  compact?: boolean;
  sparkline?: number;
  observedAt?: string | null;
  quality?: string;
}

const themeClasses: Record<string, string> = {
  blue: "text-status-info bg-status-info-bg border-border-info/20",
  fuchsia: "text-config bg-config-soft border-config/20",
  orange: "text-status-warning bg-status-warning-bg border-border-warning/20",
  cyan: "text-status-info bg-status-info-bg border-border-info/20",
  rose: "text-status-fault bg-status-fault-bg border-border-fault/20",
  emerald: "text-status-success bg-status-success-bg border-line",
};

const statusClasses: Record<string, string> = {
  good: "bg-pill text-status border-transparent",
  warn: "bg-warning-bg text-warn-deep border-transparent",
  danger: "bg-danger-bg text-error border-transparent",
  info: "bg-status-info-bg text-status-info border-transparent",
};

const sparkColor: Record<string, string> = {
  good: "bg-status",
  warn: "bg-warn-deep",
  danger: "bg-error",
  info: "bg-primary/50",
};

const clampPercent = (value: number) => Math.min(100, Math.max(0, value));

export const SensorBentoCard: React.FC<SensorBentoCardProps> = ({
  title,
  value,
  unit,
  icon: Icon,
  theme,
  statusLabel,
  statusTone = "info",
  rangeLabel,
  description,
  compact = false,
  sparkline,
  observedAt,
  quality,
}) => (
  <article
    aria-label={title}
    className={`bg-surface border rounded-2xl flex flex-col justify-between transition-[border-color,box-shadow,background-color] hover:border-primary/40 hover:shadow-medium shadow-low ${compact ? "p-3.5 min-h-[140px]" : "p-4 md:p-5 min-h-[176px]"} ${statusTone === "danger" ? "border-error/40 bg-danger-bg/30" : statusTone === "warn" ? "border-warning/40 bg-warning-bg/30" : "border-line"}`}
  >
    <div className="flex items-start justify-between gap-2">
      <div className="flex items-center gap-2.5">
        <div
          className={`p-2 rounded-xl border ${themeClasses[theme]} shrink-0`}
        >
          <Icon size={compact ? 15 : 17} strokeWidth={2.5} />
        </div>
        <span
          className={`font-semibold text-primary-deep ${compact ? "text-xs" : "text-sm"}`}
        >
          {title}
        </span>
      </div>
      {statusLabel && (
        <span
          className={`shrink-0 rounded-full border px-2 py-0.5 text-[10px] font-bold ${statusClasses[statusTone]}`}
        >
          {statusLabel}
        </span>
      )}
    </div>
    <div className={`space-y-1 ${compact ? "mt-3" : "mt-4"}`}>
      <div className="flex items-baseline gap-1">
        <span
          className={`font-black text-primary-deep ${compact ? "text-2xl" : "text-3xl"}`}
        >
          {value ?? "--"}
        </span>
        {unit && (
          <span
            className={`font-semibold text-primary/70 ${compact ? "text-xs" : "text-sm"}`}
          >
            {unit}
          </span>
        )}
      </div>
      {rangeLabel && (
        <p className="text-[11px] font-medium text-faint">{rangeLabel}</p>
      )}
      {description && (
        <p className="text-xs text-faint/80 leading-relaxed">{description}</p>
      )}
      {(observedAt || quality) && (
        <p className="text-[10px] text-faint" data-testid={`sensor-meta-${title}`}>
          {quality ? `Chất lượng: ${quality}` : ''}{quality && observedAt ? ' · ' : ''}{observedAt ? `Quan sát: ${new Date(observedAt).toLocaleString('vi-VN')}` : ''}
        </p>
      )}
    </div>
    {sparkline != null && (
      <div className="mt-3">
        <div className="h-1.5 w-full overflow-hidden rounded-full bg-line">
          <div
            className={`h-full rounded-full transition-all duration-500 ${sparkColor[statusTone] ?? sparkColor.info}`}
            style={{ width: `${clampPercent(sparkline)}%` }}
          />
        </div>
      </div>
    )}
  </article>
);
