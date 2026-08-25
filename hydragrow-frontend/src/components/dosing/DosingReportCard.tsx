// src/components/dosing/DosingReportCard.tsx
import { FlaskConical, Waves } from 'lucide-react';

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

  const status = record.payload?.status ?? record.payload?.result ?? 'success';
  let borderColor = 'border-l-4 border-l-emerald-500';
  if (status === 'partial' || status === 'warning') borderColor = 'border-l-4 border-l-amber-400';
  else if (status === 'failed' || status === 'error') borderColor = 'border-l-4 border-l-red-500';

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
              ? 'bg-orange-500 text-white'
              : hasPhUp || hasPhDown
              ? 'bg-fuchsia-600 text-white'
              : 'bg-blue-600 text-white'
          }`}
        >
          <FlaskConical size={14} strokeWidth={2.5} />
        </div>
      </div>

      {/* Thông tin châm tinh gọn */}
      <div className={`ui-card flex-1 transition-colors ${borderColor}`}>
        <div className="flex flex-col sm:flex-row sm:items-start justify-between gap-3">
          <div className="space-y-1">
            <h4 className="text-emerald-950 font-bold text-sm tracking-wide">
              {summaryTitle}
            </h4>
            <div className="flex flex-wrap items-center gap-2 pt-1 text-xs font-semibold">
              {record.pump_a_ml > 0 && (
                <span className="text-orange-700 bg-orange-50 px-2 py-0.5 rounded border border-orange-200">
                  A: {record.pump_a_ml.toFixed(1)}ml
                </span>
              )}
              {record.pump_b_ml > 0 && (
                <span className="text-orange-700 bg-orange-50 px-2 py-0.5 rounded border border-orange-200">
                  B: {record.pump_b_ml.toFixed(1)}ml
                </span>
              )}
              {record.ph_up_ml > 0 && (
                <span className="text-purple-700 bg-purple-50 px-2 py-0.5 rounded border border-purple-200">
                  pH Up: {record.ph_up_ml.toFixed(1)}ml
                </span>
              )}
              {record.ph_down_ml > 0 && (
                <span className="text-red-700 bg-red-50 px-2 py-0.5 rounded border border-red-200">
                  pH Down: {record.ph_down_ml.toFixed(1)}ml
                </span>
              )}
              {(dosing.water_in_sec ?? 0) > 0 && (
                <span className="text-blue-700 bg-blue-50 px-2 py-0.5 rounded border border-blue-200 flex items-center gap-1">
                  <Waves size={10} /> Cấp nước {dosing.water_in_sec?.toFixed(1)}s
                </span>
              )}
            </div>
          </div>

          <time className="text-[10px] text-emerald-700/75 font-mono text-right whitespace-nowrap shrink-0">
            {date.toLocaleTimeString('vi-VN', { hour: '2-digit', minute: '2-digit' })}
            <span className="block font-medium text-emerald-700/60 mt-0.5">
              {date.toLocaleDateString('vi-VN', { day: '2-digit', month: '2-digit' })}
            </span>
          </time>
        </div>
      </div>
    </div>
  );
};
