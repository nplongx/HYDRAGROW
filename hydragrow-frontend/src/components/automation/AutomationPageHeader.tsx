import { Plus } from "lucide-react";

interface AutomationPageHeaderProps {
  onNewFlow: () => void;
  onOpenConfigExplorer?: () => void;
}

export function AutomationPageHeader({ onNewFlow, onOpenConfigExplorer }: AutomationPageHeaderProps) {
  return (
    <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4 mb-6">
      <div>
        <h1 className="text-2xl font-bold text-emerald-950">Tự động hóa</h1>
        <p className="text-xs text-emerald-800/70 mt-1 max-w-2xl">
          Flow & Cấu hình thiết bị — Quản lý các Flow tự động, theo dõi trạng thái và ghi đè cấu hình theo điều kiện.
        </p>
      </div>

      <div className="flex items-center gap-3 self-start sm:self-auto">
        {onOpenConfigExplorer && (
          <button
            type="button"
            onClick={onOpenConfigExplorer}
            className="inline-flex items-center justify-center rounded-xl border border-emerald-200 bg-white px-4 py-2 text-xs font-semibold text-emerald-900 shadow-sm hover:bg-emerald-50 transition-colors cursor-pointer"
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
          <span>+ Flow mới</span>
        </button>
      </div>
    </div>
  );
}
