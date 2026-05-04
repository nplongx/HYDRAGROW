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

// ─── Renderer metadata theo từng category ────────────────────────────────────

const DosingMetadata = ({ meta }: { meta: any }) => {
  const d = meta?.dosing_data ?? meta?.dosing_report ?? meta;
  if (!d) return null;

  const rows: { label: string; value: string; accent?: string }[] = [];

  if (d.pump_a_ml != null || d.pump_b_ml != null) {
    if (d.pump_a_ml != null && d.pump_a_ml > 0) rows.push({ label: 'Phân A', value: `${Number(d.pump_a_ml).toFixed(2)} ml`, accent: 'text-orange-400' });
    if (d.pump_b_ml != null && d.pump_b_ml > 0) rows.push({ label: 'Phân B', value: `${Number(d.pump_b_ml).toFixed(2)} ml`, accent: 'text-orange-400' });
    if (d.ph_up_ml != null && d.ph_up_ml > 0) rows.push({ label: 'pH Tăng', value: `${Number(d.ph_up_ml).toFixed(2)} ml`, accent: 'text-purple-400' });
    if (d.ph_down_ml != null && d.ph_down_ml > 0) rows.push({ label: 'pH Giảm', value: `${Number(d.ph_down_ml).toFixed(2)} ml`, accent: 'text-rose-400' });
  }

  const startEc = d.start_ec ?? d.before_ec;
  const afterEc = d.after_ec ?? d.stabilized_ec;
  if (startEc != null && afterEc != null) {
    const delta = (afterEc - startEc).toFixed(2);
    const sign = afterEc >= startEc ? '+' : '';
    rows.push({ label: 'EC thay đổi', value: `${Number(startEc).toFixed(2)} → ${Number(afterEc).toFixed(2)} (${sign}${delta})`, accent: 'text-cyan-400' });
  }
  const startPh = d.start_ph ?? d.before_ph;
  const afterPh = d.after_ph ?? d.stabilized_ph;
  if (startPh != null && afterPh != null) {
    const delta = (afterPh - startPh).toFixed(2);
    const sign = afterPh >= startPh ? '+' : '';
    rows.push({ label: 'pH thay đổi', value: `${Number(startPh).toFixed(2)} → ${Number(afterPh).toFixed(2)} (${sign}${delta})`, accent: 'text-fuchsia-400' });
  }
  if (d.target_ec != null) rows.push({ label: 'Mục tiêu EC', value: Number(d.target_ec).toFixed(2) });

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

const WaterMetadata = ({ meta }: { meta: any }) => {
  if (!meta) return null;
  const rows: { label: string; value: string; accent?: string }[] = [];

  if (meta.level_before != null && meta.level_after != null) {
    const delta = (meta.level_after - meta.level_before).toFixed(1);
    const sign = meta.level_after >= meta.level_before ? '+' : '';
    rows.push({ label: 'Mực nước', value: `${Number(meta.level_before).toFixed(1)} → ${Number(meta.level_after).toFixed(1)} cm (${sign}${delta})`, accent: 'text-blue-400' });
  }
  if (meta.duration_sec != null) rows.push({ label: 'Thời gian', value: `${meta.duration_sec}s` });
  if (meta.ec_before != null && meta.ec_after != null) {
    rows.push({ label: 'EC', value: `${Number(meta.ec_before).toFixed(2)} → ${Number(meta.ec_after).toFixed(2)}`, accent: 'text-cyan-400' });
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

const CalibrationMetadata = ({ meta }: { meta: any }) => {
  if (!meta) return null;
  const rows: { label: string; value: string; accent?: string }[] = [];

  // EMA runtime calibration
  if (meta.runtime_coefficients) {
    const rc = meta.runtime_coefficients;
    if (rc.ec_gain_per_ml != null) rows.push({ label: 'EC gain/ml', value: Number(rc.ec_gain_per_ml).toFixed(5), accent: 'text-cyan-400' });
    if (rc.ph_shift_up_per_ml != null) rows.push({ label: 'pH↑/ml', value: Number(rc.ph_shift_up_per_ml).toFixed(5), accent: 'text-emerald-400' });
    if (rc.ph_shift_down_per_ml != null) rows.push({ label: 'pH↓/ml', value: Number(rc.ph_shift_down_per_ml).toFixed(5), accent: 'text-rose-400' });
  }
  if (meta.alpha != null) rows.push({ label: 'Alpha (EMA)', value: Number(meta.alpha).toFixed(2) });
  if (meta.observed_ec_gain_per_ml != null) rows.push({ label: 'Quan sát EC', value: Number(meta.observed_ec_gain_per_ml).toFixed(5), accent: 'text-yellow-400' });

  // pH calibration
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

const DosingCycleMetadata = ({ meta }: { meta: any }) => {
  if (!meta) return null;
  const pre = meta.pre ?? {};
  const post = meta.post_stable ?? meta.post ?? {};
  const rows: { label: string; value: string; accent?: string }[] = [];

  if (meta.cycle_id) rows.push({ label: 'Cycle ID', value: String(meta.cycle_id), accent: 'text-slate-200' });
  if (meta.trigger) rows.push({ label: 'Trigger', value: String(meta.trigger) });
  if (pre.ec != null && post.ec != null) rows.push({ label: 'EC', value: `${Number(pre.ec).toFixed(2)} → ${Number(post.ec).toFixed(2)}`, accent: 'text-cyan-400' });
  if (pre.ph != null && post.ph != null) rows.push({ label: 'pH', value: `${Number(pre.ph).toFixed(2)} → ${Number(post.ph).toFixed(2)}`, accent: 'text-fuchsia-400' });
  if (meta.delta_ec != null) rows.push({ label: 'Δ EC', value: Number(meta.delta_ec).toFixed(2), accent: 'text-cyan-300' });
  if (meta.delta_ph != null) rows.push({ label: 'Δ pH', value: Number(meta.delta_ph).toFixed(2), accent: 'text-fuchsia-300' });
  if (meta.duration_sec != null) rows.push({ label: 'Thời gian', value: `${meta.duration_sec}s` });

  if (rows.length === 0) return null;
  return (
    <div className="mt-3 grid grid-cols-2 gap-x-6 gap-y-1.5 text-xs font-medium bg-orange-950/20 border border-orange-900/40 rounded-lg px-3 py-2.5">
      {rows.map(r => (
        <div key={r.label} className="flex items-baseline gap-1.5">
          <span className="text-slate-500 shrink-0">{r.label}</span>
          <span className={r.accent ?? 'text-slate-300'}>{r.value}</span>
        </div>
      ))}
    </div>
  );
};

const AlertMetadata = ({ meta }: { meta: any }) => {
  if (!meta) return null;
  const rows: { label: string; value: string; accent?: string }[] = [];

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

  if (category === 'dosing') {
    if (metadata.cycle_id || metadata.pre || metadata.post_stable || metadata.post) return <DosingCycleMetadata meta={metadata} />;
    return <DosingMetadata meta={metadata} />;
  }
  if (category === 'water') return <WaterMetadata meta={metadata} />;
  if (category === 'calibration') return <CalibrationMetadata meta={metadata} />;
  if (category === 'sensor') return <SensorNoiseMetadata meta={metadata} />;
  if (category === 'alert' || level === 'critical' || level === 'warning') return <AlertMetadata meta={metadata} />;

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
  { id: 'sensor', label: 'Cảm biến', icon: Radio },
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

// ─── FSM State badge ──────────────────────────────────────────────────────────

const FsmBadge = ({ message }: { message: string }) => {
  const stateMap: Record<string, { label: string; color: string }> = {
    'WaterRefilling': { label: 'Cấp nước', color: 'text-blue-400 bg-blue-500/10 border-blue-500/20' },
    'WaterDraining': { label: 'Xả nước', color: 'text-sky-400 bg-sky-500/10 border-sky-500/20' },
    'DosingPumpA': { label: 'Châm A', color: 'text-orange-400 bg-orange-500/10 border-orange-500/20' },
    'DosingPumpB': { label: 'Châm B', color: 'text-orange-400 bg-orange-500/10 border-orange-500/20' },
    'DosingPH': { label: 'Chỉnh pH', color: 'text-fuchsia-400 bg-fuchsia-500/10 border-fuchsia-500/20' },
    'ActiveMixing': { label: 'Sục trộn', color: 'text-purple-400 bg-purple-500/10 border-purple-500/20' },
    'Stabilizing': { label: 'Chờ ổn định', color: 'text-amber-400 bg-amber-500/10 border-amber-500/20' },
    'Monitoring': { label: 'Giám sát', color: 'text-slate-400 bg-slate-800 border-slate-700' },
  };
  const matched = stateMap[message];
  if (!matched) return null;
  return (
    <span className={`px-2 py-0.5 rounded text-[10px] font-semibold border ${matched.color}`}>
      {matched.label}
    </span>
  );
};

// ─── Component chính ─────────────────────────────────────────────────────────

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
        const catParam = filter !== 'all' ? `&category=${filter}` : '';
        const res = await httpFetch(
          `${appConfig.backend_url}/api/devices/${deviceId}/events?limit=200${catParam}`,
          { headers: { 'X-API-Key': appConfig.api_key || '' } }
        );
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

                    {/* Nội dung message — ẩn nếu trùng title hoặc là FSM state thuần */}
                    {ev.message && ev.message !== ev.title && !ev.message.startsWith('Monitoring') && ev.level !== 'FSM_UPDATE' && (
                      <p className="text-xs text-slate-400 leading-relaxed mt-1">
                        {ev.message}
                      </p>
                    )}

                    {/* Reason badge */}
                    {ev.reason && (
                      <span className="inline-block mt-2 px-2 py-0.5 rounded text-[10px] font-mono bg-slate-900 border border-slate-700 text-slate-400">
                        {ev.reason}
                      </span>
                    )}

                    {/* Metadata có cấu trúc */}
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
