interface Props {
  onAddNode: (
    type: "condition" | "condition_group" | "action",
    variant?: string,
  ) => void;
  onUpdateTrigger?: (type: "sensor" | "fsm" | "cron" | "webhook") => void;
}

export function NodePalette({ onAddNode, onUpdateTrigger }: Props) {
  return (
    <div className="flex flex-col gap-3 p-3 border-b border-emerald-100 bg-white">
      <div className="flex flex-col gap-2">
        <h3 className="text-xs font-bold text-gray-500 flex items-center gap-1">
          TRIGGER
        </h3>
        <div className="flex flex-wrap gap-2">
          <button
            onClick={() => onUpdateTrigger?.("sensor")}
            className="palette-btn bg-blue-50 text-blue-700 border-blue-200"
          >
            + Sensor
          </button>
          <button
            onClick={() => onUpdateTrigger?.("fsm")}
            className="palette-btn bg-blue-50 text-blue-700 border-blue-200"
          >
            + FSM giai đoạn
          </button>
          <button
            onClick={() => onUpdateTrigger?.("cron")}
            className="palette-btn bg-purple-50 text-purple-700 border-purple-200"
          >
            + Cron (lịch){" "}
            <span className="ml-1 bg-purple-500 text-white text-[9px] px-1 rounded">
              Mới
            </span>
          </button>
          <button
            onClick={() => onUpdateTrigger?.("webhook")}
            className="palette-btn bg-indigo-50 text-indigo-700 border-indigo-200"
          >
            + Webhook{" "}
            <span className="ml-1 bg-indigo-500 text-white text-[9px] px-1 rounded">
              Mới
            </span>
          </button>
        </div>
      </div>

      <div className="flex flex-col gap-2">
        <h3 className="text-xs font-bold text-gray-500 flex items-center gap-1">
          CONDITION
        </h3>
        <div className="flex flex-wrap gap-2">
          <button
            onClick={() => onAddNode("condition")}
            className="palette-btn bg-amber-50 text-amber-800 border-amber-200"
          >
            + Condition
          </button>
          <button
            onClick={() => onAddNode("condition_group")}
            className="palette-btn bg-amber-50 text-amber-800 border-amber-200"
          >
            + Condition Group (AND/OR)
          </button>
          <button
            onClick={() => onAddNode("condition", "time-window")}
            className="palette-btn bg-amber-50 text-amber-800 border-amber-200"
          >
            + Time-window (mean/min/max)
          </button>
        </div>
      </div>

      <div className="flex flex-col gap-2">
        <h3 className="text-xs font-bold text-gray-500 flex items-center gap-1">
          DELAY
        </h3>
        <div className="flex flex-wrap gap-2">
          <button
            onClick={() => onAddNode("action", "delay")}
            className="palette-btn bg-slate-50 text-slate-700 border-slate-200"
          >
            + Delay
          </button>
        </div>
      </div>

      <div className="flex flex-col gap-2">
        <h3 className="text-xs font-bold text-gray-500 flex items-center gap-1">
          ACTION
        </h3>
        <div className="flex flex-wrap gap-2">
          <button
            onClick={() => onAddNode("action", "alert")}
            className="palette-btn bg-emerald-50 text-emerald-800 border-emerald-200"
          >
            + Alert
          </button>
          <button
            onClick={() => onAddNode("action", "control")}
            className="palette-btn bg-emerald-50 text-emerald-800 border-emerald-200"
          >
            + Dose / Water / Emergency stop
          </button>
          <button
            onClick={() => onAddNode("action", "fsm")}
            className="palette-btn bg-emerald-50 text-emerald-800 border-emerald-200"
          >
            + Advance stage / End season
          </button>
          <button
            onClick={() => onAddNode("action", "chain")}
            className="palette-btn bg-emerald-50 text-emerald-800 border-emerald-200"
          >
            + Chain — chạy Flow khác
          </button>
        </div>
      </div>

      <style>{`
        .palette-btn {
          @apply rounded-full border px-2.5 py-1 text-xs font-medium transition-colors hover:brightness-95 flex items-center;
        }
      `}</style>
    </div>
  );
}
