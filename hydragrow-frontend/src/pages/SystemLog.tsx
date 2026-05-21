import { useEffect, useState } from 'react';
import {
  AlertTriangle, CheckCircle, Info, Filter, Clock, Zap, Waves,
  FlaskConical, AlertCircle, Power, Cpu,
  Beaker, Settings2, Radio, Wifi, Download, UserCheck,
  ShieldAlert
} from 'lucide-react';
import toast from 'react-hot-toast';
import { PageHeader } from '../components/ui/PageHeader';
import { StateView } from '../components/ui/StateView';
import { httpFetch } from '../platform/http';
import { saveTextFile } from '../platform/file';
import { loadAppSettings } from '../platform/settings';

// ─── Kiểu dữ liệu cấu trúc Event từ API Gateway ───────────────────────────────
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

// Helper trích xuất số an toàn tránh crash giao diện
const getMetaNumber = (meta: any, keys: string[]): number | undefined => {
  if (!meta) return undefined;
  for (const key of keys) {
    const val = meta[key];
    if (val != null && !isNaN(Number(val))) return Number(val);
  }
  return undefined;
};

// ─── Renderers giải mã Metadata cấu trúc MIMO Giai đoạn 5 ──────────────────────

const DosingMetadata = ({ meta }: { meta: any }) => {
  if (!meta) return null;

  // Hỗ trợ cả cấu trúc DosingCycleComplete mới và gói telemetry dự phòng cũ
  const cycleMeta = meta.pre != null ? meta : (meta.dosing_report ?? meta.dosing_data ?? meta);

  const pre = cycleMeta.pre ?? {};
  const post = cycleMeta.post_stable ?? cycleMeta.post_mixing ?? cycleMeta.post ?? {};
  // const correction = cycleMeta.correction_progress ?? {};
  const dose = cycleMeta.dose ?? cycleMeta;
  const target = cycleMeta.target ?? cycleMeta;

  const sections: { title?: string; rows: { label: string; value: string; accent?: string }[] }[] = [];

  // --- Khối 1: Lượng hóa chất phân phối thực tế (ml) ---
  const doseRows: { label: string; value: string; accent?: string }[] = [];
  if (dose.pump_a_ml != null && dose.pump_a_ml > 0) doseRows.push({ label: 'Dinh dưỡng A:', value: `${Number(dose.pump_a_ml).toFixed(1)} ml`, accent: 'text-orange-400 font-bold' });
  if (dose.pump_b_ml != null && dose.pump_b_ml > 0) doseRows.push({ label: 'Dinh dưỡng B:', value: `${Number(dose.pump_b_ml).toFixed(1)} ml`, accent: 'text-orange-400 font-bold' });
  if (dose.ph_up_ml != null && dose.ph_up_ml > 0) doseRows.push({ label: 'Thuốc pH Up:', value: `${Number(dose.ph_up_ml).toFixed(1)} ml`, accent: 'text-purple-400 font-bold' });
  if (dose.ph_down_ml != null && dose.ph_down_ml > 0) doseRows.push({ label: 'Thuốc pH Down:', value: `${Number(dose.ph_down_ml).toFixed(1)} ml`, accent: 'text-rose-400 font-bold' });
  if (doseRows.length) sections.push({ title: 'Khối lượng châm phần cứng', rows: doseRows });

  // --- Khối 2: Biến thiên điện hóa trước/sau khi bão hòa nước ---
  const deltaRows: { label: string; value: string; accent?: string }[] = [];
  const ecBefore = getMetaNumber(pre, ['ec', 'EC', 'start_ec']);
  const ecAfter = getMetaNumber(post, ['ec', 'EC', 'after_ec', 'post_mixing_ec']);
  const phBefore = getMetaNumber(pre, ['ph', 'pH', 'start_ph']);
  const phAfter = getMetaNumber(post, ['ph', 'pH', 'after_ph', 'post_mixing_ph']);

  if (ecBefore != null && ecAfter != null && ecBefore !== 0.0) {
    const diff = ecAfter - ecBefore;
    const sign = diff >= 0 ? '+' : '';
    deltaRows.push({ label: 'Chỉ số EC:', value: `${ecBefore.toFixed(2)} → ${ecAfter.toFixed(2)} (${sign}${diff.toFixed(2)})`, accent: 'text-cyan-400 font-mono font-bold' });
  }

  if (phBefore != null && phAfter != null && phBefore !== 0.0) {
    const diff = phAfter - phBefore;
    const sign = diff >= 0 ? '+' : '';
    deltaRows.push({ label: 'Chỉ số pH:', value: `${phBefore.toFixed(2)} → ${phAfter.toFixed(2)} (${sign}${diff.toFixed(2)})`, accent: 'text-fuchsia-400 font-mono font-bold' });
  }
  if (deltaRows.length) sections.push({ title: 'Biến động cảm biến bão hòa', rows: deltaRows });

  // --- Khối 3: Đánh giá sai số mục tiêu và Thích ứng của ma trận ---
  const targetRows: { label: string; value: string; accent?: string }[] = [];
  const targetEc = getMetaNumber(target, ['ec', 'target_ec']);
  const targetPh = getMetaNumber(target, ['ph', 'target_ph']);

  if (targetEc != null && targetEc > 0) targetRows.push({ label: 'Điểm đặt EC:', value: targetEc.toFixed(2), accent: 'text-cyan-300 font-bold' });
  if (targetPh != null && targetPh > 0) targetRows.push({ label: 'Điểm đặt pH:', value: targetPh.toFixed(2), accent: 'text-fuchsia-300 font-bold' });

  // Trích xuất bước nhảy thích ứng step_ratio nạp từ Kalman Filter
  if (cycleMeta.step_ratio_ec != null) {
    targetRows.push({ label: 'AI Step EC:', value: `${(cycleMeta.step_ratio_ec * 100).toFixed(0)}%`, accent: 'text-teal-400 font-bold' });
  }
  if (cycleMeta.stabilized_window_sec != null) {
    targetRows.push({ label: 'Độ trễ tĩnh bồn:', value: `${cycleMeta.stabilized_window_sec} giây`, accent: 'text-indigo-400 font-bold' });
  }
  if (sections.length === 0) return null;

  return (
    <div className="mt-3 space-y-2 text-xs relative z-10 animate-in fade-in duration-300">
      {sections.map((sec, idx) => (
        <div key={idx} className="bg-slate-950/40 border border-slate-800/60 rounded-xl px-3 py-2">
          {sec.title && <div className="text-[9px] font-black text-slate-500 mb-1 uppercase tracking-wider">{sec.title}</div>}
          <div className="flex flex-col gap-1.5">
            {sec.rows.map(r => (
              <div key={r.label} className="flex items-center justify-between min-w-0 border-b border-white/5 last:border-transparent pb-1 last:pb-0">
                <span className="text-slate-400 text-[11px] font-medium">{r.label}</span>
                <span className={`${r.accent ?? 'text-slate-300'} text-[11px]`}>{r.value}</span>
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

  if (meta.level_before != null && meta.level_after != null) {
    const delta = meta.level_after - meta.level_before;
    const sign = delta >= 0 ? '+' : '';
    rows.push({ label: 'Hành trình mức nước:', value: `${meta.level_before.toFixed(1)} → ${meta.level_after.toFixed(1)} cm (${sign}${delta.toFixed(1)})`, accent: 'text-blue-400 font-bold' });
  }

  if (meta.target_level != null) rows.push({ label: 'Điểm đặt Target:', value: `${meta.target_level} cm`, accent: 'text-sky-300 font-bold' });
  if (meta.duration_sec != null) rows.push({ label: 'Thời gian kích rơ-le:', value: `${meta.duration_sec}s`, accent: 'font-mono' });
  if (meta.trigger) rows.push({ label: 'Lệnh kích hoạt:', value: String(meta.trigger), accent: 'italic text-slate-400' });

  if (meta.success != null) {
    rows.push({ label: 'Kết quả chu kỳ:', value: meta.success ? 'Thành công' : 'Lỗi Timeout', accent: meta.success ? 'text-emerald-400 font-bold' : 'text-rose-400 font-bold' });
  }

  if (rows.length === 0) return <DosingMetadata meta={meta} />;

  return (
    <div className="mt-3 flex flex-col gap-1.5 text-xs font-medium bg-slate-950/40 border border-slate-800/60 rounded-xl px-3 py-2.5 animate-in fade-in duration-300">
      <div className="text-[9px] font-black text-slate-500 mb-0.5 uppercase tracking-wider">Hydraulics Flow Log</div>
      {rows.map(r => (
        <div key={r.label} className="flex items-center justify-between border-b border-white/5 last:border-transparent pb-1 last:pb-0">
          <span className="text-slate-400 text-[11px]">{r.label}</span>
          <span className={`${r.accent ?? 'text-slate-300'} text-[11px]`}>{r.value}</span>
        </div>
      ))}
    </div>
  );
};

const AlertMetadata = ({ meta }: { meta: any }) => {
  if (!meta) return null;
  const rows: { label: string; value: string; accent?: string }[] = [];

  if (meta.alert_type) rows.push({ label: 'Mã chẩn đoán lỗi:', value: String(meta.alert_type), accent: 'text-rose-400 font-bold font-mono' });
  if (meta.source) rows.push({ label: 'Khối phát hiện:', value: String(meta.source), accent: 'text-orange-400 font-medium' });
  if (meta.retry_count != null) rows.push({ label: 'Vòng lặp thử lại:', value: `${meta.retry_count} lần`, accent: 'text-amber-400 font-mono' });
  if (meta.limit_value != null) rows.push({ label: 'Ngưỡng kịch trần:', value: String(meta.limit_value), accent: 'text-rose-400 font-mono' });

  if (rows.length === 0) return <DosingMetadata meta={meta} />;

  return (
    <div className="mt-3 flex flex-col gap-1.5 text-xs font-medium bg-rose-950/10 border border-rose-900/30 rounded-xl px-3 py-2.5 animate-in fade-in duration-300">
      <div className="text-[9px] font-black text-rose-400/70 mb-0.5 uppercase tracking-wider flex items-center gap-1">
        <ShieldAlert size={10} /> Root Cause Diagnostic Snapshot
      </div>
      {rows.map(r => (
        <div key={r.label} className="flex items-center justify-between border-b border-white/5 last:border-transparent pb-1 last:pb-0">
          <span className="text-slate-400 text-[11px]">{r.label}</span>
          <span className={`${r.accent ?? 'text-slate-300'} text-[11px]`}>{r.value}</span>
        </div>
      ))}
    </div>
  );
};

const CalibrationMetadata = ({ meta }: { meta: any }) => {
  if (!meta) return null;
  const rows: { label: string; value: string; accent?: string }[] = [];

  if (meta.parameter) rows.push({ label: 'Trục ma trận học tập:', value: String(meta.parameter), accent: 'text-purple-400 font-bold' });
  if (meta.old_value != null && meta.new_value != null) {
    rows.push({ label: 'Hệ số tăng Kalman:', value: `${Number(meta.old_value).toFixed(4)} → ${Number(meta.new_value).toFixed(4)}`, accent: 'text-emerald-400 font-mono font-bold' });
  } else if (meta.new_value != null) {
    rows.push({ label: 'Trọng số ma trận mới:', value: Number(meta.new_value).toFixed(4), accent: 'text-emerald-400 font-mono font-bold' });
  }
  if (meta.skip_reason) rows.push({ label: 'Lý do bộ lọc chặn chặn:', value: String(meta.skip_reason), accent: 'text-amber-400 font-medium' });

  if (rows.length === 0) return null;

  return (
    <div className="mt-3 flex flex-col gap-1.5 text-xs font-medium bg-slate-950/40 border border-slate-800/60 rounded-xl px-3 py-2.5 animate-in fade-in duration-300">
      <div className="text-[9px] font-black text-slate-500 mb-0.5 uppercase tracking-wider">MIMO Gain Evolution</div>
      {rows.map(r => (
        <div key={r.label} className="flex items-center justify-between border-b border-white/5 last:border-transparent pb-1 last:pb-0">
          <span className="text-slate-400 text-[11px]">{r.label}</span>
          <span className={`${r.accent ?? 'text-slate-300'} text-[11px]`}>{r.value}</span>
        </div>
      ))}
    </div>
  );
};

const MetadataRenderer = ({ metadata }: { metadata?: Record<string, any> }) => {
  if (!metadata) return null;
  const eventType = metadata.event_type;
  switch (eventType) {
    case 'WaterEvent': return <WaterMetadata meta={metadata} />;
    case 'SystemAlert': return <AlertMetadata meta={metadata} />;
    case 'CalibrationUpdate': return <CalibrationMetadata meta={metadata} />;
    case 'DosingCycleComplete': return <DosingMetadata meta={metadata} />;
    default: return null;
  }
};

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
    return { icon: AlertCircle, iconColor: 'text-rose-400', borderColor: 'border-rose-500/20', bgColor: 'bg-rose-500/5', dot: 'bg-rose-500' };
  }
  if (level === 'warning') {
    return { icon: AlertTriangle, iconColor: 'text-amber-400', borderColor: 'border-amber-500/20', bgColor: 'bg-amber-500/5', dot: 'bg-amber-500' };
  }

  const normCategory = category?.toLowerCase().replace('_', '');

  switch (normCategory) {
    case 'dosing':
      if (title.includes('MIMO') || title.includes('Chu trình') || title.includes('Báo cáo')) {
        return { icon: FlaskConical, iconColor: 'text-cyan-400', borderColor: 'border-cyan-500/20', bgColor: 'bg-cyan-500/5', dot: 'bg-cyan-400' };
      }
      if (title.includes('pH') || title.includes('Điều Chỉnh')) {
        return { icon: Beaker, iconColor: 'text-fuchsia-400', borderColor: 'border-fuchsia-500/20', bgColor: 'bg-fuchsia-500/5', dot: 'bg-fuchsia-400' };
      }
      return { icon: FlaskConical, iconColor: 'text-orange-400', borderColor: 'border-orange-500/20', bgColor: 'bg-orange-500/5', dot: 'bg-orange-400' };

    case 'water':
      return { icon: Waves, iconColor: 'text-blue-400', borderColor: 'border-blue-500/20', bgColor: 'bg-blue-500/5', dot: 'bg-blue-400' };

    case 'calibration':
      return { icon: Settings2, iconColor: 'text-purple-400', borderColor: 'border-purple-500/20', bgColor: 'bg-purple-500/5', dot: 'bg-purple-400' };

    case 'sensor':
      return { icon: Radio, iconColor: 'text-amber-400', borderColor: 'border-amber-500/20', bgColor: 'bg-amber-500/5', dot: 'bg-amber-400' };

    case 'useraction':
      return { icon: UserCheck, iconColor: 'text-indigo-400', borderColor: 'border-indigo-500/20', bgColor: 'bg-indigo-500/5', dot: 'bg-indigo-400' };

    case 'system':
      if (title.includes('Offline') || title.includes('Mất') || title.includes('tắt bơm')) {
        return { icon: Power, iconColor: 'text-slate-400', borderColor: 'border-slate-800', bgColor: 'bg-slate-900/60', dot: 'bg-slate-500' };
      }
      if (title.includes('Trực tuyến') || title.includes('Online') || title.includes('kết nối')) {
        return { icon: Wifi, iconColor: 'text-emerald-400', borderColor: 'border-emerald-500/20', bgColor: 'bg-emerald-500/5', dot: 'bg-emerald-400' };
      }
      return { icon: Cpu, iconColor: 'text-slate-300', borderColor: 'border-slate-800', bgColor: 'bg-slate-900', dot: 'bg-slate-500' };

    default:
      if (level === 'success') {
        return { icon: CheckCircle, iconColor: 'text-emerald-400', borderColor: 'border-emerald-500/20', bgColor: 'bg-emerald-500/5', dot: 'bg-emerald-400' };
      }
      return { icon: Info, iconColor: 'text-indigo-400', borderColor: 'border-slate-800', bgColor: 'bg-slate-900', dot: 'bg-indigo-500' };
  }
};

const FILTERS = [
  { id: 'all', label: 'Tất cả', icon: Filter },
  { id: 'alert', label: 'Cảnh báo', icon: AlertTriangle },
  { id: 'dosing', label: 'MIMO Optimizer', icon: FlaskConical },
  { id: 'water', label: 'Thủy lực', icon: Waves },
  { id: 'user_action', label: 'Người dùng', icon: UserCheck },
  { id: 'system', label: 'Lõi hệ thống', icon: Cpu },
];

const friendlyTitle = (title: string): string => {
  const map: Record<string, string> = {
    'Dừng Khẩn Cấp!': 'Dừng hệ thống khẩn cấp',
    'Lỗi Hệ Thống!': 'Phát hiện sự cố cứng',
    'Hoàn Tất Chu Trình': 'Kích hoạt chu kỳ đa biến MIMO',
    'Cấp Nước': 'Kích hoạt bơm cấp nước bồn',
    'Xả Nước': 'Kích hoạt bơm xả nước thải',
    'Sục Trộn Dinh Dưỡng': 'Khuấy trộn tuần hoàn Osaka',
    'Điều Chỉnh pH': 'Định lượng hóa chất chỉnh pH',
    'Runtime Calibration Tự Động (EMA)': 'Tự cập nhật ma trận hệ số thích ứng',
    'Cập nhật hệ số châm phân động': 'Sửa trọng số Kalman 8 trục',
    'Lưu Báo Cáo Châm Phân Thành Công': 'Chốt mẫu bão hòa chất lưu',
  };
  return map[title] ?? title;
};

const FsmBadge = ({ message }: { message: string }) => {
  const stateMap: Record<string, { label: string; color: string }> = {
    'WaterRefilling': { label: 'Cấp nước', color: 'text-blue-400 bg-blue-500/10 border-blue-500/20' },
    'WaterDraining': { label: 'Xả nước', color: 'text-sky-400 bg-sky-500/10 border-sky-500/20' },
    'DosingPumpA': { label: 'Châm phân A', color: 'text-orange-400 bg-orange-500/10 border-orange-500/20' },
    'DosingPumpB': { label: 'Châm phân B', color: 'text-orange-400 bg-orange-500/10 border-orange-500/20' },
    'DosingPH': { label: 'Chỉnh độ pH', color: 'text-fuchsia-400 bg-fuchsia-500/10 border-fuchsia-500/20' },
    'ActiveMixing': { label: 'Sục khuấy động', color: 'text-purple-400 bg-purple-500/10 border-purple-500/20' },
    'Stabilizing': { label: 'Lắng phẳng bồn', color: 'text-amber-400 bg-amber-500/10 border-amber-500/20' },
    'DosingStabilizing': { label: 'Lắng tĩnh bồn', color: 'text-amber-400 bg-amber-500/10 border-amber-500/20' },
    'Monitoring': { label: 'Giám sát MIMO', color: 'text-slate-400 bg-slate-800 border-slate-700' },
    'Idle': { label: 'Hệ thống nghỉ', color: 'text-slate-400 bg-slate-800 border-slate-700' },
    'EmergencyStop': { label: 'Khóa rơ-le cứng', color: 'text-red-400 bg-red-500/10 border-red-500/20' },
  };
  const matched = stateMap[message];
  if (!matched) return null;
  return (
    <span className={`px-2 py-0.5 rounded text-[10px] font-black uppercase tracking-wider border ${matched.color}`}>
      {matched.label}
    </span>
  );
};

const escapeCsv = (val: any) => {
  if (val == null) return '""';
  const str = String(val);
  return `"${str.replace(/"/g, '""')}"`;
};

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

  const handleExportCSV = async () => {
    if (systemEvents.length === 0) {
      toast.error("Không có dữ liệu hành trình để xuất!");
      return;
    }

    try {
      const headers = ["ID", "Thời Gian", "Mã Thiết Bị", "Cấp Độ", "Danh Mục", "Tiêu Đề", "Nội Dung Message", "Mã Lỗi Hệ Thống", "Cycle ID", "Gói JSON Metadata"];
      const csvRows = systemEvents.map(ev => {
        const date = new Date(ev.timestamp > 1e12 ? ev.timestamp : ev.timestamp * 1000).toLocaleString('vi-VN');
        const displayTitle = friendlyTitle(ev.title);
        const cycleId = ev.metadata?.cycle_id || '';
        const metaString = ev.metadata ? JSON.stringify(ev.metadata) : '';

        return [ev.id || '', date, ev.device_id || '', ev.level || '', ev.category || '', displayTitle || '', ev.message || '', ev.reason || '', cycleId, metaString].map(escapeCsv).join(",");
      });

      const csvContent = "\uFEFF" + [headers.join(","), ...csvRows].join("\n");
      const saved = await saveTextFile(`nhat-ky-he-thong-${deviceId || 'all'}.csv`, csvContent);
      if (saved) toast.success("Đã kết xuất dữ liệu ra file thành công!");
    } catch (err: any) {
      toast.error(err?.message || "Thất bại khi ghi file!");
    }
  };

  return (
    <div className="p-4 md:p-8 max-w-4xl mx-auto pb-28">
      <PageHeader
        icon={Clock}
        title="Nhật Ký Hệ Thống"
        subtitle={`Lịch sử vận hành và chẩn đoán trạm ${deviceId || '—'}`}
      />

      {/* Điều khiển Bộ lọc & Xuất CSV */}
      <div className="bg-slate-900 border border-slate-800 rounded-2xl p-4 mb-8 flex flex-col sm:flex-row justify-between items-start sm:items-center gap-4 relative z-10">
        <div className="flex flex-wrap gap-2">
          {FILTERS.map(btn => {
            const Icon = btn.icon;
            const active = filter === btn.id;
            return (
              <button
                key={btn.id}
                onClick={() => setFilter(btn.id)}
                className={`flex items-center gap-1.5 px-3.5 py-2 rounded-xl text-xs font-black uppercase tracking-wider transition-all duration-200 border
                  ${active
                    ? 'bg-blue-600 text-white border-transparent shadow-[0_0_15px_rgba(59,130,246,0.3)] scale-105'
                    : 'bg-slate-950 text-slate-400 border-white/5 hover:bg-slate-800 hover:text-slate-200'
                  }`}
              >
                <Icon size={13} />
                {btn.label}
              </button>
            );
          })}
        </div>

        <button
          onClick={handleExportCSV}
          disabled={systemEvents.length === 0}
          className="flex items-center justify-center space-x-2 bg-slate-800 hover:bg-slate-700 disabled:opacity-40 text-white px-4 py-2 rounded-xl border border-slate-700 transition-all active:scale-95 shrink-0 font-bold text-xs uppercase tracking-wider"
        >
          <Download size={14} className={systemEvents.length > 0 ? "text-emerald-400" : "text-slate-500"} />
          <span>Xuất CSV</span>
        </button>
      </div>

      {/* Trục Timeline Card */}
      {isLoading ? (
        <div className="flex items-center justify-center gap-3 py-20 text-slate-500">
          <div className="w-5 h-5 border-2 border-slate-700 border-t-blue-500 rounded-full animate-spin" />
          <span className="text-xs font-mono uppercase tracking-widest">Đang kết nối cổng dữ liệu...</span>
        </div>
      ) : systemEvents.length === 0 ? (
        <StateView
          icon={Zap}
          title="Chưa ghi nhận sự kiện"
          description="Hệ thống trống rỗng hoặc chưa khớp dữ liệu biên theo bộ lọc đã chọn."
        />
      ) : (
        <div className="relative pl-4">
          <div className="absolute left-[17px] top-3 bottom-3 w-px bg-gradient-to-b from-slate-800 via-slate-900 to-transparent" />

          <div className="space-y-4">
            {systemEvents.map((ev, idx) => {
              const style = getEventStyle(ev);
              const Icon = style.icon;
              const date = new Date(ev.timestamp > 1e12 ? ev.timestamp : ev.timestamp * 1000);
              const displayTitle = friendlyTitle(ev.title);

              return (
                <div key={ev.id ?? idx} className="relative flex gap-4 animate-in slide-in-from-bottom-3 duration-500" style={{ animationDelay: `${Math.min(idx * 30, 300)}ms`, animationFillMode: 'both' }}>
                  {/* Timeline Dot */}
                  <div className="relative z-10 shrink-0 mt-3.5">
                    <div className={`w-6 h-6 rounded-full border-2 border-slate-950 flex items-center justify-center shadow-md ${style.dot}`}>
                      <Icon size={12} className="text-slate-950" strokeWidth={3} />
                    </div>
                  </div>

                  {/* Log Card */}
                  <div className={`flex-1 min-w-0 border bg-slate-900/40 border-white/5 rounded-2xl p-4 shadow-sm transition-all duration-300 hover:border-slate-700/60 ${style.bgColor} ${style.borderColor}`}>

                    <div className="flex items-start justify-between gap-4 mb-1.5">
                      <div className="flex items-center gap-2 flex-wrap min-w-0">
                        <h4 className={`text-sm font-black tracking-wide leading-tight ${style.iconColor}`}>
                          {displayTitle}
                        </h4>
                        <FsmBadge message={ev.message} />
                      </div>
                      <time className="text-[10px] text-slate-500 font-mono text-right whitespace-nowrap shrink-0 leading-tight">
                        {date.toLocaleTimeString('vi-VN', { hour: '2-digit', minute: '2-digit', second: '2-digit' })}
                        <span className="block font-medium text-slate-600 text-[9px] mt-0.5">
                          {date.toLocaleDateString('vi-VN', { day: '2-digit', month: '2-digit' })}
                        </span>
                      </time>
                    </div>

                    {/* Khối bọc văn bản tin nhắn an toàn chống tràn dòng */}
                    {ev.message && ev.message !== ev.title && !ev.message.startsWith('Monitoring') && ev.level !== 'FSM_UPDATE' && (
                      <p className="text-xs text-slate-300 leading-relaxed font-medium opacity-95 whitespace-pre-line">
                        {ev.message}
                      </p>
                    )}

                    {/* Hiển thị nhanh mã lỗi chẩn đoán nền */}
                    {ev.reason && (
                      <div className="mt-2 flex items-center gap-1.5 text-[9px] text-rose-400 bg-rose-500/5 border border-rose-500/10 rounded-lg px-2 py-1 font-mono self-start max-w-max">
                        <AlertCircle size={10} /> CRITICAL_CODE: {ev.reason}
                      </div>
                    )}

                    {/* Mạch render dữ liệu chẩn đoán động Giai đoạn 5 */}
                    <MetadataRenderer metadata={ev.metadata} />
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
