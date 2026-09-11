import { Banner } from '../ui/Banner';
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
    <Banner
      tone="warning"
      title={`XUNG ĐỘT: "${first.flowA.name}" và "${first.flowB.name}" cùng điều khiển thiết bị lúc ${first.cronExpression}`}
      action={
        <button
          type="button"
          onClick={() => onViewDetail(first.flowA)}
          className="shrink-0 rounded-xl bg-white px-3 py-1.5 text-xs font-bold text-warn-deep border border-warn-deep/30 transition-colors hover:bg-warning-bg cursor-pointer"
        >
          Xem chi tiết
        </button>
      }
    >
      {rest.length > 0 && `+${rest.length} xung đột khác`}
    </Banner>
  );
};