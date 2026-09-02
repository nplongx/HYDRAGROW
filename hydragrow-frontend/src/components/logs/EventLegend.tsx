import { EVENT_LEGEND_ENTRIES } from './eventLegendData';

export type { LegendEntry } from './eventLegendData';
export { EVENT_LEGEND_ENTRIES } from './eventLegendData';

export const EventLegend = () => (
  <div className="grid grid-cols-2 md:grid-cols-4 gap-2.5">
    {EVENT_LEGEND_ENTRIES.map((entry) => (
      <div key={entry.label} className="flex items-start gap-2 text-xs">
        <span className={`mt-0.5 w-2.5 h-2.5 rounded-full shrink-0 ${entry.swatchClassName}`} />
        <div>
          <p className="font-semibold text-emerald-900">{entry.label}</p>
          <p className="text-emerald-700/70 text-[11px] leading-snug">{entry.description}</p>
        </div>
      </div>
    ))}
  </div>
);
