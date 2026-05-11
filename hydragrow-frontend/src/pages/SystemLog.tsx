import { useEffect, useState } from 'react';
import {
  AlertTriangle, CheckCircle, Info,
  Filter, Clock, Zap, Waves, RefreshCw,
  FlaskConical, Activity,
  AlertCircle, Power, Cpu,
  Beaker,
  Settings2,
  Hash
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

// ─── Các hàm tiện ích lấy giá trị linh hoạt (Cho fallback data cũ) ───────────
const getMetaNumber = (meta: any, keys: string[]): number | undefined => {
  if (!meta) return undefined;
  for (const key of keys) {
    const val = meta[key];
    if (val != null && !isNaN(Number(val))) return Number(val);
  }
  return undefined;
};

// ─── Renderers cho từng loại Metadata (Theo cấu trúc UnifiedSystemLog mới) ──

const DosingMetadata = ({ meta }: { meta: any }) => {
  if (!meta) return null;

  // Hỗ trợ cả cấu trúc DosingCycleComplete mới và dosing_report cũ
  const cycleMeta = meta.pre != null ? meta : (meta.dosing_report ?? meta.dosing_data ?? meta);

  const pre = cycleMeta.pre ?? {};
  const post = cycleMeta.post_stable ?? cycleMeta.post_mixing ?? cycleMeta.post ?? {};
  const correction = cycleMeta.correction_progress ?? {};
  const dose = cycleMeta.dose ?? cycleMeta;
  const target = cycleMeta.target ?? cycleMeta;
  // const error = cycleMeta.error ?? cycleMeta;
  // const delta = cycleMeta.delta ?? cycleMeta;

  const sections: { title?: string; rows: { label: string; value: string; accent?: string }[] }[] = [];

  // --- Dosing/Pump Rows ---
  const doseRows: { label: string; value: string; accent?: string }[] = [];
  if (dose.pump_a_ml != null && dose.pump_a_ml > 0) doseRows.push({ label: 'Phân A', value: `${Number(dose.pump_a_ml).toFixed(2)} ml`, accent: 'text-orange-400' });
  if (dose.pump_b_ml != null && dose.pump_b_ml > 0) doseRows.push({ label: 'Phân B', value: `${Number(dose.pump_b_ml).toFixed(2)} ml`, accent: 'text-orange-400' });
  if (dose.ph_up_ml != null && dose.ph_up_ml > 0) doseRows.push({ label: 'pH Tăng', value: `${Number(dose.ph_up_ml).toFixed(2)} ml`, accent: 'text-purple-400' });
  if (dose.ph_down_ml != null && dose.ph_down_ml > 0) doseRows.push({ label: 'pH Giảm', value: `${Number(dose.ph_down_ml).toFixed(2)} ml`, accent: 'text-rose-400' });
  if (doseRows.length) sections.push({ title: 'Đã Bơm', rows: doseRows });

  // --- Biến động Cảm biến ---
  const deltaRows: { label: string; value: string; accent?: string }[] = [];
  const ecBefore = getMetaNumber(pre, ['ec', 'EC', 'start_ec']);
  const ecAfter = getMetaNumber(post, ['ec', 'EC', 'after_ec']);
  const phBefore = getMetaNumber(pre, ['ph', 'pH', 'start_ph']);
  const phAfter = getMetaNumber(post, ['ph', 'pH', 'after_ph']);

  if (ecBefore != null && ecAfter != null) {
    const diff = (ecAfter - ecBefore).toFixed(2);
    const sign = ecAfter >= ecBefore ? '+' : '';
    deltaRows.push({ label: 'EC', value: `${ecBefore.toFixed(2)} → ${ecAfter.toFixed(2)} (${sign}${diff})`, accent: 'text-cyan-400' });
  }

  if (phBefore != null && phAfter != null) {
    const diff = (phAfter - phBefore).toFixed(2);
    const sign = phAfter >= phBefore ? '+' : '';
    deltaRows.push({ label: 'pH', value: `${phBefore.toFixed(2)} → ${phAfter.toFixed(2)} (${sign}${diff})`, accent: 'text-fuchsia-400' });
  }

  if (deltaRows.length) sections.push({ title: 'Biến động', rows: deltaRows });

  // --- Đánh giá Target ---
  const targetRows: { label: string; value: string; accent?: string }[] = [];
  const targetEc = getMetaNumber(target, ['ec', 'target_ec']);
  const targetPh = getMetaNumber(target, ['ph', 'target_ph']);
  // const errorEc = getMetaNumber(error, ['ec', 'error_ec']);

  if (targetEc != null) targetRows.push({ label: 'Mục tiêu EC', value: targetEc.toFixed(2), accent: 'text-cyan-300' });
  if (targetPh != null) targetRows.push({ label: 'Mục tiêu pH', value: targetPh.toFixed(2), accent: 'text-fuchsia-300' });
  if (correction.ec_remaining != null) {
    const val = Number(correction.ec_remaining);
    targetRows.push({ label: 'EC còn thiếu', value: val.toFixed(2), accent: val >= 0 ? 'text-cyan-200' : 'text-red-400' });
  }
  if (targetRows.length) sections.push({ title: 'Đánh giá', rows: targetRows });

  // --- Hệ số AI ---
  const coefRows: { label: string; value: string; accent?: string }[] = [];
  if (cycleMeta.step_ratio_ec != null) coefRows.push({ label: 'Bước EC', value: Number(cycleMeta.step_ratio_ec).toFixed(2), accent: 'text-yellow-400' });
  if (cycleMeta.ema_ec_gain_used != null) coefRows.push({ label: 'EMA EC', value: Number(cycleMeta.ema_ec_gain_used).toFixed(5), accent: 'text-cyan-500' });
  if (coefRows.length) sections.push({ title: 'Hệ số', rows: coefRows });

  if (sections.length === 0) return null;

  return (
    <div className="mt-3 space-y-2 text-xs">
      {sections.map((sec, idx) => (
        <div key={idx} className="bg-orange-950/20 border border-orange-900/40 rounded-lg px-3 py-2">
          {sec.title && <div className="text-[10px] font-semibold text-orange-300/70 mb-1 uppercase tracking-wide">{sec.title}</div>}
          <div className="grid grid-cols-2 gap-x-4 gap-y-1">
            {sec.rows.map(r => (
              <div key={r.label} className="flex items-baseline gap-1.5 min-w-0">
                <span className="text-slate-500 shrink-0">{r.label}</span>
                <span className={`${r.accent ?? 'text-slate-300'} truncate`}>{r.value}</span>
              </div>
            ))}
          </div>
        </div>
      ))}
    </div>
  );
};

const WaterMetadata = ({ meta }: { meta: any }) => {
  if (!meta) return null;
  const rows: { label: string; value: string; accent?: string }[] = [];

  // Data mới
  if (meta.level_before != null && meta.level_after != null) {
    const delta = (meta.level_after - meta.level_before).toFixed(1);
    const sign = meta.level_after >= meta.level_before ? '+' : '';
    rows.push({ label: 'Mực nước', value: `${meta.level_before.toFixed(1)} → ${meta.level_after.toFixed(1)} cm (${sign}${delta})`, accent: 'text-blue-400' });
  }

  if (meta.target_level != null) rows.push({ label: 'Mục tiêu', value: `${meta.target_level} cm`, accent: 'text-sky-300' });
  if (meta.duration_sec != null) rows.push({ label: 'Thời gian', value: `${meta.duration_sec}s` });
  if (meta.trigger) rows.push({ label: 'Nguyên nhân', value: meta.trigger });

  if (meta.success != null) {
    rows.push({ label: 'Kết quả', value: meta.success ? 'Thành công' : 'Timeout', accent: meta.success ? 'text-emerald-400' : 'text-amber-400' });
  }

  if (rows.length === 0) return <DosingMetadata meta={meta} />; // Fallback cho DB cũ

  return (
    <div className="mt-3 grid grid-cols-2 gap-x-6 gap-y-1.5 text-xs font-medium bg-blue-950/20 border border-blue-900/40 rounded-lg px-3 py-2.5">
      {rows.map(r => (
        <div key={r.label} className="flex items-baseline gap-1.5 col-span-1">
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

  // Thông tin alert chuẩn mới
  if (meta.alert_type) rows.push({ label: 'Loại lỗi', value: String(meta.alert_type), accent: 'text-red-300' });
  if (meta.source) rows.push({ label: 'Nguồn', value: String(meta.source), accent: 'text-orange-300' });
  if (meta.retry_count != null) rows.push({ label: 'Thử lại', value: `${meta.retry_count} lần`, accent: 'text-amber-400' });
  if (meta.limit_value != null) rows.push({ label: 'Ngưỡng giới hạn', value: String(meta.limit_value), accent: 'text-rose-400' });

  // Data cũ
  if (meta.message) rows.push({ label: 'Chi tiết', value: String(meta.message), accent: 'text-slate-200' });
  if (meta.pump) rows.push({ label: 'Bơm', value: String(meta.pump) });

  if (rows.length === 0) return <DosingMetadata meta={meta} />;

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

const CalibrationMetadata = ({ meta }: { meta: any }) => {
  if (!meta) return null;
  const rows: { label: string; value: string; accent?: string }[] = [];

  // Data chuẩn mới
  if (meta.parameter) rows.push({ label: 'Thông số', value: String(meta.parameter), accent: 'text-purple-300' });
  if (meta.old_value != null && meta.new_value != null) {
    rows.push({ label: 'Cập nhật', value: `${Number(meta.old_value).toFixed(4)} → ${Number(meta.new_value).toFixed(4)}`, accent: 'text-emerald-400' });
  } else if (meta.new_value != null) {
    rows.push({ label: 'Giá trị mới', value: Number(meta.new_value).toFixed(4), accent: 'text-emerald-400' });
  }
  if (meta.skip_reason) rows.push({ label: 'Lý do bỏ qua', value: String(meta.skip_reason), accent: 'text-amber-400' });

  // Data cũ
  if (meta.observed_ec_gain_per_ml != null) rows.push({ label: 'Quan sát EC gain', value: Number(meta.observed_ec_gain_per_ml).toFixed(5) });
  if (meta.start_ec != null) rows.push({ label: 'EC trước', value: Number(meta.start_ec).toFixed(2) });

  if (rows.length === 0) return null;

  return (
    <div className="mt-3 grid grid-cols-2 gap-x-6 gap-y-1.5 text-xs font-medium bg-purple-950/20 border border-purple-900/40 rounded-lg px-3 py-2.5">
      {rows.map(r => (
        <div key={r.label} className="flex items-baseline gap-1.5">
          <span className="text-slate-500 shrink-0">{r.label}</span>
          <span className={r.accent ?? 'text-slate-300'}>{r.value}</span>
        </div>
      ))}
    </div>
  );
};

// ─── Component Router điều hướng hiển thị Metadata ────────────────────────────
const MetadataRenderer = ({ category, level, metadata }: { category: string; level: string; title: string; metadata?: Record<string, any> }) => {
  if (!metadata) return null;

  // 1. Ưu tiên cao nhất: Dựa vào event_type của kiến trúc mới
  const eventType = metadata.event_type;
  if (eventType === 'WaterEvent') return <WaterMetadata meta={metadata} />;
  if (eventType === 'SystemAlert') return <AlertMetadata meta={metadata} />;
  if (eventType === 'CalibrationUpdate' || eventType === 'ema_update' || eventType === 'auto_tune') return <CalibrationMetadata meta={metadata} />;
  if (eventType === 'DosingCycleComplete' || eventType === 'dosing_cycle') return <DosingMetadata meta={metadata} />;
  if (eventType === 'BasicSystemLog') return null; // Message đã có trong text

  // 2. Fallback: Dựa vào category của hệ thống cũ
  if (category === 'dosing') return <DosingMetadata meta={metadata} />;
  if (category === 'water') return <WaterMetadata meta={metadata} />;
  if (category === 'calibration') return <CalibrationMetadata meta={metadata} />;
  if (category === 'alert' || level === 'critical' || level === 'warning') return <AlertMetadata meta={metadata} />;

  return null;
};

// ─── Kiểu hiển thị icon/color ────────────────────────────────────────────────
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
      if (title.includes('Chu trình') || title.includes('Báo cáo')) {
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
      if (title.includes('Tự điều chỉnh') || title.includes('EMA')) {
        return { icon: Settings2, iconColor: 'text-purple-400', borderColor: 'border-purple-500/20', bgColor: 'bg-purple-500/5', dot: 'bg-purple-400' };
      }
      return { icon: Activity, iconColor: 'text-emerald-400', borderColor: 'border-emerald-500/20', bgColor: 'bg-emerald-500/5', dot: 'bg-emerald-400' };

    case 'user_action':
    case 'system':
      if (title.includes('Offline') || title.includes('Mất') || title.includes('tắt bơm')) {
        return { icon: Power, iconColor: 'text-slate-400', borderColor: 'border-slate-500/20', bgColor: 'bg-slate-500/5', dot: 'bg-slate-400' };
      }
      return { icon: Cpu, iconColor: 'text-slate-300', borderColor: 'border-slate-700', bgColor: 'bg-slate-900', dot: 'bg-slate-500' };

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
        let url = `${appConfig.backend_url}/api/devices/${deviceId}/events?limit=200`;
        if (filter !== 'all') {
          url += `&category=${encodeURIComponent(filter)}`;
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
              const date = new Date(ev.timestamp > 1e12 ? ev.timestamp : ev.timestamp * 1000);
              const displayTitle = friendlyTitle(ev.title);
              const cycleId = ev.metadata?.cycle_id;

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

                    {/* Badge Cycle ID nếu có (Nhỏ xinh ở góc phải trên) */}
                    {cycleId && (
                      <div className="absolute top-3 right-4 flex items-center gap-1 opacity-50 hover:opacity-100 transition-opacity cursor-pointer">
                        <Hash size={10} className="text-slate-400" />
                        <span className="text-[9px] font-mono text-slate-400" title="Cycle ID (Mã quy trình liên kết)">
                          {String(cycleId).substring(0, 8)}
                        </span>
                      </div>
                    )}

                    <div className="flex items-start justify-between gap-2 mb-1 pr-16">
                      <div className="flex items-center gap-2 flex-wrap min-w-0">
                        <h4 className={`text-sm font-semibold leading-tight ${style.iconColor}`}>
                          {displayTitle}
                        </h4>
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
