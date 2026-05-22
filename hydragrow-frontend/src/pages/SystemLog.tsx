import { useEffect, useState } from 'react';
import {
  AlertTriangle, CheckCircle, Info, Filter, Clock, Zap, Waves,
  FlaskConical, AlertCircle, Power, Cpu, Settings2,
  Radio, Wifi, Download, UserCheck, ShieldAlert, ChevronDown, ChevronUp
} from 'lucide-react';
import toast from 'react-hot-toast';
import { PageHeader } from '../components/ui/PageHeader';
import { StateView } from '../components/ui/StateView';
import { httpFetch } from '../platform/http';
import { saveTextFile } from '../platform/file';
import { loadAppSettings } from '../platform/settings';

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

const getMetaNumber = (meta: any, keys: string[]): number | undefined => {
  if (!meta) return undefined;
  for (const key of keys) {
    const val = meta[key];
    if (val != null && !isNaN(Number(val))) return Number(val);
  }
  return undefined;
};

// ─── Renderers giải mã Metadata dạng ngăn chứa mở rộng (Tech Specs) ───

const DosingMetadata = ({ meta }: { meta: any }) => {
  if (!meta) return null;
  const cycleMeta = meta.pre != null ? meta : (meta.dosing_report ?? meta.dosing_data ?? meta);
  const pre = cycleMeta.pre ?? {};
  const post = cycleMeta.post_stable ?? cycleMeta.post_mixing ?? cycleMeta.post ?? {};
  const dose = cycleMeta.dose ?? cycleMeta;
  const target = cycleMeta.target ?? cycleMeta;

  const sections: { title?: string; rows: { label: string; value: string; accent?: string }[] }[] = [];

  const doseRows: { label: string; value: string; accent?: string }[] = [];
  if (dose.pump_a_ml != null && dose.pump_a_ml > 0) doseRows.push({ label: 'Dinh dưỡng A:', value: `${Number(dose.pump_a_ml).toFixed(1)} ml`, accent: 'text-orange-400 font-bold' });
  if (dose.pump_b_ml != null && dose.pump_b_ml > 0) doseRows.push({ label: 'Dinh dưỡng B:', value: `${Number(dose.pump_b_ml).toFixed(1)} ml`, accent: 'text-orange-400 font-bold' });
  if (dose.ph_up_ml != null && dose.ph_up_ml > 0) doseRows.push({ label: 'Thuốc pH Up:', value: `${Number(dose.ph_up_ml).toFixed(1)} ml`, accent: 'text-purple-400 font-bold' });
  if (dose.ph_down_ml != null && dose.ph_down_ml > 0) doseRows.push({ label: 'Thuốc pH Down:', value: `${Number(dose.ph_down_ml).toFixed(1)} ml`, accent: 'text-rose-400 font-bold' });
  if (doseRows.length) sections.push({ title: 'Khối lượng định lượng thực tế', rows: doseRows });

  const deltaRows: { label: string; value: string; accent?: string }[] = [];
  const ecBefore = getMetaNumber(pre, ['ec', 'EC', 'start_ec']);
  const ecAfter = getMetaNumber(post, ['ec', 'EC', 'after_ec', 'post_mixing_ec']);
  const phBefore = getMetaNumber(pre, ['ph', 'pH', 'start_ph']);
  const phAfter = getMetaNumber(post, ['ph', 'pH', 'after_ph', 'post_mixing_ph']);

  if (ecBefore != null && ecAfter != null && ecBefore !== 0.0) {
    const diff = ecAfter - ecBefore;
    deltaRows.push({ label: 'Hành trình sai số EC:', value: `${ecBefore.toFixed(2)} → ${ecAfter.toFixed(2)} (${diff >= 0 ? '+' : ''}${diff.toFixed(2)})`, accent: 'text-cyan-400 font-mono font-bold' });
  }
  if (phBefore != null && phAfter != null && phBefore !== 0.0) {
    const diff = phAfter - phBefore;
    deltaRows.push({ label: 'Hành trình sai số pH:', value: `${phBefore.toFixed(2)} → ${phAfter.toFixed(2)} (${diff >= 0 ? '+' : ''}${diff.toFixed(2)})`, accent: 'text-fuchsia-400 font-mono font-bold' });
  }
  if (deltaRows.length) sections.push({ title: 'Biến động bão hòa cảm biến', rows: deltaRows });

  const targetRows: { label: string; value: string; accent?: string }[] = [];
  const targetEc = getMetaNumber(target, ['ec', 'target_ec']);
  const targetPh = getMetaNumber(target, ['ph', 'target_ph']);
  if (targetEc != null && targetEc > 0) targetRows.push({ label: 'Ngưỡng đặt EC mục tiêu:', value: targetEc.toFixed(2), accent: 'text-cyan-300 font-bold' });
  if (targetPh != null && targetPh > 0) targetRows.push({ label: 'Ngưỡng đặt pH mục tiêu:', value: targetPh.toFixed(2), accent: 'text-fuchsia-300 font-bold' });
  if (cycleMeta.step_ratio_ec != null) targetRows.push({ label: 'AI Kalman Step EC:', value: `${(cycleMeta.step_ratio_ec * 100).toFixed(0)}%`, accent: 'text-teal-400 font-bold' });
  if (cycleMeta.stabilized_window_sec != null) targetRows.push({ label: 'Độ trễ tĩnh bồn tự học:', value: `${cycleMeta.stabilized_window_sec} giây`, accent: 'text-indigo-400 font-bold' });
  if (targetRows.length) sections.push({ title: 'Trí tuệ nhân tạo & Điểm đặt toán học', rows: targetRows });

  if (sections.length === 0) return null;
  return (
    <div className="mt-3 space-y-2 text-xs grid grid-cols-1 sm:grid-cols-2 gap-2">
      {sections.map((sec, idx) => (
        <div key={idx} className="bg-slate-950/60 border border-slate-800/50 rounded-xl px-3 py-2">
          {sec.title && <div className="text-[9px] font-black text-slate-500 mb-1.5 uppercase tracking-wider">{sec.title}</div>}
          <div className="flex flex-col gap-1.5">
            {sec.rows.map(r => (
              <div key={r.label} className="flex items-center justify-between border-b border-white/5 last:border-transparent pb-1 last:pb-0">
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
    rows.push({ label: 'Hành trình mức nước bồn:', value: `${meta.level_before.toFixed(1)}cm → ${meta.level_after.toFixed(1)}cm (${delta >= 0 ? '+' : ''}${delta.toFixed(1)})`, accent: 'text-blue-400 font-bold' });
  }
  if (meta.target_level != null) rows.push({ label: 'Mực nước đích cấu hình:', value: `${meta.target_level} cm`, accent: 'text-sky-300 font-bold' });
  if (meta.duration_sec != null) rows.push({ label: 'Thời gian mở van rơ-le:', value: `${meta.duration_sec} giây`, accent: 'font-mono' });
  if (meta.success != null) rows.push({ label: 'Kết quả chu kỳ thủy lực:', value: meta.success ? 'Thành công' : 'Lỗi Timeout cấp nước', accent: meta.success ? 'text-emerald-400 font-bold' : 'text-rose-400 font-bold' });

  if (rows.length === 0) return <DosingMetadata meta={meta} />;
  return (
    <div className="mt-3 flex flex-col gap-1.5 text-xs font-medium bg-slate-950/60 border border-slate-800/50 rounded-xl px-3 py-2.5">
      <div className="text-[9px] font-black text-slate-500 mb-0.5 uppercase tracking-wider">Thông số chu trình thủy lực</div>
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
  if (meta.alert_type) rows.push({ label: 'Mã thuật toán chẩn đoán:', value: String(meta.alert_type), accent: 'text-rose-400 font-bold font-mono' });
  if (meta.source) rows.push({ label: 'Khối phần cứng phát hiện:', value: String(meta.source), accent: 'text-orange-400 font-medium' });
  if (meta.retry_count != null) rows.push({ label: 'Vòng lặp thử lại châm bù:', value: `${meta.retry_count} lần`, accent: 'text-amber-400 font-mono' });
  if (meta.limit_value != null) rows.push({ label: 'Ngưỡng kịch trần ranh giới:', value: String(meta.limit_value), accent: 'text-rose-400 font-mono' });

  if (rows.length === 0) return <DosingMetadata meta={meta} />;
  return (
    <div className="mt-3 flex flex-col gap-1.5 text-xs font-medium bg-rose-950/10 border border-rose-900/20 rounded-xl px-3 py-2.5">
      <div className="text-[9px] font-black text-rose-400/70 mb-0.5 uppercase tracking-wider flex items-center gap-1"><ShieldAlert size={10} /> Root Cause Diagnostic Snapshot</div>
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
  if (meta.parameter) rows.push({ label: 'Trục ma trận thích ứng:', value: String(meta.parameter), accent: 'text-purple-400 font-bold' });
  if (meta.old_value != null && meta.new_value != null) {
    rows.push({ label: 'Ma trận tiến hóa (Kalman):', value: `${Number(meta.old_value).toFixed(4)} → ${Number(meta.new_value).toFixed(4)}`, accent: 'text-emerald-400 font-mono font-bold' });
  }
  if (meta.skip_reason) rows.push({ label: 'Lý do bộ lọc chặn cập nhật:', value: String(meta.skip_reason), accent: 'text-amber-400 font-medium' });

  if (rows.length === 0) return null;
  return (
    <div className="mt-3 flex flex-col gap-1.5 text-xs font-medium bg-slate-950/60 border border-slate-800/50 rounded-xl px-3 py-2.5">
      <div className="text-[9px] font-black text-slate-500 mb-0.5 uppercase tracking-wider">MIMO Gain Matrix Evolution</div>
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
  switch (metadata.event_type) {
    case 'WaterEvent': return <WaterMetadata meta={metadata} />;
    case 'SystemAlert': return <AlertMetadata meta={metadata} />;
    case 'CalibrationUpdate': return <CalibrationMetadata meta={metadata} />;
    case 'DosingCycleComplete': return <DosingMetadata meta={metadata} />;
    default: return null;
  }
};

const getEventStyle = (event: SystemEvent): EventStyle => {
  const { level, category, title } = event;
  if (level === 'critical' || title.toLowerCase().includes('khẩn cấp') || title.toLowerCase().includes('emergency')) {
    return { icon: AlertCircle, iconColor: 'text-rose-400', borderColor: 'border-rose-500/10', bgColor: 'from-rose-500/5 to-transparent', dot: 'bg-rose-500' };
  }
  if (level === 'warning') {
    return { icon: AlertTriangle, iconColor: 'text-amber-400', borderColor: 'border-amber-500/10', bgColor: 'from-amber-500/5 to-transparent', dot: 'bg-amber-500' };
  }

  switch (category?.toLowerCase().replace('_', '')) {
    case 'dosing': return { icon: FlaskConical, iconColor: 'text-cyan-400', borderColor: 'border-cyan-500/10', bgColor: 'from-cyan-500/5 to-transparent', dot: 'bg-cyan-400' };
    case 'water': return { icon: Waves, iconColor: 'text-blue-400', borderColor: 'border-blue-500/10', bgColor: 'from-blue-500/5 to-transparent', dot: 'bg-blue-400' };
    case 'calibration': return { icon: Settings2, iconColor: 'text-purple-400', borderColor: 'border-purple-500/10', bgColor: 'from-purple-500/5 to-transparent', dot: 'bg-purple-400' };
    case 'sensor': return { icon: Radio, iconColor: 'text-amber-400', borderColor: 'border-amber-500/10', bgColor: 'from-amber-500/5 to-transparent', dot: 'bg-amber-400' };
    case 'useraction': return { icon: UserCheck, iconColor: 'text-indigo-400', borderColor: 'border-indigo-500/10', bgColor: 'from-indigo-500/5 to-transparent', dot: 'bg-indigo-400' };
    case 'system':
      if (title.includes('Offline') || title.includes('Mất') || title.includes('tắt bơm')) {
        return { icon: Power, iconColor: 'text-slate-500', borderColor: 'border-slate-800', bgColor: 'from-slate-900/40 to-transparent', dot: 'bg-slate-600' };
      }
      if (title.includes('Trực tuyến') || title.includes('Online') || title.includes('kết nối')) {
        return { icon: Wifi, iconColor: 'text-emerald-400', borderColor: 'border-emerald-500/10', bgColor: 'from-emerald-500/5 to-transparent', dot: 'bg-emerald-400' };
      }
      return { icon: Cpu, iconColor: 'text-slate-400', borderColor: 'border-slate-800', bgColor: 'from-slate-900/60 to-transparent', dot: 'bg-slate-500' };
    default:
      if (level === 'success') {
        return { icon: CheckCircle, iconColor: 'text-emerald-400', borderColor: 'border-emerald-500/10', bgColor: 'from-emerald-500/5 to-transparent', dot: 'bg-emerald-400' };
      }
      return { icon: Info, iconColor: 'text-indigo-400', borderColor: 'border-slate-800', bgColor: 'from-slate-900/40 to-transparent', dot: 'bg-indigo-500' };
  }
};

const FILTERS = [
  { id: 'all', label: 'Tất cả', icon: Filter },
  { id: 'alert', label: 'Cảnh báo', icon: AlertTriangle },
  { id: 'dosing', label: 'Châm vi chất', icon: FlaskConical },
  { id: 'water', label: 'Thủy lực nước', icon: Waves },
  { id: 'user_action', label: 'Người dùng', icon: UserCheck },
  { id: 'system', label: 'Lõi hệ thống', icon: Cpu },
];

// 🌟 THẾ HỆ 5 CHỈNH SỬA: Chuẩn hóa nhãn pha MIMO đồng bộ phần mềm
const FsmBadge = ({ message }: { message: string }) => {
  const stateMap: Record<string, { label: string; color: string }> = {
    'WaterRefilling': { label: 'Đang cấp nước', color: 'text-blue-400 bg-blue-500/10 border-blue-500/20' },
    'WaterDraining': { label: 'Đang xả nước', color: 'text-sky-400 bg-sky-500/10 border-sky-500/20' },
    'MimoDosing': { label: 'Đang châm MIMO', color: 'text-cyan-400 bg-cyan-500/10 border-cyan-500/20' },
    'ActiveMixing': { label: 'Sục khuấy động', color: 'text-purple-400 bg-purple-500/10 border-purple-500/20' },
    'Stabilizing': { label: 'Lắng bão hòa', color: 'text-amber-400 bg-amber-500/10 border-amber-500/20' },
    'Cooldown': { label: 'Nghỉ dưỡng bồn', color: 'text-amber-400 bg-amber-500/10 border-amber-500/20' },
    'Monitoring': { label: 'Chăm sóc tự động', color: 'text-slate-400 bg-slate-800 border-slate-700' },
    'Idle': { label: 'Hệ thống nghỉ', color: 'text-slate-400 bg-slate-800 border-slate-700' },
    'EmergencyStop': { label: 'Dừng khẩn cấp', color: 'text-red-400 bg-red-500/10 border-red-500/20' },
  };
  const matched = stateMap[message];
  if (!matched) return null;
  return (
    <span className={`px-2 py-0.5 rounded-full text-[10px] font-bold border ${matched.color}`}>
      {matched.label}
    </span>
  );
};

// 🌟 THẾ HỆ 5 REBUILD CARD: Tách nhỏ card dòng nhật ký thành component riêng để tự quản lý toggle thông số kỹ thuật ẩn
const EventLogCard = ({ ev, idx }: { ev: SystemEvent; idx: number }) => {
  const [isExpanded, setIsExpanded] = useState(false);
  const style = getEventStyle(ev);
  const Icon = style.icon;
  const date = new Date(ev.timestamp > 1e12 ? ev.timestamp : ev.timestamp * 1000);

  // Ép ẩn nếu tin nhắn trùng lặp hệ thống thô hoặc trùng tiêu đề
  const hasValidMsg = ev.message && ev.message !== ev.title && !ev.message.startsWith('Monitoring') && ev.level !== 'FSM_UPDATE';
  const hasMetadata = ev.metadata && Object.keys(ev.metadata).length > 0;

  return (
    <div
      className="relative flex gap-4 animate-in slide-in-from-bottom-3 duration-500"
      style={{ animationDelay: `${Math.min(idx * 20, 200)}ms`, animationFillMode: 'both' }}
    >
      {/* Trục dòng thời gian */}
      <div className="relative z-10 shrink-0 mt-3.5">
        <div className={`w-7 h-7 rounded-full border-4 border-slate-950 flex items-center justify-center shadow-md ${style.dot}`}>
          <Icon size={11} className="text-slate-950" strokeWidth={3} />
        </div>
      </div>

      {/* Thẻ Log chính (Consumer Feed UI) */}
      <div className={`flex-1 min-w-0 border bg-gradient-to-r via-slate-900/60 to-transparent border-slate-800/60 rounded-2xl p-4 shadow-sm transition-all duration-300 hover:border-slate-700 ${style.bgColor}`}>
        <div className="flex items-start justify-between gap-4 mb-2">
          <div className="space-y-1 min-w-0">
            {/* Tiêu đề tự nhiên dịch từ Firmware */}
            <h4 className={`text-sm font-bold tracking-tight leading-snug ${style.iconColor}`}>
              {ev.title}
            </h4>
            <div className="flex items-center gap-2 pt-0.5">
              <FsmBadge message={ev.message} />
            </div>
          </div>
          <time className="text-[10px] text-slate-500 font-mono text-right whitespace-nowrap shrink-0 leading-tight">
            {date.toLocaleTimeString('vi-VN', { hour: '2-digit', minute: '2-digit', second: '2-digit' })}
            <span className="block font-semibold text-slate-600 text-[9px] mt-0.5">
              {date.toLocaleDateString('vi-VN', { day: '2-digit', month: '2-digit' })}
            </span>
          </time>
        </div>

        {/* Nội dung bản tin thời sự chính */}
        {hasValidMsg && (
          <p className="text-xs text-slate-300 leading-relaxed font-medium opacity-95">
            {ev.message}
          </p>
        )}

        {/* Mã lỗi nội bộ hệ thống hiển thị dạng tag nhỏ chân trang */}
        {ev.reason && (
          <div className="mt-2 flex items-center gap-1.5 text-[9px] text-rose-400 bg-rose-500/5 border border-rose-500/10 rounded-md px-2 py-0.5 font-mono max-w-max">
            <AlertCircle size={10} /> MÃ SỰ CỐ: {ev.reason}
          </div>
        )}

        {/* NÚT BẤM TIẾT LỘ THÔNG TIN LŨY TIẾN (PROGRESSIVE DISCLOSURE TOGGLE) */}
        {hasMetadata && (
          <div className="mt-2.5 pt-2 border-t border-slate-800/40 flex flex-col items-start">
            <button
              onClick={() => setIsExpanded(!isExpanded)}
              className="flex items-center gap-1 text-[10px] font-bold text-slate-500 hover:text-slate-400 tracking-wide uppercase transition-colors"
            >
              <span>{isExpanded ? 'Thu nhỏ thông số' : 'Xem thông số kỹ thuật'}</span>
              {isExpanded ? <ChevronUp size={12} /> : <ChevronDown size={12} />}
            </button>

            {/* Ngăn chứa lưới toán học ẩn */}
            {isExpanded && <MetadataRenderer metadata={ev.metadata} />}
          </div>
        )}
      </div>
    </div>
  );
};

const escapeCsv = (val: any) => {
  if (val == null) return '""';
  return `"${String(val).replace(/"/g, '""')}"`;
};

interface EventStyle {
  icon: React.ElementType;
  iconColor: string;
  borderColor: string;
  bgColor: string;
  dot: string;
}

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
        const res = await httpFetch(url, { headers: { 'X-API-Key': appConfig.api_key || '' } });
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
        const cycleId = ev.metadata?.cycle_id || '';
        return [ev.id || '', date, ev.device_id || '', ev.level || '', ev.category || '', ev.title || '', ev.message || '', ev.reason || '', cycleId, ev.metadata ? JSON.stringify(ev.metadata) : ''].map(escapeCsv).join(",");
      });
      const csvContent = "\uFEFF" + [headers.join(","), ...csvRows].join("\n");
      const saved = await saveTextFile(`nhat-ky-hanh-trinh-${deviceId || 'all'}.csv`, csvContent);
      if (saved) toast.success("Đã xuất tệp dữ liệu CSV thành công!");
    } catch (err: any) {
      toast.error(err?.message || "Thất bại khi xuất file!");
    }
  };

  return (
    <div className="p-4 md:p-8 max-w-3xl mx-auto pb-28 text-slate-200">
      <PageHeader
        icon={Clock}
        title="Nhật Ký Hành Trình"
        subtitle={`Dòng thời gian vận hành tự động của trạm ${deviceId || '—'}`}
      />

      {/* Điều khiển Bộ lọc & Xuất CSV (Bento Box tinh gọn) */}
      <div className="bg-slate-900/60 border border-slate-800/80 rounded-3xl p-4 mb-8 flex flex-col md:flex-row justify-between items-stretch md:items-center gap-4 relative z-10 backdrop-blur-md">
        <div className="flex flex-wrap gap-1.5 flex-1 min-w-0">
          {FILTERS.map(btn => {
            const Icon = btn.icon;
            const active = filter === btn.id;
            return (
              <button
                key={btn.id}
                onClick={() => setFilter(btn.id)}
                className={`flex items-center gap-1.5 px-3 py-1.5 rounded-xl text-xs font-semibold transition-all duration-200 border whitespace-nowrap
                  ${active
                    ? 'bg-blue-500 text-white border-transparent shadow-md shadow-blue-500/10 scale-102'
                    : 'bg-slate-950 text-slate-400 border-slate-800/80 hover:bg-slate-900 hover:text-slate-200'
                  }`}
              >
                <Icon size={12} />
                {btn.label}
              </button>
            );
          })}
        </div>

        <button
          onClick={handleExportCSV}
          disabled={systemEvents.length === 0}
          className="flex items-center justify-center space-x-2 bg-slate-800 hover:bg-slate-700 disabled:opacity-40 text-white px-4 py-1.5 rounded-xl border border-slate-700 transition-all duration-200 shadow-sm text-xs font-bold shrink-0 active:scale-95"
        >
          <Download size={13} className={systemEvents.length > 0 ? "text-emerald-400" : "text-slate-500"} />
          <span>Xuất tệp</span>
        </button>
      </div>

      {/* Mạch quét dữ liệu thực thời */}
      {isLoading ? (
        <div className="flex items-center justify-center gap-2.5 py-24 text-slate-500">
          <div className="w-4 h-4 border-2 border-slate-800 border-t-blue-500 rounded-full animate-spin" />
          <span className="text-xs font-semibold uppercase tracking-wider text-slate-400">Đang đồng bộ dòng thời gian...</span>
        </div>
      ) : systemEvents.length === 0 ? (
        <StateView
          icon={Zap}
          title="Dòng thời gian trống"
          description="Chưa ghi nhận sự kiện nào khớp với bộ lọc danh mục đã lựa chọn."
        />
      ) : (
        <div className="relative pl-3">
          {/* Đường line kết nối các dot thời gian dọc */}
          <div className="absolute left-[13px] top-4 bottom-4 w-0.5 bg-gradient-to-b from-slate-800 via-slate-900 to-transparent pointer-events-none" />

          <div className="space-y-4">
            {systemEvents.map((ev, idx) => (
              <EventLogCard key={ev.id ?? idx} ev={ev} idx={idx} />
            ))}
          </div>
        </div>
      )}
    </div>
  );
};

export default SystemLog;
