import { PUMP_VISUAL_THEME, PUMP_BAR_FILL } from '../../lib/dosing/pumpVisualTheme';

type PumpField = 'pump_a_ml' | 'pump_b_ml' | 'ph_up_ml' | 'ph_down_ml';

const SERIES: { field: PumpField; label: string; theme: keyof typeof PUMP_VISUAL_THEME }[] = [
  { field: 'pump_a_ml', label: 'Phân A', theme: 'nutrient' },
  { field: 'pump_b_ml', label: 'Phân B', theme: 'nutrient' },
  { field: 'ph_up_ml', label: 'pH Up', theme: 'phUp' },
  { field: 'ph_down_ml', label: 'pH Down', theme: 'phDown' },
];

export interface DosingHourlyChartProps {
  bucketsByPump: Record<PumpField, number[]>;
}

export const DosingHourlyChart = ({ bucketsByPump }: DosingHourlyChartProps) => {
  const max = Math.max(
    1,
    ...SERIES.flatMap((s) => bucketsByPump[s.field]),
  );

  return (
    <div className="space-y-3">
      {/* Chú giải — dùng chung pumpVisualTheme với card điều khiển */}
      <div className="flex flex-wrap gap-3 text-[10px] font-semibold text-text-muted">
        {SERIES.map((s) => (
          <span key={s.field} className="inline-flex items-center gap-1.5">
            <span aria-hidden="true" className={`h-2 w-2 rounded-full ${PUMP_BAR_FILL[s.field]}`} />
            {s.label}
          </span>
        ))}
      </div>

      {/* Biểu đồ cột nhóm — 24 cột giờ, mỗi cột 4 thanh mini */}
      <div className="flex items-end gap-1 h-24 pt-2">
        {Array.from({ length: 24 }, (_, hour) => (
          <div
            key={hour}
            aria-label={`Giờ ${hour}: ${SERIES.map((s) => `${s.label} ${bucketsByPump[s.field][hour].toFixed(1)}ml`).join(', ')}`}
            className="flex-1 flex items-end gap-px h-full"
          >
            {SERIES.map((s) => {
              const v = bucketsByPump[s.field][hour];
              return (
                <div
                  key={s.field}
                  title={`Giờ ${hour} · ${s.label}: ${v.toFixed(1)}ml`}
                  className={`flex-1 ${PUMP_BAR_FILL[s.field]} rounded-t transition-colors`}
                  style={{ height: `${Math.max(v > 0 ? 3 : 0, (v / max) * 100)}%` }}
                />
              );
            })}
          </div>
        ))}
      </div>

      {/* Nhãn trục giờ — mốc 0/6/12/18 để không cần hover mới biết đang xem khung giờ nào */}
      <div className="flex justify-between text-[10px] text-faint font-medium px-0.5">
        <span>0h</span>
        <span>6h</span>
        <span>12h</span>
        <span>18h</span>
      </div>

      {/* Text alternative cho screen reader — data-visualization skill: "Provide text alternatives for charts" */}
      <table className="sr-only" aria-label="Lượng châm theo giờ, chi tiết từng bơm">
        <caption>Lượng châm dinh dưỡng và pH theo từng giờ trong ngày, tính bằng ml</caption>
        <thead>
          <tr>
            <th scope="col">Khung giờ</th>
            {SERIES.map((s) => (
              <th key={s.field} scope="col">{s.label} (ml)</th>
            ))}
          </tr>
        </thead>
        <tbody>
          {Array.from({ length: 24 }, (_, hour) => (
            <tr key={hour}>
              <th scope="row">{hour}:00</th>
              {SERIES.map((s) => (
                <td key={s.field}>{bucketsByPump[s.field][hour].toFixed(1)}ml</td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
};
