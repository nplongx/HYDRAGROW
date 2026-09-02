// src/components/logs/CycleEventCard.tsx
import { Link2 } from 'lucide-react';
import type { SystemEvent } from './EventLogCard';

const stepDotColor = (level: string) => {
  if (level === 'critical') return 'bg-rose-500';
  if (level === 'warning') return 'bg-amber-500';
  if (level === 'success') return 'bg-emerald-500';
  return 'bg-sky-500';
};

const toDate = (timestamp: number) => new Date(timestamp > 1e12 ? timestamp : timestamp * 1000);

export interface CycleEventCardProps {
  cycleId: string;
  events: SystemEvent[];
  onOpenDetail: (event: SystemEvent) => void;
}

export const CycleEventCard = ({ cycleId, events, onOpenDetail }: CycleEventCardProps) => {
  const sorted = [...events].sort((a, b) => a.timestamp - b.timestamp);
  const first = sorted[0];
  const last = sorted[sorted.length - 1];

  return (
    <div className="border border-emerald-100 rounded-2xl p-4 bg-gradient-to-r from-cyan-500/5 to-transparent shadow-sm">
      <div className="flex items-start justify-between gap-3 mb-3">
        <div>
          <h4 className="text-sm font-bold text-emerald-950">{first.title}</h4>
          <div className="flex items-center gap-1.5 mt-1 text-[10px] font-mono text-emerald-700/75">
            <Link2 size={10} />
            <span>{cycleId}</span>
            <span>· {sorted.length} bước</span>
          </div>
        </div>
        <time className="text-[10px] text-emerald-700/75 font-mono text-right shrink-0">
          {toDate(first.timestamp).toLocaleTimeString('vi-VN', { hour: '2-digit', minute: '2-digit' })}
          {' – '}
          {toDate(last.timestamp).toLocaleTimeString('vi-VN', { hour: '2-digit', minute: '2-digit' })}
        </time>
      </div>

      <ol className="relative pl-4 space-y-2 border-l-2 border-cyan-200/60">
        {sorted.map((ev) => (
          <li key={ev.id} className="relative pl-3">
            <span className={`absolute -left-[21px] top-1 w-2.5 h-2.5 rounded-full border-2 border-white ${stepDotColor(ev.level)}`} />
            <button
              type="button"
              onClick={() => onOpenDetail(ev)}
              className="text-left w-full text-xs text-emerald-900 hover:text-emerald-700 font-medium"
            >
              {ev.title}
              {ev.message && ev.message !== ev.title && (
                <span className="block text-[11px] text-emerald-800/70 font-normal">{ev.message}</span>
              )}
            </button>
          </li>
        ))}
      </ol>
    </div>
  );
};
