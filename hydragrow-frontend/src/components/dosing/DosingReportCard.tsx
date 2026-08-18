import { useState } from 'react';
import { FlaskConical, Target, Waves, ChevronDown, ChevronUp } from 'lucide-react';

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

const getMetaNumber = (meta: any, keys: string[]): number | undefined => {
  for (const key of keys) {
    const val = meta?.[key];
    if (val != null && !isNaN(Number(val))) return Number(val);
  }
  return undefined;
};

// Bảng thông số thuật toán MIMO & Kalman
const AdvancedSpecsGrid = ({ dosing }: { dosing: any }) => {
  const pre = dosing.pre ?? {};
  const post = dosing.post_stable ?? dosing.post_mixing ?? {};
  const rows: { label: string; value: string; accent?: string }[] = [];

  const ecBefore = getMetaNumber(pre, ['tds', 'TDS', 'ec', 'EC']);
  const ecAfter = getMetaNumber(post, ['tds', 'TDS', 'ec', 'EC']);
  const phBefore = getMetaNumber(pre, ['ph', 'pH']);
  const phAfter = getMetaNumber(post, ['ph', 'pH']);

  if (ecBefore != null) rows.push({ label: 'TDS trước châm', value: ecBefore.toFixed(2), accent: 'text-cyan-700' });
  if (ecAfter != null) rows.push({ label: 'TDS sau ổn định', value: ecAfter.toFixed(2), accent: 'text-cyan-700 font-bold' });
  if (phBefore != null) rows.push({ label: 'pH trước châm', value: phBefore.toFixed(2), accent: 'text-fuchsia-600' });
  if (phAfter != null) rows.push({ label: 'pH sau ổn định', value: phAfter.toFixed(2), accent: 'text-fuchsia-600 font-bold' });

  if (dosing.target_ec != null) rows.push({ label: 'Ngưỡng TDS mục tiêu', value: Number(dosing.target_ec).toFixed(2), accent: 'text-emerald-950 font-semibold' });
  if (dosing.target_ph != null) rows.push({ label: 'Ngưỡng pH mục tiêu', value: Number(dosing.target_ph).toFixed(2), accent: 'text-emerald-950 font-semibold' });
  if (dosing.delta_ec != null) rows.push({ label: 'Biến thiên TDS (Δ)', value: Number(dosing.delta_ec).toFixed(2), accent: 'text-teal-600' });
  if (dosing.delta_ph != null) rows.push({ label: 'Biến thiên pH (Δ)', value: Number(dosing.delta_ph).toFixed(2), accent: 'text-teal-600' });

  if (dosing.ema_ec_gain_used != null) rows.push({ label: 'Hệ số Gain TDS', value: Number(dosing.ema_ec_gain_used).toFixed(5), accent: 'text-orange-600 font-mono' });
  if (dosing.ema_ph_shift_used != null) rows.push({ label: 'Hệ số Shift pH', value: Number(dosing.ema_ph_shift_used).toFixed(5), accent: 'text-orange-600 font-mono' });
  if (dosing.step_ratio_ec != null) rows.push({ label: 'Bước nhảy Kalman TDS', value: `${(Number(dosing.step_ratio_ec) * 100).toFixed(0)}%`, accent: 'text-amber-600' });
  if (dosing.step_ratio_ph != null) rows.push({ label: 'Bước nhảy Kalman pH', value: `${(Number(dosing.step_ratio_ph) * 100).toFixed(0)}%`, accent: 'text-amber-600' });

  if (rows.length === 0) return null;

  return (
    <div className="mt-3 bg-emerald-50/80 border border-emerald-100 rounded-xl p-3 animate-in slide-in-from-top-2 duration-300">
      <div className="text-[9px] font-black text-emerald-800 uppercase tracking-wider mb-2 flex items-center gap-1.5">
        <Target size={12} className="text-indigo-700" />
        Hệ Tọa Độ Thuật Toán MIMO & Kalman
      </div>
      <div className="grid grid-cols-1 sm:grid-cols-2 gap-x-6 gap-y-1.5 text-[11px]">
        {rows.map(r => (
          <div key={r.label} className="flex items-center justify-between border-b border-emerald-100/50 pb-1 last:border-transparent last:pb-0">
            <span className="text-emerald-800/80 font-medium">{r.label}</span>
            <span className={r.accent ?? 'text-emerald-950'}>{r.value}</span>
          </div>
        ))}
      </div>
    </div>
  );
};

export const DosingReportCard = ({ record, index }: { record: DosingReportRecord; index: number }) => {
  const [isExpanded, setIsExpanded] = useState(false);
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
  else if ((dosing.water_out_sec ?? 0) > 0) summaryTitle = 'Xả nước thoát bồn';

  const date = new Date(record.created_at);

  return (
    <div
      className="flex items-start space-x-4 animate-in slide-in-from-bottom-4 duration-500"
      style={{ animationDelay: `${Math.min(index * 40, 400)}ms`, animationFillMode: 'both' }}
    >
      {/* Node Timeline */}
      <div className="shrink-0 mt-3.5 relative z-10">
        <div className={`w-8 h-8 rounded-full border-4 border-white flex items-center justify-center shadow-md
          ${hasNutrient ? 'bg-orange-500 text-white' :
            (hasPhUp || hasPhDown) ? 'bg-fuchsia-600 text-white' : 'bg-blue-600 text-white'}`}
        >
          <FlaskConical size={14} strokeWidth={2.5} />
        </div>
      </div>

      {/* Thẻ thông tin châm */}
      <div className="flex-1 bg-white border border-emerald-100 rounded-2xl p-4 hover:border-emerald-300 transition-colors shadow-sm">
        <div className="flex flex-col sm:flex-row sm:items-start justify-between gap-3 mb-2">
          <div className="space-y-1">
            <h4 className="text-emerald-950 font-bold text-sm tracking-wide">
              {summaryTitle}
            </h4>
            <div className="flex flex-wrap items-center gap-2 pt-1 text-xs font-semibold">
              {record.pump_a_ml > 0 && <span className="text-orange-700 bg-orange-50 px-2 py-0.5 rounded border border-orange-200">A: {record.pump_a_ml.toFixed(1)}ml</span>}
              {record.pump_b_ml > 0 && <span className="text-orange-700 bg-orange-50 px-2 py-0.5 rounded border border-orange-200">B: {record.pump_b_ml.toFixed(1)}ml</span>}
              {record.ph_up_ml > 0 && <span className="text-purple-700 bg-purple-50 px-2 py-0.5 rounded border border-purple-200">pH Up: {record.ph_up_ml.toFixed(1)}ml</span>}
              {record.ph_down_ml > 0 && <span className="text-red-700 bg-red-50 px-2 py-0.5 rounded border border-red-200">pH Down: {record.ph_down_ml.toFixed(1)}ml</span>}
              {(dosing.water_in_sec ?? 0) > 0 && (
                <span className="text-blue-700 bg-blue-50 px-2 py-0.5 rounded border border-blue-200 flex items-center gap-1">
                  <Waves size={10} /> Cấp {dosing.water_in_sec?.toFixed(1)}s
                </span>
              )}
            </div>
          </div>
          <time className="text-[10px] text-emerald-700/75 font-mono text-right whitespace-nowrap shrink-0">
            {date.toLocaleTimeString('vi-VN', { hour: '2-digit', minute: '2-digit' })}
            <span className="block font-medium text-emerald-700/60 mt-0.5">{date.toLocaleDateString('vi-VN', { day: '2-digit', month: '2-digit' })}</span>
          </time>
        </div>

        {/* Toggle xem thông số kỹ thuật MIMO */}
        <div className="mt-3 pt-2.5 border-t border-emerald-100">
          <button
            onClick={() => setIsExpanded(!isExpanded)}
            className="flex items-center gap-1.5 text-[10px] font-bold text-emerald-700/75 hover:text-emerald-950 uppercase tracking-wider transition-colors cursor-pointer"
          >
            <span>{isExpanded ? 'Thu nhỏ thông số' : 'Xem thông số kỹ thuật MIMO'}</span>
            {isExpanded ? <ChevronUp size={12} /> : <ChevronDown size={12} />}
          </button>
          {isExpanded && <AdvancedSpecsGrid dosing={dosing} />}
        </div>
      </div>
    </div>
  );
};
