interface Props {
  onAddNode: (
    type: "condition" | "condition_group" | "action" | "config",
    variant?: string,
  ) => void;
  onUpdateTrigger?: (type: "sensor" | "fsm" | "cron" | "webhook") => void;
}

export function NodePalette({ onAddNode, onUpdateTrigger }: Props) {
  return (
    <div className="flex flex-col gap-2 px-4 py-2.5 border-b border-line bg-white/90 backdrop-blur-xs text-xs">
      <div className="flex flex-wrap items-center gap-x-6 gap-y-2">
        {/* 1. TRIGGER */}
        <div className="flex items-center gap-1.5 flex-wrap">
          <span className="text-[10px] font-bold uppercase tracking-wider text-info bg-info-bg px-1.5 py-0.5 rounded border border-info">
            TRIGGER
          </span>
          <button
            type="button"
            onClick={() => onUpdateTrigger?.("sensor")}
            className="palette-btn bg-info-bg/70 text-info border-info hover:bg-info-bg"
          >
            + Sensor
          </button>
          <button
            type="button"
            onClick={() => onUpdateTrigger?.("fsm")}
            className="palette-btn bg-info-bg/70 text-info border-info hover:bg-info-bg"
          >
            + FSM giai đoạn
          </button>
          <button
            type="button"
            onClick={() => onUpdateTrigger?.("cron")}
            className="palette-btn bg-config-soft text-config border-info hover:bg-config-soft"
          >
            + Cron (lịch)
          </button>
          <button
            type="button"
            onClick={() => onUpdateTrigger?.("webhook")}
            className="palette-btn bg-config-soft/70 text-config border-info hover:bg-config-soft"
          >
            + Webhook
          </button>
        </div>

        {/* 2. CONDITION */}
        <div className="flex items-center gap-1.5 flex-wrap">
          <span className="text-[10px] font-bold uppercase tracking-wider text-warning bg-warning-bg px-1.5 py-0.5 rounded border border-warning">
            CONDITION
          </span>
          <button
            type="button"
            onClick={() => onAddNode("condition")}
            className="palette-btn bg-warning-bg/70 text-warning border-warning hover:bg-warning-bg"
          >
            + Condition
          </button>
          <button
            type="button"
            onClick={() => onAddNode("condition_group")}
            className="palette-btn bg-warning-bg/70 text-warning border-warning hover:bg-warning-bg"
          >
            + Condition Group (AND/OR)
          </button>
          <button
            type="button"
            onClick={() => onAddNode("condition", "time-window")}
            className="palette-btn bg-warning-bg/70 text-warning border-warning hover:bg-warning-bg"
          >
            + Time-window (mean/min/max)
          </button>
        </div>


        {/* 4. CONFIG */}
        <div className="flex items-center gap-1.5 flex-wrap">
          <span className="text-[10px] font-bold uppercase tracking-wider text-config bg-config-soft px-1.5 py-0.5 rounded border border-info">
            CONFIG
          </span>
          <button
            type="button"
            onClick={() => onAddNode("config", "read")}
            className="palette-btn bg-config-soft/70 text-config border-info hover:bg-config-soft"
          >
            + Đọc cấu hình
          </button>
          <button
            type="button"
            onClick={() => onAddNode("config", "overwrite")}
            className="palette-btn bg-config text-white border-config hover:bg-config font-semibold shadow-2xs"
          >
            + Ghi đè cấu hình
          </button>
        </div>

        {/* 5. ACTION */}
        <div className="flex items-center gap-1.5 flex-wrap">
          <span className="text-[10px] font-bold uppercase tracking-wider text-text-muted bg-pill px-1.5 py-0.5 rounded border border-line">
            ACTION
          </span>
          <button
            type="button"
            onClick={() => onAddNode("action", "alert")}
            className="palette-btn bg-pill/70 text-text-muted border-line hover:bg-pill"
          >
            + Alert
          </button>
          <button
            type="button"
            onClick={() => onAddNode("action", "control")}
            className="palette-btn bg-pill/70 text-text-muted border-line hover:bg-pill"
          >
            + Dose / Water / Emergency stop
          </button>
          <button
            type="button"
            onClick={() => onAddNode("action", "fsm")}
            className="palette-btn bg-pill/70 text-text-muted border-line hover:bg-pill"
          >
            + Advance stage / End season
          </button>
          <button
            type="button"
            onClick={() => onAddNode("action", "chain")}
            className="palette-btn bg-pill/70 text-text-muted border-line hover:bg-pill"
          >
            + Chain — chạy Flow khác
          </button>
        </div>
      </div>

      <style>{`
        .palette-btn {
          @apply rounded-lg border px-2 py-0.5 text-xs font-medium transition-all hover:brightness-95 flex items-center cursor-pointer;
        }
      `}</style>
    </div>
  );
}
