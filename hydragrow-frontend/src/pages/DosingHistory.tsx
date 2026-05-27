import { useState, useEffect } from 'react';
import {
  ShieldCheck, Box,
  Calendar, ChevronDown, ChevronUp, Download,
  FlaskConical, Target, Waves,
  AlertTriangle
} from 'lucide-react';
import toast from 'react-hot-toast';
import { PageHeader } from '../components/ui/PageHeader';
import { StateView } from '../components/ui/StateView';
import { httpFetch } from '../platform/http';
import { saveTextFile } from '../platform/file';
import { loadAppSettings } from '../platform/settings';

// ─── Kiểu dữ liệu mở rộng ──────────────────────────────────────────────────
interface DosingReportRecord {
  id: number;
  device_id: string;
  season_id?: string;
  pump_a_ml: number;
  pump_b_ml: number;
  ph_up_ml: number;
  ph_down_ml: number;
  payload?: {
    dosing_data?: {
      trigger?: string;
      cycle_id?: string;
      pre?: { ec: number; ph: number; water_level: number; temp?: number };
      post_stable?: { ec: number; ph: number; water_level?: number; temp?: number };
      post_mixing?: { ec: number; ph: number };
      target_ec?: number;
      target_ph?: number;
      delta_ec?: number;
      delta_ph?: number;
      error_ec?: number;
      error_ph?: number;
      duration_ms?: number;
      ema_ec_gain_used?: number;
      ema_ph_shift_used?: number;
      step_ratio_ec?: number;
      step_ratio_ph?: number;
      [key: string]: any;
    };
    [key: string]: any;
  };
  created_at: string;
}

interface CropSeason {
  id: string;
  name: string;
  status: 'active' | 'completed';
  start_time: string;
  end_time?: string;
  plant_type?: string;
}

// ─── Helper lấy số từ object meta ──────────────────────────────────────────
const getMetaNumber = (meta: any, keys: string[]): number | undefined => {
  for (const key of keys) {
    const val = meta?.[key];
    if (val != null && !isNaN(Number(val))) return Number(val);
  }
  return undefined;
};

// ─── Lưới thông số kỹ thuật ẩn (Progressive Disclosure) ────────────────────
const AdvancedSpecsGrid = ({ dosing }: { dosing: any }) => {
  const pre = dosing.pre ?? {};
  const post = dosing.post_stable ?? dosing.post_mixing ?? {};
  const rows: { label: string; value: string; accent?: string }[] = [];

  const ecBefore = getMetaNumber(pre, ['ec', 'EC']);
  const ecAfter = getMetaNumber(post, ['ec', 'EC']);
  const phBefore = getMetaNumber(pre, ['ph', 'pH']);
  const phAfter = getMetaNumber(post, ['ph', 'pH']);

  if (ecBefore != null) rows.push({ label: 'EC trước bão hòa', value: ecBefore.toFixed(2), accent: 'text-cyan-700' });
  if (ecAfter != null) rows.push({ label: 'EC sau bão hòa', value: ecAfter.toFixed(2), accent: 'text-cyan-700 font-bold' });
  if (phBefore != null) rows.push({ label: 'pH trước bão hòa', value: phBefore.toFixed(2), accent: 'text-fuchsia-400' });
  if (phAfter != null) rows.push({ label: 'pH sau bão hòa', value: phAfter.toFixed(2), accent: 'text-fuchsia-400 font-bold' });

  if (dosing.target_ec != null) rows.push({ label: 'Ngưỡng EC mục tiêu', value: Number(dosing.target_ec).toFixed(2), accent: 'text-emerald-900' });
  if (dosing.target_ph != null) rows.push({ label: 'Ngưỡng pH mục tiêu', value: Number(dosing.target_ph).toFixed(2), accent: 'text-emerald-900' });
  if (dosing.delta_ec != null) rows.push({ label: 'Biến thiên Δ EC', value: Number(dosing.delta_ec).toFixed(2), accent: 'text-teal-400' });
  if (dosing.delta_ph != null) rows.push({ label: 'Biến thiên Δ pH', value: Number(dosing.delta_ph).toFixed(2), accent: 'text-teal-400' });

  if (dosing.ema_ec_gain_used != null) rows.push({ label: 'Hệ số tăng (Gain) EC', value: Number(dosing.ema_ec_gain_used).toFixed(5), accent: 'text-orange-400 font-mono' });
  if (dosing.ema_ph_shift_used != null) rows.push({ label: 'Hệ số dịch chuyển pH', value: Number(dosing.ema_ph_shift_used).toFixed(5), accent: 'text-orange-400 font-mono' });
  if (dosing.step_ratio_ec != null) rows.push({ label: 'Bước nhảy Kalman EC', value: `${(Number(dosing.step_ratio_ec) * 100).toFixed(0)}%`, accent: 'text-yellow-400' });
  if (dosing.step_ratio_ph != null) rows.push({ label: 'Bước nhảy Kalman pH', value: `${(Number(dosing.step_ratio_ph) * 100).toFixed(0)}%`, accent: 'text-yellow-400' });
  if (dosing.kalman?.matrix_update_count != null) rows.push({ label: 'Số lần cập nhật ma trận', value: String(dosing.kalman.matrix_update_count), accent: 'text-emerald-700 font-mono' });
  if (dosing.kalman?.matrix_is_warm != null) rows.push({ label: 'Trạng thái ma trận', value: dosing.kalman.matrix_is_warm ? 'Đã ổn định' : 'Đang học', accent: 'text-emerald-300' });
  if (dosing.kalman?.adaptive_mixing_sec != null) rows.push({ label: 'Thời gian trộn tự học', value: `${dosing.kalman.adaptive_mixing_sec} giây`, accent: 'text-indigo-300' });
  if (dosing.kalman?.adaptive_stabilize_sec != null) rows.push({ label: 'Thời gian bão hòa tự học', value: `${dosing.kalman.adaptive_stabilize_sec} giây`, accent: 'text-indigo-300' });

  if (rows.length === 0) return null;

  return (
    <div className="mt-3 bg-emerald-50/80 border border-emerald-100 rounded-xl p-3 animate-in slide-in-from-top-2 duration-300">
      <div className="text-[9px] font-black text-emerald-700/75 uppercase tracking-wider mb-2 flex items-center gap-1.5">
        <Target size={12} className="text-indigo-700" />
        Hồ sơ chẩn đoán Toán học MIMO
      </div>
      <div className="grid grid-cols-1 sm:grid-cols-2 gap-x-6 gap-y-1.5 text-[11px]">
        {rows.map(r => (
          <div key={r.label} className="flex items-center justify-between border-b border-white/5 pb-1 last:border-transparent last:pb-0">
            <span className="text-emerald-800/80 font-medium">{r.label}</span>
            <span className={r.accent ?? 'text-emerald-900'}>{r.value}</span>
          </div>
        ))}
      </div>
    </div>
  );
};

// ─── Component Thẻ Chu Kỳ Thông Minh ───────────────────────────────────────
const DosingReportCard = ({ record, index }: { record: DosingReportRecord; index: number }) => {
  const [isExpanded, setIsExpanded] = useState(false);
  const dosing = record.payload?.dosing_data ?? record.payload;
  if (!dosing) return null;

  // Tính toán câu tóm tắt tự động
  const totalNutrient = record.pump_a_ml + record.pump_b_ml;
  const hasNutrient = totalNutrient > 0;
  const hasPhUp = record.ph_up_ml > 0;
  const hasPhDown = record.ph_down_ml > 0;

  let summaryTitle = 'Đã tối ưu hóa môi trường';
  if (hasNutrient && (hasPhUp || hasPhDown)) summaryTitle = 'Đã bổ sung dinh dưỡng & Cân bằng độ pH';
  else if (hasNutrient) summaryTitle = 'Đã châm bổ sung phân bón dinh dưỡng';
  else if (hasPhUp || hasPhDown) summaryTitle = 'Đã cân bằng độ kiềm/axit của nước';
  else if ((dosing.water_in_sec ?? 0) > 0) summaryTitle = 'Đã cấp thêm nước sạch vào bồn';
  else if ((dosing.water_out_sec ?? 0) > 0) summaryTitle = 'Đã kích bơm xả bớt nước thải';

  const date = new Date(record.created_at);

  return (
    <div className="flex items-start space-x-4 animate-in slide-in-from-bottom-4 duration-500" style={{ animationDelay: `${Math.min(index * 40, 400)}ms`, animationFillMode: 'both' }}>

      {/* Nút Timeline (Icon rơ-le) */}
      <div className="shrink-0 mt-3.5 relative z-10">
        <div className={`w-8 h-8 rounded-full border-4 border-white flex items-center justify-center shadow-lg
          ${hasNutrient ? 'bg-orange-500 text-orange-950' :
            (hasPhUp || hasPhDown) ? 'bg-fuchsia-500 text-fuchsia-950' :
              'bg-blue-500 text-blue-950'}`}>
          <FlaskConical size={14} strokeWidth={2.5} />
        </div>
      </div>

      {/* Thẻ nội dung */}
      <div className="flex-1 bg-gradient-to-br from-white to-emerald-50/70 backdrop-blur-md border border-emerald-100 rounded-2xl p-4 hover:border-emerald-200 transition-colors shadow-sm">

        <div className="flex flex-col sm:flex-row sm:items-start justify-between gap-3 mb-2">
          <div className="space-y-1">
            <h4 className="text-emerald-950 font-bold text-sm tracking-wide">
              {summaryTitle}
            </h4>
            <div className="flex flex-wrap items-center gap-2 pt-1 text-xs font-semibold">
              {record.pump_a_ml > 0 && <span className="text-orange-400 bg-orange-500/10 px-2 py-0.5 rounded border border-orange-500/20">A: {record.pump_a_ml.toFixed(1)}ml</span>}
              {record.pump_b_ml > 0 && <span className="text-orange-400 bg-orange-500/10 px-2 py-0.5 rounded border border-orange-500/20">B: {record.pump_b_ml.toFixed(1)}ml</span>}
              {record.ph_up_ml > 0 && <span className="text-purple-700 bg-purple-500/10 px-2 py-0.5 rounded border border-purple-500/20">pH↑: {record.ph_up_ml.toFixed(1)}ml</span>}
              {record.ph_down_ml > 0 && <span className="text-red-700 bg-red-50 px-2 py-0.5 rounded border border-red-200">pH↓: {record.ph_down_ml.toFixed(1)}ml</span>}
              {(dosing.water_in_sec ?? 0) > 0 && <span className="text-blue-700 bg-blue-50 px-2 py-0.5 rounded border border-blue-200 flex items-center gap-1"><Waves size={10} /> Cấp {dosing.water_in_sec?.toFixed(1)}s</span>}
            </div>
          </div>
          <time className="text-[10px] text-emerald-700/75 font-mono text-right whitespace-nowrap shrink-0">
            {date.toLocaleTimeString('vi-VN', { hour: '2-digit', minute: '2-digit' })}
            <span className="block font-medium text-emerald-700/60 mt-0.5">{date.toLocaleDateString('vi-VN', { day: '2-digit', month: '2-digit' })}</span>
          </time>
        </div>

        {/* Nút bấm hiển thị thông số nâng cao */}
        <div className="mt-3 pt-2.5 border-t border-emerald-100">
          <button
            onClick={() => setIsExpanded(!isExpanded)}
            className="flex items-center gap-1.5 text-[10px] font-bold text-emerald-700/75 hover:text-emerald-900 uppercase tracking-wider transition-colors"
          >
            <span>{isExpanded ? 'Ẩn thông số chẩn đoán' : 'Xem thông số chẩn đoán'}</span>
            {isExpanded ? <ChevronUp size={12} /> : <ChevronDown size={12} />}
          </button>

          {isExpanded && <AdvancedSpecsGrid dosing={dosing} />}
        </div>
      </div>
    </div>
  );
};

// ─── Màn Hình Chính ────────────────────────────────────────────────────────
const DosingHistory = () => {
  const [appConfig, setAppConfig] = useState<any>(null);
  const [deviceId, setDeviceId] = useState<string | null>(null);

  const [seasons, setSeasons] = useState<CropSeason[]>([]);
  const [selectedSeason, setSelectedSeason] = useState<string | null>(null);
  const [history, setHistory] = useState<DosingReportRecord[]>([]);

  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const init = async () => {
      try {
        const settings: any = await loadAppSettings();
        if (settings && settings.device_id) {
          setAppConfig(settings);
          setDeviceId(settings.device_id);
          await fetchSeasons(settings.device_id, settings.backend_url, settings.api_key);
        } else {
          setIsLoading(false);
        }
      } catch (err) {
        console.error("Lỗi khi tải cấu hình:", err);
        setIsLoading(false);
      }
    };
    init();
  }, []);

  const fetchSeasons = async (devId: string, backendUrl: string, apiKey: string) => {
    try {
      const url = `${backendUrl}/api/devices/${devId}/seasons`;
      const response = await httpFetch(url, { headers: { 'X-API-Key': apiKey } });
      if (!response.ok) throw new Error("API chưa sẵn sàng");
      const resData = await response.json();
      const actualData = resData.data ? resData.data : resData;
      setSeasons(actualData);
      if (actualData.length > 0) setSelectedSeason(actualData[0].id);
    } catch (err) { console.warn(err); }
  };

  useEffect(() => {
    if (appConfig && selectedSeason) {
      fetchHistory(appConfig.backend_url, appConfig.api_key, selectedSeason);
    }
  }, [selectedSeason, appConfig]);

  const fetchHistory = async (backendUrl: string, apiKey: string, seasonId: string) => {
    setIsLoading(true); setError(null);
    try {
      const url = `${backendUrl}/api/devices/${deviceId}/dosing-reports?season_id=${seasonId}`;
      const response = await httpFetch(url, { headers: { 'X-API-Key': apiKey } });
      if (!response.ok) throw new Error(`Lỗi máy chủ: HTTP ${response.status}`);
      const resData = await response.json();
      setHistory(resData.data ? resData.data : resData);
    } catch (err: any) {
      setError(err.message || "Không thể tải dữ liệu");
    } finally {
      setIsLoading(false);
    }
  };

  const handleExportCSV = async () => {
    if (history.length === 0) return toast.error("Không có dữ liệu để xuất!");
    try {
      const headers = [
        "Mã Thiết Bị", "Mã Mùa Vụ", "Thời Gian", "Phân A (ml)", "Phân B (ml)", "pH Tăng (ml)", "pH Giảm (ml)",
        "pH Trước", "pH Sau", "EC Trước", "EC Sau", "Mục tiêu EC", "Mục tiêu pH",
        "Sai số EC", "Sai số pH", "Δ EC", "Δ pH", "Hệ số Gain EC", "Hệ số Gain pH"
      ];

      const csvRows = history.map(row => {
        const d = row.payload?.dosing_data ?? row.payload ?? {};
        const prePh = d.pre?.ph?.toFixed(2) ?? ''; const postPh = d.post_stable?.ph?.toFixed(2) ?? '';
        const preEc = d.pre?.ec?.toFixed(2) ?? ''; const postEc = d.post_stable?.ec?.toFixed(2) ?? '';

        return [
          row.device_id, row.season_id || '', new Date(row.created_at).toLocaleString('vi-VN'),
          row.pump_a_ml, row.pump_b_ml, row.ph_up_ml, row.ph_down_ml,
          prePh, postPh, preEc, postEc, d.target_ec, d.target_ph,
          d.error_ec, d.error_ph, d.delta_ec, d.delta_ph, d.ema_ec_gain_used, d.ema_ph_shift_used
        ].map(val => `"${val == null ? '' : val}"`).join(",");
      });

      const csvContent = "\uFEFF" + [headers.join(","), ...csvRows].join("\n");
      const saved = await saveTextFile(`lich-su-cham-phan-${selectedSeason || 'tat-ca'}.csv`, csvContent);
      if (saved) toast.success("Đã xuất tệp CSV thành công!");
    } catch (err) { toast.error("Lỗi khi kết xuất file!"); }
  };

  const activeSeasonData = seasons.find(s => s.id === selectedSeason);

  return (
    <div className="p-4 md:p-8 space-y-6 pb-28 max-w-4xl mx-auto text-emerald-950">

      <PageHeader
        icon={ShieldCheck}
        title="Tiến Trình Sinh Trưởng"
        subtitle="Theo dõi chi tiết các lần máy tự động nạp hóa chất cho cây."
      />

      <div className="bg-white/90 border border-emerald-100 rounded-3xl p-4 flex flex-col md:flex-row items-stretch md:items-center justify-between gap-4 backdrop-blur-md relative z-20">
        <div className="relative flex-1 max-w-xs">
          <label className="text-[10px] font-bold text-emerald-700/75 uppercase tracking-widest flex items-center gap-1.5 mb-1.5 ml-1">
            <Calendar size={12} /> Chọn mùa vụ
          </label>
          <div className="relative">
            <select
              value={selectedSeason || ''}
              onChange={(e) => setSelectedSeason(e.target.value)}
              disabled={seasons.length === 0}
              className="w-full bg-white border border-emerald-100 text-emerald-950 text-sm font-semibold rounded-2xl pl-4 pr-10 py-2.5 appearance-none outline-none focus:border-indigo-500 disabled:opacity-50 cursor-pointer shadow-inner"
            >
              {seasons.length === 0 && <option value="">Chưa có dữ liệu vụ mùa</option>}
              {seasons.map(ss => (
                <option key={ss.id} value={ss.id}>{ss.status === 'active' ? '🟢' : '📦'} {ss.name}</option>
              ))}
            </select>
            <ChevronDown className="absolute right-4 top-1/2 -translate-y-1/2 text-emerald-700/75 pointer-events-none" size={16} />
          </div>
        </div>

        <button
          onClick={handleExportCSV}
          disabled={history.length === 0}
          className="flex items-center justify-center gap-2 bg-emerald-100 hover:bg-emerald-200 disabled:opacity-40 text-white px-5 py-2.5 rounded-2xl border border-emerald-200 transition-all font-bold text-xs uppercase tracking-wider shrink-0 mt-auto shadow-sm active:scale-95"
        >
          <Download size={14} className={history.length > 0 ? "text-emerald-700" : "text-emerald-700/75"} />
          <span>Lưu tệp Excel</span>
        </button>
      </div>

      {activeSeasonData && (
        <div className="flex flex-col sm:flex-row items-start sm:items-center justify-between px-5 py-4 bg-indigo-500/5 border border-indigo-500/10 rounded-2xl">
          <div className="flex items-center gap-3">
            <div className="p-2.5 bg-indigo-500/10 rounded-xl">
              <Calendar size={20} className="text-indigo-700" />
            </div>
            <div>
              <div className="flex items-center gap-2 mb-1">
                <p className="text-[10px] text-emerald-700/75 font-bold uppercase tracking-wider">Niên vụ canh tác</p>
                {activeSeasonData.plant_type && (
                  <span className="px-2 py-0.5 bg-emerald-500/10 text-emerald-700 border border-emerald-500/20 rounded-md text-[9px] font-black uppercase">
                    {activeSeasonData.plant_type}
                  </span>
                )}
              </div>
              <p className="text-sm text-emerald-950 font-semibold">
                {new Date(activeSeasonData.start_time).toLocaleDateString('vi-VN')}
                <span className="text-emerald-700/60 mx-1.5">→</span>
                {activeSeasonData.end_time ? new Date(activeSeasonData.end_time).toLocaleDateString('vi-VN') : 'Đang phát triển'}
              </p>
            </div>
          </div>
        </div>
      )}

      {error && <StateView icon={AlertTriangle} title={error} className="animate-in fade-in" />}

      <div className="relative pt-4 pl-1">
        <div className="absolute left-[29px] top-8 bottom-0 w-0.5 bg-gradient-to-b from-slate-800 to-transparent -z-10" />

        {isLoading ? (
          <div className="flex flex-col items-center justify-center gap-3 py-20 text-emerald-700/75">
            <div className="w-5 h-5 border-2 border-emerald-200 border-t-indigo-500 rounded-full animate-spin" />
            <span className="text-xs font-bold tracking-widest uppercase">Đang trích xuất dữ liệu...</span>
          </div>
        ) : history.length === 0 && !error ? (
          <StateView icon={Box} title="Chưa có chu kỳ châm phân nào" description="Hệ thống sẽ tự động ghi nhận khi mô hình MIMO kích hoạt rơ-le hóa chất." />
        ) : (
          <div className="space-y-6">
            {history.map((record, index) => (
              <DosingReportCard key={record.id || index} record={record} index={index} />
            ))}
          </div>
        )}
      </div>

    </div>
  );
};

export default DosingHistory;
