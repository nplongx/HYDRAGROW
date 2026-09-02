// src/components/logs/HealthSummaryBar.tsx
import { Search, ShieldAlert, FlaskConical, Waves, AlertTriangle } from 'lucide-react';
import { Switch } from '../ui/Switch';

export interface SystemHealthSummary {
  window_seconds?: number;
  ec_dosing_count?: number;
  ph_dosing_count?: number;
  water_operation_count?: number;
  warning_count?: number;
  critical_count?: number;
  latest_ph_dosing_at?: string | null;
}

export type LogViewMode = 'important' | 'all_technical';

interface HealthSummaryBarProps {
  summary?: SystemHealthSummary;
  mode: LogViewMode;
  onModeChange: (mode: LogViewMode) => void;
  search: string;
  onSearchChange: (value: string) => void;
}

export const HealthSummaryBar = ({ summary, mode, onModeChange, search, onSearchChange }: HealthSummaryBarProps) => {
  return (
    <div className="ui-card space-y-4">
      <div className="grid grid-cols-2 md:grid-cols-4 gap-3 text-xs">
        <div className="farm-muted-panel flex items-center gap-2">
          <FlaskConical size={14} className="text-cyan-700 shrink-0" />
          <span className="text-emerald-900 font-semibold">{summary?.ec_dosing_count ?? 0} lần châm EC</span>
        </div>
        <div className="farm-muted-panel flex items-center gap-2">
          <FlaskConical size={14} className="text-purple-700 shrink-0" />
          <span className="text-emerald-900 font-semibold">{summary?.ph_dosing_count ?? 0} lần châm pH</span>
        </div>
        <div className="farm-muted-panel flex items-center gap-2">
          <Waves size={14} className="text-sky-700 shrink-0" />
          <span className="text-emerald-900 font-semibold">{summary?.water_operation_count ?? 0} thao tác nước</span>
        </div>
        <div className="farm-muted-panel flex items-center gap-2">
          <AlertTriangle size={14} className="text-amber-700 shrink-0" />
          <span className="text-emerald-900 font-semibold">
            {summary?.warning_count ?? 0} cảnh báo · {summary?.critical_count ?? 0} nghiêm trọng
          </span>
        </div>
      </div>

      <div className="flex flex-col md:flex-row items-stretch md:items-center gap-3">
        <div className="relative flex-1 min-w-0">
          <Search size={14} className="absolute left-3 top-1/2 -translate-y-1/2 text-emerald-700/50" />
          <input
            type="search"
            value={search}
            onChange={(e) => onSearchChange(e.target.value)}
            placeholder="Tìm theo tiêu đề, nội dung, danh mục..."
            className="ui-input pl-8"
            aria-label="Tìm kiếm nhật ký"
          />
        </div>
        <div className="flex items-center gap-2 shrink-0">
          <ShieldAlert size={14} className="text-emerald-700" />
          <Switch
            size="sm"
            checked={mode === 'all_technical'}
            onChange={(checked) => onModeChange(checked ? 'all_technical' : 'important')}
            label={mode === 'all_technical' ? 'Toàn bộ kỹ thuật' : 'Quan trọng'}
          />
        </div>
      </div>
    </div>
  );
};
