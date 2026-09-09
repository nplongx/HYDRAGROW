import { AlertTriangle } from 'lucide-react';
import type { ScheduleConflict } from '../../lib/automation/scheduleConflicts';
import type { UserScript } from '../../types/automation';

interface AutomationConflictBannerProps {
  conflicts: ScheduleConflict[];
  onViewDetail: (script: UserScript) => void;
}

export const AutomationConflictBanner = ({ conflicts, onViewDetail }: AutomationConflictBannerProps) => {
  if (conflicts.length === 0) return null;
  const [first, ...rest] = conflicts;

  return (
    <div className="bg-amber-50 border border-amber-300 rounded-2xl p-4 flex items-start gap-3 text-amber-900 shadow-sm">
      <AlertTriangle className="text-amber-600 shrink-0 mt-0.5" size={20} />
      <div className="space-y-1 flex-1">
        <h4 className="font-bold text-sm">
          XUNG ĐỘT: "{first.flowA.name}" và "{first.flowB.name}" cùng điều khiển thiết bị lúc {first.cronExpression}
        </h4>
        {rest.length > 0 && (
          <p className="text-xs text-amber-800/80">+{rest.length} xung đột khác</p>
        )}
      </div>
      <button
        onClick={() => onViewDetail(first.flowA)}
        className="shrink-0 px-3 py-1.5 bg-white text-amber-800 border border-amber-200 rounded-xl text-xs font-bold hover:bg-amber-100 transition-all cursor-pointer"
      >
        Xem chi tiết
      </button>
    </div>
  );
};
