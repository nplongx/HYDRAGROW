import { Plus } from "lucide-react";

interface AutomationPageHeaderProps {
  onNewFlow: () => void;
  onOpenConfigExplorer?: () => void;
  stationName?: string;
  stationId?: string;
}

export function AutomationPageHeader({ onNewFlow, onOpenConfigExplorer, stationName, stationId }: AutomationPageHeaderProps) {
  return (
    <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4 mb-6">
      <div>
        <h1 className="text-2xl font-bold text-primary-deep">Tự động hóa</h1>
        <p className="text-xs text-text-muted mt-1 max-w-2xl">
          Rule theo trạm — định nghĩa, validate, mô phỏng và quản lý lifecycle automation.
        </p>
        {stationId && (
          <div className="mt-2 inline-flex max-w-full items-center gap-2 rounded-full border border-line bg-white px-3 py-1 text-[11px]">
            <span className="font-semibold text-primary-deep">Trạm</span>
            <span className="truncate text-text-muted">{stationName || stationId}</span>
            <span className="font-mono text-faint">{stationId}</span>
          </div>
        )}
      </div>

      <div className="flex items-center gap-3 self-start sm:self-auto">
        {onOpenConfigExplorer && (
          <button
            type="button"
            onClick={onOpenConfigExplorer}
            className="inline-flex items-center justify-center rounded-xl border border-line bg-white px-4 py-2 text-xs font-semibold text-primary-deep shadow-sm hover:bg-soft transition-colors cursor-pointer"
          >
            Config Explorer
          </button>
        )}
        <button
          type="button"
          onClick={onNewFlow}
          className="ui-btn-primary flex items-center gap-1.5 text-xs font-semibold px-4 py-2 rounded-xl"
        >
          <Plus className="w-4 h-4" />
          <span>+ Tạo Automation (Flow mới)</span>
        </button>
      </div>
    </div>
  );
}
