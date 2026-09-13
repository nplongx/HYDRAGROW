// src/components/dosing/DosingReportCard.tsx
import { FlaskConical, Waves } from 'lucide-react';
import { PUMP_VISUAL_THEME } from '../../lib/dosing/pumpVisualTheme';

export interface DosingReportRecord {
  id: number;
  device_id: string;
  season_id?: string;
  pump_a_ml: number;
  pump_b_ml: number;
  ph_up_ml: number;
  ph_down_ml: number;
  payload?: any;
  created_at: string;
}

export const DosingReportCard = ({ record, index }: { record: DosingReportRecord; index: number }) => {
  const dosing = record.payload?.dosing_data ?? record.payload;
  if (!dosing) return null;

  const totalNutrient = record.pump_a_ml + record.pump_b_ml;
  const hasNutrient = totalNutrient > 0;
  const hasPhUp = record.ph_up_ml > 0;
  const hasPhDown = record.ph_down_ml > 0;

  let summaryTitle = 'Châm dung dịch';
  if (hasNutrient && (hasPhUp || hasPhDown)) summaryTitle = 'Bổ sung dinh dưỡng & Bơm pH';
  else if (hasNutrient) summaryTitle = 'Bổ sung phân dinh dưỡng (A/B)';
  else if (hasPhUp || hasPhDown) summaryTitle = 'Bơm dung dịch điều chỉnh pH';
  else if ((dosing.water_in_sec ?? 0) > 0) summaryTitle = 'Cấp nước pha loãng';
  else if ((dosing.water_out_sec ?? 0) > 0) summaryTitle = 'Xả bớt nước';

  const date = new Date(record.created_at);

  return (
    <div
      className="flex items-start space-x-4 animate-in slide-in-from-bottom-4 duration-500"
      style={{ animationDelay: `${Math.min(index * 40, 400)}ms`, animationFillMode: 'both' }}
    >
      {/* Node Timeline */}
      <div className="shrink-0 mt-3.5 relative z-10">
        <div
          className={`w-8 h-8 rounded-full border-4 border-white flex items-center justify-center shadow-md ${
            hasNutrient
              ? PUMP_VISUAL_THEME.nutrient.activeIcon
              : hasPhUp
              ? PUMP_VISUAL_THEME.phUp.activeIcon
              : hasPhDown
              ? PUMP_VISUAL_THEME.phDown.activeIcon
              : PUMP_VISUAL_THEME.aqua.activeIcon
          }`}
        >
          <FlaskConical size={14} strokeWidth={2.5} />
        </div>
      </div>

      {/* Thông tin châm tinh gọn */}
      <div className="flex-1 bg-white border border-line rounded-2xl p-4 hover:border-primary/40 transition-colors shadow-sm">
        <div className="flex flex-col sm:flex-row sm:items-start justify-between gap-3">
          <div className="space-y-1">
            <h4 className="text-primary-deep font-bold text-sm tracking-wide">
              {summaryTitle}
            </h4>
            <div className="flex flex-wrap items-center gap-2 pt-1 text-xs font-semibold">
              {record.pump_a_ml > 0 && (
                <span className={`px-2 py-0.5 rounded border ${PUMP_VISUAL_THEME.nutrient.badge}`}>
                  A: {record.pump_a_ml.toFixed(1)}ml
                </span>
              )}
              {record.pump_b_ml > 0 && (
                <span className={`px-2 py-0.5 rounded border ${PUMP_VISUAL_THEME.nutrient.badge}`}>
                  B: {record.pump_b_ml.toFixed(1)}ml
                </span>
              )}
              {record.ph_up_ml > 0 && (
                <span className={`px-2 py-0.5 rounded border ${PUMP_VISUAL_THEME.phUp.badge}`}>
                  pH Up: {record.ph_up_ml.toFixed(1)}ml
                </span>
              )}
              {record.ph_down_ml > 0 && (
                <span className={`px-2 py-0.5 rounded border ${PUMP_VISUAL_THEME.phDown.badge}`}>
                  pH Down: {record.ph_down_ml.toFixed(1)}ml
                </span>
              )}
              {(dosing.water_in_sec ?? 0) > 0 && (
                <span className={`px-2 py-0.5 rounded border flex items-center gap-1 ${PUMP_VISUAL_THEME.aqua.badge}`}>
                  <Waves size={10} /> Cấp nước {dosing.water_in_sec?.toFixed(1)}s
                </span>
              )}
            </div>
          </div>

          <time className="text-[10px] text-text-muted font-mono text-right whitespace-nowrap shrink-0">
            {date.toLocaleTimeString('vi-VN', { hour: '2-digit', minute: '2-digit' })}
            <span className="block font-medium text-faint mt-0.5">
              {date.toLocaleDateString('vi-VN', { day: '2-digit', month: '2-digit' })}
            </span>
          </time>
        </div>
      </div>
    </div>
  );
};
