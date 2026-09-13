import { Check, Circle } from 'lucide-react';
import type { StageChecklistItem } from '../../lib/seasons/seasonProgress';

interface SeasonStageChecklistProps {
  items: StageChecklistItem[];
  remainingDaysCount: number;
}

/**
 * Checklist các giai đoạn mùa vụ — mỗi mục chưa "done" là 1 open loop
 * (zeigarnik-effect). Luôn kèm số ngày còn lại để open loop có lối ra rõ
 * ràng, tránh biến thành lo âu thay vì động lực (skill: "incomplete
 * without a route is anxiety, not motivation").
 */
export const SeasonStageChecklist = ({ items, remainingDaysCount }: SeasonStageChecklistProps) => (
  <div className="space-y-2">
    <p className="text-[11px] font-semibold text-text-muted">
      Còn {remainingDaysCount} ngày để hoàn thành mùa vụ
    </p>
    <ul className="space-y-1.5">
      {items.map((item) => (
        <li
          key={item.name}
          className={`flex items-center gap-2 text-xs ${
            item.status === 'done'
              ? 'text-faint line-through'
              : item.status === 'current'
              ? 'text-primary-deep font-bold'
              : 'text-text-muted'
          }`}
        >
          {item.status === 'done' ? (
            <Check size={14} className="text-status shrink-0" />
          ) : (
            <Circle
              size={14}
              className={`shrink-0 ${item.status === 'current' ? 'text-primary fill-primary/20' : 'text-line'}`}
            />
          )}
          {item.name}
        </li>
      ))}
    </ul>
  </div>
);
