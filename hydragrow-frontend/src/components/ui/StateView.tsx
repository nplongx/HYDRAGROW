import React from "react";
import { LucideIcon } from "lucide-react";

interface StateViewProps {
  icon: LucideIcon | React.ElementType;
  title: string;
  description?: string;
  action?: React.ReactNode;
  className?: string;
  tone?: "neutral" | "info" | "warning" | "danger";
}

const TONE_STYLES: Record<
  NonNullable<StateViewProps["tone"]>,
  { icon: string; panel: string }
> = {
  neutral: { icon: "bg-pill text-primary", panel: "" },
  info: { icon: "bg-info-bg text-info-fg", panel: "" },
  warning: { icon: "bg-warning-bg text-warn-deep", panel: "border-warning/40" },
  danger: { icon: "bg-danger-bg text-error", panel: "border-error/40" },
};

export const StateView: React.FC<StateViewProps> = ({
  icon: Icon,
  title,
  description,
  action,
  className = "",
  tone = "neutral",
}) => (
  <div className={`ui-state ${TONE_STYLES[tone].panel} ${className}`}>
    <div className={`ui-state-icon ${TONE_STYLES[tone].icon}`}>
      <Icon size={32} strokeWidth={1.5} />
    </div>
    <div className="space-y-1">
      <h3 className="ui-state-title">{title}</h3>
      {description && <p className="ui-state-desc">{description}</p>}
    </div>
    {action && <div>{action}</div>}
  </div>
);
