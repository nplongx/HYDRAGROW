interface Props {
  onAddNode: (
    type: "condition" | "condition_group" | "action" | "config",
    variant?: string,
  ) => void;
  onUpdateTrigger?: (type: "sensor" | "fsm" | "cron" | "webhook") => void;
}

export function NodePalette({ onAddNode, onUpdateTrigger }: Props) {
  return (
    <div className="flex flex-col gap-2 px-4 py-2.5 border-b border-emerald-100 bg-white/90 backdrop-blur-xs text-xs">
      <div className="flex flex-wrap items-center gap-x-6 gap-y-2">
        {/* 1. TRIGGER */}
        <div className="flex items-center gap-1.5 flex-wrap">
          <span className="text-[10px] font-bold uppercase tracking-wider text-sky-800 bg-sky-50 px-1.5 py-0.5 rounded border border-sky-200">
            TRIGGER
          </span>
          <button
            type="button"
            onClick={() => onUpdateTrigger?.("sensor")}
            className="palette-btn bg-sky-50/70 text-sky-700 border-sky-200 hover:bg-sky-100"
          >
            + Sensor
          </button>
          <button
            type="button"
            onClick={() => onUpdateTrigger?.("fsm")}
            className="palette-btn bg-sky-50/70 text-sky-700 border-sky-200 hover:bg-sky-100"
          >
            + FSM giai đoạn
          </button>
          <button
            type="button"
            onClick={() => onUpdateTrigger?.("cron")}
            className="palette-btn bg-purple-50/70 text-purple-700 border-purple-200 hover:bg-purple-100"
          >
            + Cron (lịch)
          </button>
          <button
            type="button"
            onClick={() => onUpdateTrigger?.("webhook")}
            className="palette-btn bg-indigo-50/70 text-indigo-700 border-indigo-200 hover:bg-indigo-100"
          >
            + Webhook
          </button>
        </div>

        {/* 2. CONDITION */}
        <div className="flex items-center gap-1.5 flex-wrap">
          <span className="text-[10px] font-bold uppercase tracking-wider text-amber-800 bg-amber-50 px-1.5 py-0.5 rounded border border-amber-200">
            CONDITION
          </span>
          <button
            type="button"
            onClick={() => onAddNode("condition")}
            className="palette-btn bg-amber-50/70 text-amber-800 border-amber-200 hover:bg-amber-100"
          >
            + Condition
          </button>
          <button
            type="button"
            onClick={() => onAddNode("condition_group")}
            className="palette-btn bg-amber-50/70 text-amber-800 border-amber-200 hover:bg-amber-100"
          >
            + Condition Group (AND/OR)
          </button>
          <button
            type="button"
            onClick={() => onAddNode("condition", "time-window")}
            className="palette-btn bg-amber-50/70 text-amber-800 border-amber-200 hover:bg-amber-100"
          >
            + Time-window (mean/min/max)
          </button>
        </div>

        {/* 3. DELAY */}
        <div className="flex items-center gap-1.5 flex-wrap">
          <span className="text-[10px] font-bold uppercase tracking-wider text-orange-800 bg-orange-50 px-1.5 py-0.5 rounded border border-orange-200">
            DELAY
          </span>
          <button
            type="button"
            onClick={() => onAddNode("action", "delay")}
            className="palette-btn bg-orange-50/70 text-orange-800 border-orange-200 hover:bg-orange-100"
          >
            + Delay
          </button>
        </div>

        {/* 4. CONFIG */}
        <div className="flex items-center gap-1.5 flex-wrap">
          <span className="text-[10px] font-bold uppercase tracking-wider text-indigo-800 bg-indigo-50 px-1.5 py-0.5 rounded border border-indigo-200">
            CONFIG
          </span>
          <button
            type="button"
            onClick={() => onAddNode("config", "read")}
            className="palette-btn bg-indigo-50/70 text-indigo-700 border-indigo-200 hover:bg-indigo-100"
          >
            + Đọc cấu hình
          </button>
          <button
            type="button"
            onClick={() => onAddNode("config", "overwrite")}
            className="palette-btn bg-indigo-600 text-white border-indigo-600 hover:bg-indigo-700 font-semibold shadow-2xs"
          >
            + Ghi đè cấu hình
          </button>
        </div>

        {/* 5. ACTION */}
        <div className="flex items-center gap-1.5 flex-wrap">
          <span className="text-[10px] font-bold uppercase tracking-wider text-emerald-800 bg-emerald-50 px-1.5 py-0.5 rounded border border-emerald-200">
            ACTION
          </span>
          <button
            type="button"
            onClick={() => onAddNode("action", "alert")}
            className="palette-btn bg-emerald-50/70 text-emerald-800 border-emerald-200 hover:bg-emerald-100"
          >
            + Alert
          </button>
          <button
            type="button"
            onClick={() => onAddNode("action", "control")}
            className="palette-btn bg-emerald-50/70 text-emerald-800 border-emerald-200 hover:bg-emerald-100"
          >
            + Dose / Water / Emergency stop
          </button>
          <button
            type="button"
            onClick={() => onAddNode("action", "fsm")}
            className="palette-btn bg-emerald-50/70 text-emerald-800 border-emerald-200 hover:bg-emerald-100"
          >
            + Advance stage / End season
          </button>
          <button
            type="button"
            onClick={() => onAddNode("action", "chain")}
            className="palette-btn bg-emerald-50/70 text-emerald-800 border-emerald-200 hover:bg-emerald-100"
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
