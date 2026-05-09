import { useEffect, useState } from 'react';
import {
  AlertTriangle, CheckCircle, Info,
  Filter, Clock, Zap, Waves, RefreshCw,
  FlaskConical, Activity,
  AlertCircle, Power, Radio, Cpu,
  Beaker,
  Wifi,
  Settings2
} from 'lucide-react';
import { PageHeader } from '../components/ui/PageHeader';
import { StateView } from '../components/ui/StateView';
import { httpFetch } from '../platform/http';
import { loadAppSettings } from '../platform/settings';

// ─── Kiểu event từ backend ───────────────────────────────────────────────────
interface SystemEvent {
  id: number;
  device_id: string;
  level: string;
  category: string;
  title: string;
  message: string;
  reason?: string;
  metadata?: Record<string, any>;
  timestamp: number;
}

// ─── Các hàm tiện ích lấy giá trị linh hoạt ─────────────────────────────────
const getMetaNumber = (meta: any, keys: string[]): number | undefined => {
  for (const key of keys) {
    const val = meta[key];
    if (val != null && !isNaN(Number(val))) return Number(val);
  }
  return undefined;
};

// ─── Renderer metadata theo từng category (được cải tiến) ────────────────────

const DosingMetadata = ({ meta }: { meta: any }) => {
  if (!meta) return null;

  const rows: { label: string; value: string; accent?: string }[] = [];

  // Hiển thị lượng bơm
  if (meta.pump_a_ml != null && meta.pump_a_ml > 0) rows.push({ label: 'Phân A', value: `${Number(meta.pump_a_ml).toFixed(2)} ml`, accent: 'text-orange-400' });
  if (meta.pump_b_ml != null && meta.pump_b_ml > 0) rows.push({ label: 'Phân B', value: `${Number(meta.pump_b_ml).toFixed(2)} ml`, accent: 'text-orange-400' });
  if (meta.ph_up_ml != null && meta.ph_up_ml > 0) rows.push({ label: 'pH Tăng', value: `${Number(meta.ph_up_ml).toFixed(2)} ml`, accent: 'text-purple-400' });
  if (meta.ph_down_ml != null && meta.ph_down_ml > 0) rows.push({ label: 'pH Giảm', value: `${Number(meta.ph_down_ml).toFixed(2)} ml`, accent: 'text-rose-400' });

  // EC trước/sau
  const startEc = getMetaNumber(meta, ['start_ec', 'before_ec', 'ec_before', 'pre_ec']);
  const afterEc = getMetaNumber(meta, ['after_ec', 'stabilized_ec', 'ec_after', 'post_ec']);
  if (startEc != null && afterEc != null) {
    const delta = (afterEc - startEc).toFixed(2);
    const sign = afterEc >= startEc ? '+' : '';
    rows.push({ label: 'EC thay đổi', value: `${startEc.toFixed(2)} → ${afterEc.toFixed(2)} (${sign}${delta})`, accent: 'text-cyan-400' });
  } else if (startEc != null) {
    rows.push({ label: 'EC trước', value: startEc.toFixed(2), accent: 'text-cyan-400' });
  } else if (afterEc != null) {
    rows.push({ label: 'EC sau', value: afterEc.toFixed(2), accent: 'text-cyan-400' });
  }

  // pH trước/sau
  const startPh = getMetaNumber(meta, ['start_ph', 'before_ph', 'ph_before', 'pre_ph']);
  const afterPh = getMetaNumber(meta, ['after_ph', 'stabilized_ph', 'ph_after', 'post_ph']);
  if (startPh != null && afterPh != null) {
    const delta = (afterPh - startPh).toFixed(2);
    const sign = afterPh >= startPh ? '+' : '';
    rows.push({ label: 'pH thay đổi', value: `${startPh.toFixed(2)} → ${afterPh.toFixed(2)} (${sign}${delta})`, accent: 'text-fuchsia-400' });
  } else if (startPh != null) {
    rows.push({ label: 'pH trước', value: startPh.toFixed(2), accent: 'text-fuchsia-400' });
  } else if (afterPh != null) {
    rows.push({ label: 'pH sau', value: afterPh.toFixed(2), accent: 'text-fuchsia-400' });
  }

  // Mục tiêu
  if (meta.target_ec != null) rows.push({ label: 'Mục tiêu EC', value: Number(meta.target_ec).toFixed(2), accent: 'text-cyan-300' });
  if (meta.target_ph != null) rows.push({ label: 'Mục tiêu pH', value: Number(meta.target_ph).toFixed(2), accent: 'text-fuchsia-300' });

  if (rows.length === 0) return null;

  return (
    <div className="mt-3 grid grid-cols-2 gap-x-6 gap-y-1.5 text-xs font-medium bg-slate-950/60 border border-slate-800 rounded-lg px-3 py-2.5">
      {rows.map(r => (
        <div key={r.label} className="flex items-baseline gap-1.5">
          <span className="text-slate-500 shrink-0">{r.label}</span>
          <span className={r.accent ?? 'text-slate-300'}>{r.value}</span>
        </div>
      ))}
    </div>
  );
};




const SensorNoiseMetadata = ({ meta }: { meta: any }) => {
  if (!meta) return null;
  const rows: { label: string; value: string; accent?: string }[] = [];

  if (meta.sensor) rows.push({ label: 'Cảm biến', value: String(meta.sensor), accent: 'text-amber-300' });
  if (meta.raw_value != null) rows.push({ label: 'Raw', value: Number(meta.raw_value).toFixed(3), accent: 'text-orange-300' });
  if (meta.prev_value != null) rows.push({ label: 'Prev', value: Number(meta.prev_value).toFixed(3), accent: 'text-slate-300' });
  if (meta.delta != null) rows.push({ label: 'Δ', value: Number(meta.delta).toFixed(3), accent: 'text-amber-400' });
  if (meta.threshold != null) rows.push({ label: 'Ngưỡng', value: Number(meta.threshold).toFixed(3) });

  if (rows.length === 0) return null;
  return (
    <div className="mt-3 grid grid-cols-2 gap-x-6 gap-y-1.5 text-xs font-medium bg-amber-950/20 border border-amber-900/40 rounded-lg px-3 py-2.5">
      {rows.map(r => (
        <div key={r.label} className="flex items-baseline gap-1.5">
          <span className="text-slate-500 shrink-0">{r.label}</span>
          <span className={r.accent ?? 'text-slate-300'}>{r.value}</span>
        </div>
      ))}
    </div>
  );
};


// ─── CalibrationMetadata (mở rộng: EMA update & Auto-tune) ───────────────
const CalibrationMetadata = ({ meta }: { meta: any }) => {
  if (!meta) return null;
  // Dữ liệu calibration raw từ backend gồm nhiều trường:
  //   runtime_coefficients: { ec_gain_per_ml, ph_shift_up_per_ml, ph_shift_down_per_ml, step_ratio_ec, step_ratio_ph, auto_tune_locked }
  //   observed_ec_gain_per_ml, observed_ph_up_per_ml, observed_ph_down_per_ml
  //   start_ec, start_ph, ec_after, ph_after
  //   pump_a_ml, pump_b_ml, ph_up_ml, ph_down_ml

  const rows: { label: string; value: string; accent?: string }[] = [];

  // Hệ số runtime hiện tại
  if (meta.runtime_coefficients) {
    const rc = meta.runtime_coefficients;
    if (rc.ec_gain_per_ml != null) rows.push({ label: 'EC gain/ml (hiện tại)', value: Number(rc.ec_gain_per_ml).toFixed(5), accent: 'text-cyan-400' });
    if (rc.ph_shift_up_per_ml != null) rows.push({ label: 'pH↑/ml (hiện tại)', value: Number(rc.ph_shift_up_per_ml).toFixed(5), accent: 'text-emerald-400' });
    if (rc.ph_shift_down_per_ml != null) rows.push({ label: 'pH↓/ml (hiện tại)', value: Number(rc.ph_shift_down_per_ml).toFixed(5), accent: 'text-rose-400' });
    if (rc.step_ratio_ec != null) rows.push({ label: 'Bước EC', value: Number(rc.step_ratio_ec).toFixed(2), accent: 'text-yellow-400' });
    if (rc.step_ratio_ph != null) rows.push({ label: 'Bước pH', value: Number(rc.step_ratio_ph).toFixed(2), accent: 'text-yellow-400' });
    if (rc.auto_tune_locked != null) rows.push({ label: 'Khóa tự động', value: rc.auto_tune_locked ? 'Có' : 'Không' });
  }

  // Giá trị quan sát thực tế
  if (meta.observed_ec_gain_per_ml != null) rows.push({ label: 'Quan sát EC gain', value: Number(meta.observed_ec_gain_per_ml).toFixed(5), accent: 'text-yellow-400' });
  if (meta.observed_ph_up_per_ml != null) rows.push({ label: 'Quan sát pH↑/ml', value: Number(meta.observed_ph_up_per_ml).toFixed(5), accent: 'text-yellow-400' });
  if (meta.observed_ph_down_per_ml != null) rows.push({ label: 'Quan sát pH↓/ml', value: Number(meta.observed_ph_down_per_ml).toFixed(5), accent: 'text-yellow-400' });

  // Điều kiện bắt đầu và kết thúc
  if (meta.start_ec != null) rows.push({ label: 'EC trước', value: Number(meta.start_ec).toFixed(2), accent: 'text-cyan-400' });
  if (meta.ec_after != null) rows.push({ label: 'EC sau', value: Number(meta.ec_after).toFixed(2), accent: 'text-cyan-400' });
  if (meta.start_ph != null) rows.push({ label: 'pH trước', value: Number(meta.start_ph).toFixed(2), accent: 'text-fuchsia-400' });
  if (meta.ph_after != null) rows.push({ label: 'pH sau', value: Number(meta.ph_after).toFixed(2), accent: 'text-fuchsia-400' });

  // Liều lượng bơm trong chu kỳ này
  if (meta.pump_a_ml != null && meta.pump_a_ml > 0) rows.push({ label: 'Phân A', value: `${Number(meta.pump_a_ml).toFixed(2)} ml`, accent: 'text-orange-400' });
  if (meta.pump_b_ml != null && meta.pump_b_ml > 0) rows.push({ label: 'Phân B', value: `${Number(meta.pump_b_ml).toFixed(2)} ml`, accent: 'text-orange-400' });
  if (meta.ph_up_ml != null && meta.ph_up_ml > 0) rows.push({ label: 'pH Tăng', value: `${Number(meta.ph_up_ml).toFixed(2)} ml`, accent: 'text-purple-400' });
  if (meta.ph_down_ml != null && meta.ph_down_ml > 0) rows.push({ label: 'pH Giảm', value: `${Number(meta.ph_down_ml).toFixed(2)} ml`, accent: 'text-rose-400' });

  if (meta.alpha != null) rows.push({ label: 'Alpha (EMA)', value: Number(meta.alpha).toFixed(2) });
  if (meta.result) {
    const r = meta.result;
    if (r.ph_v7 != null) rows.push({ label: 'V tại pH 7', value: `${Number(r.ph_v7).toFixed(4)} V` });
    if (r.ph_v4 != null) rows.push({ label: 'V tại pH 4', value: `${Number(r.ph_v4).toFixed(4)} V` });
    if (r.ph_v10 != null) rows.push({ label: 'V tại pH 10', value: `${Number(r.ph_v10).toFixed(4)} V` });
  }
  if (meta.mode) rows.push({ label: 'Chế độ', value: meta.mode });
  if (meta.error != null) rows.push({ label: 'Sai số', value: `${Number(meta.error).toFixed(4)} mV`, accent: Number(meta.error) < 10 ? 'text-emerald-400' : 'text-amber-400' });

  if (rows.length === 0) return null;

  return (
    <div className="mt-3 grid grid-cols-2 gap-x-6 gap-y-1.5 text-xs font-medium bg-slate-950/60 border border-slate-800 rounded-lg px-3 py-2.5">
      {rows.map(r => (
        <div key={r.label} className="flex items-baseline gap-1.5">
          <span className="text-slate-500 shrink-0">{r.label}</span>
          <span className={r.accent ?? 'text-slate-300'}>{r.value}</span>
        </div>
      ))}
    </div>
  );
};
// ========== Cập nhật WaterMetadata ==========
const WaterMetadata = ({ meta }: { meta: any }) => {
  if (!meta) return null;
  const rows: { label: string; value: string; accent?: string }[] = [];

  // Mực nước
  const levelBefore = getMetaNumber(meta, ['level_before', 'water_before']);
  const levelAfter = getMetaNumber(meta, ['level_after', 'water_after']);
  if (levelBefore != null && levelAfter != null) {
    const delta = (levelAfter - levelBefore).toFixed(1);
    const sign = levelAfter >= levelBefore ? '+' : '';
    rows.push({ label: 'Mực nước', value: `${levelBefore.toFixed(1)} → ${levelAfter.toFixed(1)} cm (${sign}${delta})`, accent: 'text-blue-400' });
  }

  if (meta.duration_sec != null) rows.push({ label: 'Thời gian', value: `${meta.duration_sec}s` });

  // EC
  const ecBefore = getMetaNumber(meta, ['ec_before', 'before_ec', 'start_ec']);
  const ecAfter = getMetaNumber(meta, ['ec_after', 'after_ec']);
  if (ecBefore != null && ecAfter != null) {
    const delta = (ecAfter - ecBefore).toFixed(2);
    rows.push({ label: 'EC', value: `${ecBefore.toFixed(2)} → ${ecAfter.toFixed(2)} (${delta})`, accent: 'text-cyan-400' });
  } else if (ecBefore != null) {
    rows.push({ label: 'EC trước', value: ecBefore.toFixed(2), accent: 'text-cyan-400' });
  } else if (ecAfter != null) {
    rows.push({ label: 'EC sau', value: ecAfter.toFixed(2), accent: 'text-cyan-400' });
  }

  // pH – BỔ SUNG
  const phBefore = getMetaNumber(meta, ['ph_before', 'before_ph', 'start_ph']);
  const phAfter = getMetaNumber(meta, ['ph_after', 'after_ph']);
  if (phBefore != null && phAfter != null) {
    const delta = (phAfter - phBefore).toFixed(2);
    rows.push({ label: 'pH', value: `${phBefore.toFixed(2)} → ${phAfter.toFixed(2)} (${delta})`, accent: 'text-fuchsia-400' });
  } else if (phBefore != null) {
    rows.push({ label: 'pH trước', value: phBefore.toFixed(2), accent: 'text-fuchsia-400' });
  } else if (phAfter != null) {
    rows.push({ label: 'pH sau', value: phAfter.toFixed(2), accent: 'text-fuchsia-400' });
  }

  if (meta.trigger) rows.push({ label: 'Nguyên nhân', value: meta.trigger });
  if (meta.success != null) rows.push({ label: 'Kết quả', value: meta.success ? 'Thành công' : 'Timeout', accent: meta.success ? 'text-emerald-400' : 'text-amber-400' });

  if (rows.length === 0) return null;

  return (
    <div className="mt-3 grid grid-cols-2 gap-x-6 gap-y-1.5 text-xs font-medium bg-slate-950/60 border border-slate-800 rounded-lg px-3 py-2.5">
      {rows.map(r => (
        <div key={r.label} className="flex items-baseline gap-1.5 col-span-1">
          <span className="text-slate-500 shrink-0">{r.label}</span>
          <span className={r.accent ?? 'text-slate-300'}>{r.value}</span>
        </div>
      ))}
    </div>
  );
};
// ========== Cập nhật DosingCycleMetadata (rõ ràng hơn) ==========
// Chỉ hiển thị phần sửa đổi của DosingCycleMetadata và bổ sung nếu cần
const DosingCycleMetadata = ({ meta }: { meta: any }) => {
  if (!meta) return null;
  const pre = meta.pre ?? {};
  const post = meta.post_stable ?? meta.post ?? {};
  const correction = meta.correction_progress ?? {};

  const sections: { title?: string; rows: { label: string; value: string; accent?: string }[] }[] = [];

  // 1. Thông tin chu kỳ
  const infoRows: { label: string; value: string; accent?: string }[] = [];
  if (meta.cycle_id) infoRows.push({ label: 'Cycle ID', value: String(meta.cycle_id).slice(0, 8), accent: 'text-slate-200' });
  if (meta.trigger) infoRows.push({ label: 'Trigger', value: String(meta.trigger) });
  if (meta.duration_ms != null) infoRows.push({ label: 'Thời gian', value: `${(Number(meta.duration_ms) / 1000).toFixed(1)}s` });
  if (infoRows.length) sections.push({ title: 'Chu kỳ', rows: infoRows });

  // 2. Liều lượng bơm
  const doseRows: { label: string; value: string; accent?: string }[] = [];
  const dose = meta.dose ?? {};
  if (dose.pump_a_ml != null && dose.pump_a_ml > 0) doseRows.push({ label: 'Phân A', value: `${Number(dose.pump_a_ml).toFixed(2)} ml`, accent: 'text-orange-400' });
  if (dose.pump_b_ml != null && dose.pump_b_ml > 0) doseRows.push({ label: 'Phân B', value: `${Number(dose.pump_b_ml).toFixed(2)} ml`, accent: 'text-orange-400' });
  if (dose.ph_up_ml != null && dose.ph_up_ml > 0) doseRows.push({ label: 'pH Tăng', value: `${Number(dose.ph_up_ml).toFixed(2)} ml`, accent: 'text-purple-400' });
  if (dose.ph_down_ml != null && dose.ph_down_ml > 0) doseRows.push({ label: 'pH Giảm', value: `${Number(dose.ph_down_ml).toFixed(2)} ml`, accent: 'text-rose-400' });
  if (doseRows.length) sections.push({ title: 'Bơm', rows: doseRows });

  // 3. Biến động EC/pH (trước → sau ổn định)
  const deltaRows: { label: string; value: string; accent?: string }[] = [];
  const ecBefore = getMetaNumber(pre, ['ec', 'EC']);
  const ecAfter = getMetaNumber(post, ['ec', 'EC']);
  const phBefore = getMetaNumber(pre, ['ph', 'pH']);
  const phAfter = getMetaNumber(post, ['ph', 'pH']);

  if (ecBefore != null && ecAfter != null) {
    const delta = (ecAfter - ecBefore).toFixed(2);
    const sign = ecAfter >= ecBefore ? '+' : '';
    deltaRows.push({ label: 'EC', value: `${ecBefore.toFixed(2)} → ${ecAfter.toFixed(2)} (${sign}${delta})`, accent: 'text-cyan-400' });
  } else if (ecBefore != null) deltaRows.push({ label: 'EC trước', value: ecBefore.toFixed(2), accent: 'text-cyan-400' });
  else if (ecAfter != null) deltaRows.push({ label: 'EC sau', value: ecAfter.toFixed(2), accent: 'text-cyan-400' });

  if (phBefore != null && phAfter != null) {
    const delta = (phAfter - phBefore).toFixed(2);
    const sign = phAfter >= phBefore ? '+' : '';
    deltaRows.push({ label: 'pH', value: `${phBefore.toFixed(2)} → ${phAfter.toFixed(2)} (${sign}${delta})`, accent: 'text-fuchsia-400' });
  } else if (phBefore != null) deltaRows.push({ label: 'pH trước', value: phBefore.toFixed(2), accent: 'text-fuchsia-400' });
  else if (phAfter != null) deltaRows.push({ label: 'pH sau', value: phAfter.toFixed(2), accent: 'text-fuchsia-400' });

  if (meta.delta_ec != null) deltaRows.push({ label: 'Δ EC', value: Number(meta.delta_ec).toFixed(2), accent: 'text-cyan-300' });
  if (meta.delta_ph != null) deltaRows.push({ label: 'Δ pH', value: Number(meta.delta_ph).toFixed(2), accent: 'text-fuchsia-300' });
  if (deltaRows.length) sections.push({ title: 'Biến động', rows: deltaRows });

  // 4. Mục tiêu, sai số & tiến độ hiệu chỉnh còn lại
  const targetRows: { label: string; value: string; accent?: string }[] = [];
  if (meta.target_ec != null) targetRows.push({ label: 'Mục tiêu EC', value: Number(meta.target_ec).toFixed(2), accent: 'text-cyan-300' });
  if (meta.target_ph != null) targetRows.push({ label: 'Mục tiêu pH', value: Number(meta.target_ph).toFixed(2), accent: 'text-fuchsia-300' });
  if (meta.error_ec != null) targetRows.push({ label: 'Sai số EC', value: Number(meta.error_ec).toFixed(2), accent: 'text-amber-400' });
  if (meta.error_ph != null) targetRows.push({ label: 'Sai số pH', value: Number(meta.error_ph).toFixed(2), accent: 'text-amber-400' });

  // Hiển thị tiến độ còn thiếu sau khi châm (nếu có)
  if (correction.ec_remaining != null) {
    const val = Number(correction.ec_remaining);
    targetRows.push({ label: 'EC còn thiếu', value: val.toFixed(2), accent: val >= 0 ? 'text-cyan-200' : 'text-red-400' });
  }
  if (correction.ph_remaining != null) {
    const val = Number(correction.ph_remaining);
    targetRows.push({ label: 'pH còn thiếu', value: val.toFixed(2), accent: val >= 0 ? 'text-fuchsia-200' : 'text-red-400' });
  }

  if (targetRows.length) sections.push({ title: 'Đánh giá', rows: targetRows });

  // 5. Hệ số sử dụng trong lần châm này
  const coefRows: { label: string; value: string; accent?: string }[] = [];
  if (meta.step_ratio_ec != null) coefRows.push({ label: 'Bước EC', value: Number(meta.step_ratio_ec).toFixed(2), accent: 'text-yellow-400' });
  if (meta.step_ratio_ph != null) coefRows.push({ label: 'Bước pH', value: Number(meta.step_ratio_ph).toFixed(2), accent: 'text-yellow-400' });
  if (meta.ema_ec_gain_used != null) coefRows.push({ label: 'EMA EC gain', value: Number(meta.ema_ec_gain_used).toFixed(5), accent: 'text-cyan-500' });
  if (meta.ema_ph_shift_used != null) coefRows.push({ label: 'EMA pH shift', value: Number(meta.ema_ph_shift_used).toFixed(5), accent: 'text-fuchsia-500' });
  if (coefRows.length) sections.push({ title: 'Hệ số', rows: coefRows });

  if (sections.length === 0) return null;

  return (
    <div className="mt-3 space-y-2 text-xs">
      {sections.map((sec, idx) => (
        <div key={idx} className="bg-orange-950/20 border border-orange-900/40 rounded-lg px-3 py-2">
          {sec.title && <div className="text-[10px] font-semibold text-orange-300/70 mb-1 uppercase tracking-wide">{sec.title}</div>}
          <div className="grid grid-cols-2 gap-x-4 gap-y-1">
            {sec.rows.map(r => (
              <div key={r.label} className="flex items-baseline gap-1.5">
                <span className="text-slate-500 shrink-0">{r.label}</span>
                <span className={r.accent ?? 'text-slate-300'}>{r.value}</span>
              </div>
            ))}
          </div>
        </div>
      ))}
    </div>
  );
};

// ========== Cập nhật AlertMetadata (lỗi rõ ràng) ==========
const AlertMetadata = ({ meta }: { meta: any }) => {
  if (!meta) return null;
  const rows: { label: string; value: string; accent?: string }[] = [];

  // Thông tin alert từ backend: type, source, message, retry_count, pump
  if (meta.type) rows.push({ label: 'Loại', value: String(meta.type), accent: 'text-red-300' });
  if (meta.source) rows.push({ label: 'Nguồn', value: String(meta.source), accent: 'text-orange-300' });
  if (meta.message) rows.push({ label: 'Chi tiết', value: String(meta.message), accent: 'text-slate-200' });
  if (meta.retry_count != null) rows.push({ label: 'Thử lại', value: String(meta.retry_count), accent: 'text-amber-400' });
  if (meta.pump) rows.push({ label: 'Bơm', value: String(meta.pump) });

  // Các giá trị cảm biến (nếu có)
  if (meta.ec != null) rows.push({ label: 'EC', value: Number(meta.ec).toFixed(2), accent: 'text-cyan-400' });
  if (meta.ph != null) rows.push({ label: 'pH', value: Number(meta.ph).toFixed(2), accent: 'text-fuchsia-400' });
  if (meta.temp != null) rows.push({ label: 'Nhiệt độ', value: `${Number(meta.temp).toFixed(1)}°C`, accent: 'text-orange-400' });
  if (meta.water_level != null) rows.push({ label: 'Mực nước', value: `${Number(meta.water_level).toFixed(1)} cm`, accent: 'text-blue-400' });
  if (meta.err_ec === true) rows.push({ label: 'Cảm biến EC', value: 'LỖI', accent: 'text-red-400' });
  if (meta.err_ph === true) rows.push({ label: 'Cảm biến pH', value: 'LỖI', accent: 'text-red-400' });
  if (meta.err_water === true) rows.push({ label: 'Cảm biến nước', value: 'LỖI', accent: 'text-red-400' });
  if (meta.err_temp === true) rows.push({ label: 'Cảm biến nhiệt', value: 'LỖI', accent: 'text-red-400' });

  if (rows.length === 0) return null;

  return (
    <div className="mt-3 grid grid-cols-2 gap-x-6 gap-y-1.5 text-xs font-medium bg-red-950/30 border border-red-900/40 rounded-lg px-3 py-2.5">
      {rows.map(r => (
        <div key={r.label} className="flex items-baseline gap-1.5">
          <span className="text-slate-500 shrink-0">{r.label}</span>
          <span className={r.accent ?? 'text-slate-300'}>{r.value}</span>
        </div>
      ))}
    </div>
  );
};
const MetadataRenderer = ({ category, level, metadata }: { category: string; level: string; title: string; metadata?: Record<string, any> }) => {
  if (!metadata) return null;

  // dosing: ưu tiên dạng dosing cycle nếu có pre/post
  if (category === 'dosing') {
    if (metadata.pre || metadata.post || metadata.post_stable) return <DosingCycleMetadata meta={metadata} />;
    return <DosingMetadata meta={metadata} />;
  }
  if (category === 'water') return <WaterMetadata meta={metadata} />;
  if (category === 'calibration') return <CalibrationMetadata meta={metadata} />;
  if (category === 'sensor_noise' || category === 'sensor') return <SensorNoiseMetadata meta={metadata} />;
  if (category === 'alert' || level === 'critical' || level === 'warning') return <AlertMetadata meta={metadata} />;

  // fallback: thử parse như dosing nếu có dữ liệu EC/pH
  if (metadata.pump_a_ml != null || metadata.start_ec != null || metadata.before_ph != null) {
    return <DosingMetadata meta={metadata} />;
  }

  return null;
};

// ─── Kiểu hiển thị theo category + level ────────────────────────────────────
type EventStyle = {
  icon: React.ElementType;
  iconColor: string;
  borderColor: string;
  bgColor: string;
  dot: string;
};

const getEventStyle = (event: SystemEvent): EventStyle => {
  const { level, category, title } = event;

  if (level === 'critical' || title.toLowerCase().includes('khẩn cấp') || title.toLowerCase().includes('emergency')) {
    return { icon: AlertCircle, iconColor: 'text-red-500', borderColor: 'border-red-500/25', bgColor: 'bg-red-500/5', dot: 'bg-red-500' };
  }
  if (level === 'warning') {
    return { icon: AlertTriangle, iconColor: 'text-amber-500', borderColor: 'border-amber-500/25', bgColor: 'bg-amber-500/5', dot: 'bg-amber-500' };
  }

  switch (category) {
    case 'dosing':
      if (title.includes('Chu trình châm phân') || title.includes('Dosing Cycle')) {
        return { icon: FlaskConical, iconColor: 'text-orange-400', borderColor: 'border-orange-500/20', bgColor: 'bg-orange-500/5', dot: 'bg-orange-400' };
      }
      if (title.includes('pH') || title.includes('Điều Chỉnh')) {
        return { icon: Beaker, iconColor: 'text-fuchsia-500', borderColor: 'border-fuchsia-500/20', bgColor: 'bg-fuchsia-500/5', dot: 'bg-fuchsia-500' };
      }
      if (title.includes('Sục') || title.includes('Trộn')) {
        return { icon: RefreshCw, iconColor: 'text-purple-400', borderColor: 'border-purple-500/20', bgColor: 'bg-purple-500/5', dot: 'bg-purple-400' };
      }
      return { icon: FlaskConical, iconColor: 'text-orange-400', borderColor: 'border-orange-500/20', bgColor: 'bg-orange-500/5', dot: 'bg-orange-400' };

    case 'water':
      return { icon: Waves, iconColor: 'text-blue-400', borderColor: 'border-blue-500/20', bgColor: 'bg-blue-500/5', dot: 'bg-blue-400' };

    case 'calibration':
      if (title.includes('Tự điều chỉnh bước châm') || title.includes('AUTO TUNE')) {
        return { icon: Settings2, iconColor: 'text-purple-400', borderColor: 'border-purple-500/20', bgColor: 'bg-purple-500/5', dot: 'bg-purple-400' };
      }
      return { icon: Activity, iconColor: 'text-emerald-400', borderColor: 'border-emerald-500/20', bgColor: 'bg-emerald-500/5', dot: 'bg-emerald-400' };

    case 'sensor_noise':
    case 'sensor':
      return { icon: Radio, iconColor: 'text-amber-400', borderColor: 'border-amber-500/20', bgColor: 'bg-amber-500/5', dot: 'bg-amber-400' };

    case 'system':
      if (title.includes('Offline') || title.includes('Mất') || title.includes('ngắt')) {
        return { icon: Wifi, iconColor: 'text-red-400', borderColor: 'border-red-500/20', bgColor: 'bg-red-500/5', dot: 'bg-red-400' };
      }
      if (title.includes('Trực tuyến') || title.includes('Online') || title.includes('kết nối')) {
        return { icon: Radio, iconColor: 'text-emerald-400', borderColor: 'border-emerald-500/20', bgColor: 'bg-emerald-500/5', dot: 'bg-emerald-400' };
      }
      return { icon: Cpu, iconColor: 'text-slate-400', borderColor: 'border-slate-700', bgColor: 'bg-slate-900', dot: 'bg-slate-500' };

    case 'alert':
    default:
      if (level === 'success') {
        return { icon: CheckCircle, iconColor: 'text-emerald-500', borderColor: 'border-emerald-500/20', bgColor: 'bg-emerald-500/5', dot: 'bg-emerald-500' };
      }
      return { icon: Info, iconColor: 'text-slate-400', borderColor: 'border-slate-700', bgColor: 'bg-slate-900', dot: 'bg-slate-500' };
  }
};

// ─── Filters ─────────────────────────────────────────────────────────────────
const FILTERS = [
  { id: 'all', label: 'Tất cả', icon: Filter },
  { id: 'alert', label: 'Cảnh báo', icon: AlertTriangle },
  { id: 'dosing', label: 'Dinh dưỡng', icon: FlaskConical },
  { id: 'water', label: 'Nước', icon: Waves },
  { id: 'calibration', label: 'Hiệu chuẩn', icon: Activity },
  { id: 'sensor', label: 'Cảm biến', icon: Radio }, // giờ sẽ gửi thêm sensor_noise
  { id: 'system', label: 'Hệ thống', icon: Power },
];

// ─── Tiêu đề thân thiện hơn ──────────────────────────────────────────────────
const friendlyTitle = (title: string): string => {
  const map: Record<string, string> = {
    'Dừng Khẩn Cấp!': 'Dừng khẩn cấp',
    'Lỗi Hệ Thống!': 'Lỗi hệ thống',
    'Hoàn Tất Chu Trình': 'Hoàn tất chu trình châm',
    'Cấp Nước': 'Cấp nước vào bồn',
    'Xả Nước': 'Xả nước ra ngoài',
    'Sục Trộn Dinh Dưỡng': 'Sục trộn dung dịch',
    'Điều Chỉnh pH': 'Điều chỉnh pH',
    'Runtime Calibration Tự Động (EMA)': 'Cập nhật hệ số EMA',
    'Cập nhật hệ số châm phân động': 'Cập nhật hệ số động',
    'Lưu Báo Cáo Châm Phân Thành Công': 'Báo cáo châm phân',
  };
  return map[title] ?? title;
};

// ─── FSM State badge (bổ sung các state mới) ─────────────────────────────────
const FsmBadge = ({ message }: { message: string }) => {
  const stateMap: Record<string, { label: string; color: string }> = {
    'WaterRefilling': { label: 'Cấp nước', color: 'text-blue-400 bg-blue-500/10 border-blue-500/20' },
    'WaterDraining': { label: 'Xả nước', color: 'text-sky-400 bg-sky-500/10 border-sky-500/20' },
    'DosingPumpA': { label: 'Châm A', color: 'text-orange-400 bg-orange-500/10 border-orange-500/20' },
    'DosingPumpB': { label: 'Châm B', color: 'text-orange-400 bg-orange-500/10 border-orange-500/20' },
    'DosingPH': { label: 'Chỉnh pH', color: 'text-fuchsia-400 bg-fuchsia-500/10 border-fuchsia-500/20' },
    'ActiveMixing': { label: 'Sục trộn', color: 'text-purple-400 bg-purple-500/10 border-purple-500/20' },
    'Stabilizing': { label: 'Chờ ổn định', color: 'text-amber-400 bg-amber-500/10 border-amber-500/20' },
    'DosingStabilizing': { label: 'Chờ ổn định', color: 'text-amber-400 bg-amber-500/10 border-amber-500/20' },
    'Monitoring': { label: 'Giám sát', color: 'text-slate-400 bg-slate-800 border-slate-700' },
    'Idle': { label: 'Nghỉ', color: 'text-slate-400 bg-slate-800 border-slate-700' },
    'EmergencyStop': { label: 'Dừng khẩn', color: 'text-red-400 bg-red-500/10 border-red-500/20' },
  };
  const matched = stateMap[message];
  if (!matched) return null;
  return (
    <span className={`px-2 py-0.5 rounded text-[10px] font-semibold border ${matched.color}`}>
      {matched.label}
    </span>
  );
};

// ─── Component chính (cải thiện fetch URL để lọc sensor kép) ─────────────────
const SystemLog = () => {
  const [filter, setFilter] = useState<string>('all');
  const [appConfig, setAppConfig] = useState<any>(null);
  const [deviceId, setDeviceId] = useState<string | null>(null);
  const [systemEvents, setSystemEvents] = useState<SystemEvent[]>([]);
  const [isLoading, setIsLoading] = useState(false);

  useEffect(() => {
    const init = async () => {
      const settings: any = await loadAppSettings().catch(() => null);
      if (settings?.device_id) {
        setAppConfig(settings);
        setDeviceId(settings.device_id);
      }
    };
    init();
  }, []);

  useEffect(() => {
    if (!deviceId || !appConfig) return;
    const loadEvents = async () => {
      setIsLoading(true);
      try {
        let url = `${appConfig.backend_url}/api/devices/${deviceId}/events?limit=200`;
        // Xử lý filter: nếu là 'sensor' thì gửi cả sensor và sensor_noise
        if (filter !== 'all') {
          if (filter === 'sensor') {
            url += '&category=sensor&category=sensor_noise';
          } else {
            url += `&category=${filter}`;
          }
        }
        const res = await httpFetch(url, {
          headers: { 'X-API-Key': appConfig.api_key || '' }
        });
        if (res.ok) {
          const data = await res.json();
          setSystemEvents(data.data ?? []);
        }
      } catch (e) {
        console.error(e);
      } finally {
        setIsLoading(false);
      }
    };
    loadEvents();
  }, [filter, deviceId, appConfig]);

  return (
    <div className="p-4 md:p-8 max-w-4xl mx-auto pb-28">
      <PageHeader
        icon={Clock}
        title="Nhật Ký Hệ Thống"
        subtitle={`Lịch sử vận hành trạm ${deviceId || '—'}`}
      />

      {/* Bộ lọc */}
      <div className="bg-slate-900 border border-slate-800 rounded-xl p-4 mb-8">
        <div className="flex flex-wrap gap-2">
          {FILTERS.map(btn => {
            const Icon = btn.icon;
            const active = filter === btn.id;
            return (
              <button
                key={btn.id}
                onClick={() => setFilter(btn.id)}
                className={`flex items-center gap-1.5 px-3.5 py-2 rounded-lg text-xs font-medium transition-colors border
                  ${active
                    ? 'bg-blue-600 text-white border-blue-500'
                    : 'bg-slate-950 text-slate-400 border-slate-800 hover:bg-slate-800 hover:text-slate-200'
                  }`}
              >
                <Icon size={13} />
                {btn.label}
              </button>
            );
          })}
        </div>
      </div>

      {/* Timeline */}
      {isLoading ? (
        <div className="flex items-center justify-center gap-3 py-16 text-slate-500">
          <div className="w-5 h-5 border-2 border-slate-700 border-t-blue-500 rounded-full animate-spin" />
          <span className="text-sm font-medium">Đang tải nhật ký...</span>
        </div>
      ) : systemEvents.length === 0 ? (
        <StateView
          icon={Zap}
          title="Chưa có sự kiện nào"
          description="Hệ thống chưa ghi nhận sự kiện nào theo bộ lọc hiện tại."
        />
      ) : (
        <div className="relative pl-4">
          {/* Đường dọc timeline */}
          <div className="absolute left-[17px] top-3 bottom-3 w-px bg-slate-800/80" />

          <div className="space-y-4">
            {systemEvents.map((ev, idx) => {
              const style = getEventStyle(ev);
              const Icon = style.icon;
              const date = new Date(
                ev.timestamp > 1e12 ? ev.timestamp : ev.timestamp * 1000
              );
              const displayTitle = friendlyTitle(ev.title);

              return (
                <div key={ev.id ?? idx} className="relative flex gap-4">
                  {/* Dot */}
                  <div className="relative z-10 shrink-0 mt-3.5">
                    <div className={`w-6 h-6 rounded-full border-2 border-slate-950 flex items-center justify-center ${style.dot}`}>
                      <Icon size={12} className="text-white" strokeWidth={2.5} />
                    </div>
                  </div>

                  {/* Card */}
                  <div className={`flex-1 min-w-0 border rounded-xl p-4 transition-colors hover:brightness-110 ${style.bgColor} ${style.borderColor}`}>
                    <div className="flex items-start justify-between gap-2 mb-1">
                      <div className="flex items-center gap-2 flex-wrap min-w-0">
                        <h4 className={`text-sm font-semibold leading-tight ${style.iconColor}`}>
                          {displayTitle}
                        </h4>
                        <FsmBadge message={ev.message} />
                      </div>
                      <time className="text-[10px] text-slate-500 font-mono whitespace-nowrap shrink-0 mt-0.5">
                        {date.toLocaleTimeString('vi-VN', { hour: '2-digit', minute: '2-digit', second: '2-digit' })}
                        <span className="block text-center">
                          {date.toLocaleDateString('vi-VN', { day: '2-digit', month: '2-digit' })}
                        </span>
                      </time>
                    </div>

                    {/* Nội dung message */}
                    {ev.message && ev.message !== ev.title && !ev.message.startsWith('Monitoring') && ev.level !== 'FSM_UPDATE' && (
                      <p className="text-xs text-slate-400 leading-relaxed mt-1">
                        {ev.message}
                      </p>
                    )}

                    {ev.reason && (
                      <div className="mt-2 flex items-center gap-1.5">
                        <span className="text-[10px] text-slate-500">Mã lỗi:</span>
                        <span className="px-2 py-0.5 rounded text-[10px] font-mono bg-slate-900 border border-slate-700 text-slate-400">
                          {ev.reason}
                        </span>
                      </div>
                    )}
                    <MetadataRenderer
                      category={ev.category}
                      level={ev.level}
                      title={ev.title}
                      metadata={ev.metadata}
                    />
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      )}
    </div>
  );
};

export default SystemLog;
