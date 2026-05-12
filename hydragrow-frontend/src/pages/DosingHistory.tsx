import { useState, useEffect } from 'react';
import {
  ShieldCheck, Clock, Box,
  AlertTriangle, Settings, Calendar, ChevronDown, Download, Droplet, Activity,
  Zap, FlaskConical, TrendingUp, Target
} from 'lucide-react';
import toast from 'react-hot-toast';
import { PageHeader } from '../components/ui/PageHeader';
import { StateView } from '../components/ui/StateView';
import { LoadingState } from '../components/ui/LoadingState';
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
      cycle_id?: string;          // UUID chu kỳ
      pre?: { ec: number; ph: number; water_level: number };
      post_stable?: { ec: number; ph: number };
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

// ─── Component con hiển thị chi tiết một chu kỳ châm ────────────────────────
const DosingReportDetail = ({ record }: { record: DosingReportRecord }) => {
  const dosing = record.payload?.dosing_data;
  if (!dosing) return null;

  const pre = dosing.pre ?? {};
  const post = dosing.post_stable ?? dosing.post_mixing ?? {};
  const rows: { label: string; value: string; accent?: string }[] = [];

  // --- Thông tin chu kỳ ---
  // Đã ẨN cycle_id khỏi UI
  if (dosing.trigger) rows.push({ label: 'Trigger', value: dosing.trigger.replace(/_/g, ' '), accent: 'text-indigo-300' });
  if (dosing.duration_ms != null) rows.push({ label: 'Thời gian', value: `${(Number(dosing.duration_ms) / 1000).toFixed(1)}s` });

  // --- Chỉ số trước/sau ---
  const ecBefore = getMetaNumber(pre, ['ec', 'EC']);
  const ecAfter = getMetaNumber(post, ['ec', 'EC']);
  const phBefore = getMetaNumber(pre, ['ph', 'pH']);
  const phAfter = getMetaNumber(post, ['ph', 'pH']);
  const waterBefore = getMetaNumber(pre, ['water_level', 'waterLevel']);

  if (ecBefore != null) rows.push({ label: 'EC trước', value: ecBefore.toFixed(2), accent: 'text-cyan-400' });
  if (ecAfter != null) rows.push({ label: 'EC sau', value: ecAfter.toFixed(2), accent: 'text-cyan-400' });
  if (phBefore != null) rows.push({ label: 'pH trước', value: phBefore.toFixed(2), accent: 'text-fuchsia-400' });
  if (phAfter != null) rows.push({ label: 'pH sau', value: phAfter.toFixed(2), accent: 'text-fuchsia-400' });
  if (waterBefore != null) rows.push({ label: 'Mực nước', value: `${waterBefore.toFixed(1)} cm`, accent: 'text-blue-400' });

  // --- Mục tiêu và sai số ---
  if (dosing.target_ec != null) rows.push({ label: 'Mục tiêu EC', value: Number(dosing.target_ec).toFixed(2), accent: 'text-cyan-300' });
  if (dosing.target_ph != null) rows.push({ label: 'Mục tiêu pH', value: Number(dosing.target_ph).toFixed(2), accent: 'text-fuchsia-300' });
  if (dosing.error_ec != null) rows.push({ label: 'Sai số EC', value: Number(dosing.error_ec).toFixed(2), accent: 'text-amber-400' });
  if (dosing.error_ph != null) rows.push({ label: 'Sai số pH', value: Number(dosing.error_ph).toFixed(2), accent: 'text-amber-400' });

  // --- Biến động ---
  if (dosing.delta_ec != null) rows.push({ label: 'Δ EC', value: Number(dosing.delta_ec).toFixed(2), accent: 'text-cyan-300' });
  if (dosing.delta_ph != null) rows.push({ label: 'Δ pH', value: Number(dosing.delta_ph).toFixed(2), accent: 'text-fuchsia-300' });

  // --- Hệ số kỹ thuật ---
  if (dosing.ema_ec_gain_used != null) rows.push({ label: 'EMA EC gain', value: Number(dosing.ema_ec_gain_used).toFixed(5), accent: 'text-cyan-500' });
  if (dosing.ema_ph_shift_used != null) rows.push({ label: 'EMA pH shift', value: Number(dosing.ema_ph_shift_used).toFixed(5), accent: 'text-fuchsia-500' });
  if (dosing.step_ratio_ec != null) rows.push({ label: 'Bước EC', value: Number(dosing.step_ratio_ec).toFixed(2), accent: 'text-yellow-400' });
  if (dosing.step_ratio_ph != null) rows.push({ label: 'Bước pH', value: Number(dosing.step_ratio_ph).toFixed(2), accent: 'text-yellow-400' });

  if (rows.length === 0) return null;

  return (
    <div className="mt-3 p-3 bg-slate-950/50 border border-slate-800 rounded-xl">
      <div className="text-[10px] font-bold text-slate-500 uppercase tracking-wider mb-2 flex items-center gap-1.5">
        <Activity size={12} className="text-indigo-400" />
        Phân tích chu kỳ
      </div>
      <div className="grid grid-cols-2 md:grid-cols-3 gap-x-6 gap-y-1.5 text-xs">
        {rows.map(r => (
          <div key={r.label} className="flex items-baseline gap-1.5">
            <span className="text-slate-500 shrink-0">{r.label}</span>
            <span className={r.accent ?? 'text-slate-300'}>{r.value}</span>
          </div>
        ))}
      </div>
    </div>
  );
};

// ─── Component chính ────────────────────────────────────────────────────────
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
      const response = await httpFetch(url, {
        method: 'GET',
        headers: { 'Content-Type': 'application/json', 'X-API-Key': apiKey }
      });

      if (!response.ok) throw new Error("API chưa sẵn sàng");

      const resData = await response.json();
      const actualData = resData.data ? resData.data : resData;
      setSeasons(actualData);

      if (actualData.length > 0) setSelectedSeason(actualData[0].id);

    } catch (err) {
      console.warn("Lỗi khi tải dữ liệu vụ mùa:", err);
    }
  };

  useEffect(() => {
    if (appConfig && selectedSeason) {
      fetchHistory(appConfig.backend_url, appConfig.api_key, selectedSeason);
    }
  }, [selectedSeason, appConfig]);

  const fetchHistory = async (backendUrl: string, apiKey: string, seasonId: string) => {
    setIsLoading(true);
    setError(null);
    try {
      if (!backendUrl) throw new Error("Chưa cấu hình URL máy chủ.");

      const url = `${backendUrl}/api/devices/${deviceId}/dosing-reports?season_id=${seasonId}`;
      const response = await httpFetch(url, {
        method: 'GET',
        headers: {
          'Content-Type': 'application/json',
          'X-API-Key': apiKey
        }
      });

      if (!response.ok) throw new Error(`Lỗi máy chủ: HTTP ${response.status}`);

      const resData = await response.json();
      const actualData = resData.data ? resData.data : resData;
      setHistory(actualData);
    } catch (err: any) {
      console.error("Lỗi tải nhật ký niêm phong:", err);
      const errMsg = err.message || (typeof err === 'string' ? err : "Không thể tải dữ liệu");
      setError(errMsg);
      toast.error(errMsg);
    } finally {
      setIsLoading(false);
    }
  };

  // ── Xuất CSV mở rộng ──────────────────────────────────────────────────────
  const handleExportCSV = async () => {
    if (history.length === 0) {
      toast.error("Không có dữ liệu để xuất!");
      return;
    }

    try {
      const headers = [
        "ID", "Mã Thiết Bị", "Mã Vụ Mùa", "Cycle ID", "Trigger",
        "Bơm A (ml)", "Bơm B (ml)", "pH Tăng (ml)", "pH Giảm (ml)",
        "pH Trước", "pH Sau", "EC Trước", "EC Sau",
        "Mục tiêu EC", "Mục tiêu pH", "Sai số EC", "Sai số pH",
        "Δ EC", "Δ pH", "Thời gian (ms)",
        "EMA EC gain", "EMA pH shift", "Bước EC", "Bước pH",
        "Thời Gian"
      ];

      const csvRows = history.map(row => {
        const d = row.payload?.dosing_data ?? {};
        // Cycle ID vẫn giữ trong CSV để dev debug
        const cycleId = d.cycle_id || '';
        const trigger = (d.trigger || 'Không rõ').replace(/_/g, ' ');
        const prePh = d.pre?.ph?.toFixed(2) ?? '';
        const postPh = d.post_stable?.ph?.toFixed(2) ?? '';
        const preEc = d.pre?.ec?.toFixed(2) ?? '';
        const postEc = d.post_stable?.ec?.toFixed(2) ?? '';
        const targetEc = d.target_ec?.toFixed(2) ?? '';
        const targetPh = d.target_ph?.toFixed(2) ?? '';
        const errorEc = d.error_ec?.toFixed(2) ?? '';
        const errorPh = d.error_ph?.toFixed(2) ?? '';
        const deltaEc = d.delta_ec?.toFixed(2) ?? '';
        const deltaPh = d.delta_ph?.toFixed(2) ?? '';
        const duration = d.duration_ms ?? '';
        const emaEc = d.ema_ec_gain_used?.toFixed(5) ?? '';
        const emaPh = d.ema_ph_shift_used?.toFixed(5) ?? '';
        const stepEc = d.step_ratio_ec?.toFixed(2) ?? '';
        const stepPh = d.step_ratio_ph?.toFixed(2) ?? '';

        return [
          row.id, row.device_id, row.season_id || '', cycleId, trigger,
          row.pump_a_ml || 0, row.pump_b_ml || 0,
          row.ph_up_ml || 0, row.ph_down_ml || 0,
          prePh, postPh, preEc, postEc,
          targetEc, targetPh, errorEc, errorPh,
          deltaEc, deltaPh, duration,
          emaEc, emaPh, stepEc, stepPh,
          new Date(row.created_at).toLocaleString('vi-VN')
        ].map(val => `"${val}"`).join(",");
      });

      const csvContent = "\uFEFF" + [headers.join(","), ...csvRows].join("\n");
      const saved = await saveTextFile(`nhat-ky-bom-${selectedSeason || 'tat-ca'}.csv`, csvContent);
      if (!saved) return;
      toast.success("Đã lưu file thành công!");
    } catch (err: any) {
      console.error("ERROR SAVE FILE:", err);
      toast.error(err?.message || "Lỗi khi lưu file!");
    }
  };

  const formatDate = (isoString: string) => {
    return new Date(isoString).toLocaleDateString('vi-VN', {
      day: '2-digit', month: '2-digit', year: 'numeric'
    });
  };

  const activeSeasonData = seasons.find(s => s.id === selectedSeason);

  if (isLoading && !selectedSeason && seasons.length === 0) {
    return <LoadingState message="Đang tải dữ liệu..." />;
  }

  if (!deviceId) {
    return (
      <div className="flex flex-col items-center justify-center h-screen space-y-4 p-6 text-center animate-in fade-in bg-slate-950">
        <div className="p-4 bg-slate-900 rounded-full border border-slate-800">
          <Settings size={32} className="text-slate-400" />
        </div>
        <h2 className="text-xl font-bold text-white">Chưa cấu hình thiết bị</h2>
        <p className="text-sm text-slate-400 max-w-xs">
          Vui lòng vào mục Cài đặt để nhập Device ID trước khi xem nhật ký.
        </p>
      </div>
    );
  }

  return (
    <div className="app-page animate-in fade-in slide-in-from-bottom-4 duration-500 pb-24 max-w-4xl mx-auto">

      <div className="ui-card flex flex-col md:flex-row md:items-center justify-between gap-6 border-indigo-500/20">
        <PageHeader
          icon={ShieldCheck}
          title="Nhật ký châm phân & pH"
          className="w-full"
        />

        <div className="flex flex-col sm:flex-row items-end gap-3 shrink-0">
          <button
            onClick={handleExportCSV}
            disabled={history.length === 0}
            className="ui-btn-md flex items-center justify-center space-x-2 bg-slate-800 hover:bg-slate-700 disabled:opacity-50 text-white rounded-2xl transition-all border border-slate-700 active:scale-95 h-[42px]"
            title="Xuất dữ liệu ra Excel"
          >
            <Download size={18} className={history.length > 0 ? "text-emerald-400" : "text-slate-500"} />
            <span className="hidden sm:inline">Xuất CSV</span>
          </button>

          <div className="relative min-w-[220px] w-full sm:w-auto">
            <label className="text-[10px] font-bold text-indigo-400 uppercase tracking-widest mb-1.5 block ml-1 flex items-center gap-1.5">
              <Calendar size={12} /> Mùa vụ
            </label>
            <div className="relative">
              <select
                value={selectedSeason || ''}
                onChange={(e) => setSelectedSeason(e.target.value)}
                disabled={seasons.length === 0}
                className="ui-input h-[42px] bg-slate-950 border-slate-800 hover:border-indigo-500/50 text-white font-semibold rounded-2xl pr-10 appearance-none focus:ring-indigo-500/30 cursor-pointer disabled:opacity-50"
              >
                {seasons.length === 0 && <option value="">Không có dữ liệu</option>}
                {seasons.map(ss => (
                  <option key={ss.id} value={ss.id}>
                    {ss.status === 'active' ? '🟢' : '📦'} {ss.name}
                  </option>
                ))}
              </select>
              <ChevronDown className="absolute right-4 top-1/2 -translate-y-1/2 text-slate-400 pointer-events-none" size={18} />
            </div>
          </div>
        </div>
      </div>

      {activeSeasonData && (
        <div className="flex items-center justify-between px-4 py-3 bg-indigo-500/5 border border-indigo-500/10 rounded-2xl mb-6">
          <div className="flex items-center space-x-3">
            <div className="p-2 bg-indigo-500/10 rounded-lg">
              <Calendar size={18} className="text-indigo-400" />
            </div>
            <div>
              <div className="flex items-center gap-2 mb-0.5">
                <p className="text-[10px] text-slate-500 font-bold uppercase tracking-wider">Thời gian canh tác</p>
                {activeSeasonData.plant_type && (
                  <span className="px-1.5 py-[1px] bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 rounded text-[9px] font-bold uppercase">
                    {activeSeasonData.plant_type}
                  </span>
                )}
              </div>
              <p className="text-sm text-slate-300 font-medium">
                {formatDate(activeSeasonData.start_time)} - {activeSeasonData.end_time ? formatDate(activeSeasonData.end_time) : 'Đang sinh trưởng'}
              </p>
            </div>
          </div>
        </div>
      )}

      {error && <StateView icon={AlertTriangle} variant="error" title={error} className="animate-in fade-in mb-6" />}

      <div className="space-y-6 relative pt-4">
        <div className="absolute left-6 top-8 bottom-0 w-px bg-slate-800 -z-10"></div>

        {isLoading ? (
          <LoadingState fullscreen={false} className="py-8" message="Đang tải dữ liệu..." />
        ) : history.length === 0 && !error ? (
          <StateView icon={Box} title="Chưa có dữ liệu nào được ghi nhận cho mẻ trồng này." className="bg-slate-900/30" />
        ) : (
          history.map((record, index) => {
            const triggerName = (record.payload?.dosing_data?.trigger || 'Không rõ').replace(/_/g, ' ');
            const dosing = record.payload?.dosing_data;

            return (
              <div key={record.id || index} className="flex items-start space-x-4 animate-in slide-in-from-right-4 duration-500" style={{ animationDelay: `${index * 50}ms` }}>
                {/* Timeline dot */}
                <div className="shrink-0">
                  <div className="h-12 w-12 rounded-full bg-slate-900 border-4 border-slate-950 flex items-center justify-center shadow-lg relative z-10">
                    <Droplet size={18} className="text-indigo-400" />
                  </div>
                </div>

                {/* Card nội dung */}
                <div className="flex-1 bg-slate-900/80 backdrop-blur-md border border-slate-800 rounded-2xl p-4 hover:border-indigo-500/40 transition-all hover:shadow-[0_0_20px_rgba(99,102,241,0.1)] group">
                  <div className="flex flex-col sm:flex-row sm:items-start justify-between gap-4">
                    <div className="flex-1">
                      <div className="flex items-center gap-2 mb-1">
                        <h4 className="text-white font-bold text-sm capitalize tracking-wide">
                          {triggerName}
                        </h4>
                        {/* Đã ẨN badge cycle_id ở đây */}
                      </div>

                      <div className="flex items-center space-x-3 text-xs text-slate-400 font-medium">
                        <Clock size={12} className="mr-1.5" />
                        {new Date(record.created_at).toLocaleString('vi-VN', {
                          hour: '2-digit', minute: '2-digit', second: '2-digit',
                          day: '2-digit', month: '2-digit', year: 'numeric'
                        })}
                        {dosing?.duration_ms != null && (
                          <span className="text-slate-500 flex items-center gap-1">
                            <Zap size={12} className="text-yellow-500" />
                            {(Number(dosing.duration_ms) / 1000).toFixed(1)}s
                          </span>
                        )}
                      </div>

                      {/* Liều lượng bơm */}
                      <div className="mt-3 flex flex-wrap gap-2">
                        {record.pump_a_ml > 0 && (
                          <span className="px-2 py-1 bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 rounded text-xs font-semibold flex items-center gap-1">
                            <FlaskConical size={12} /> A: {record.pump_a_ml.toFixed(2)} ml
                          </span>
                        )}
                        {record.pump_b_ml > 0 && (
                          <span className="px-2 py-1 bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 rounded text-xs font-semibold flex items-center gap-1">
                            <FlaskConical size={12} /> B: {record.pump_b_ml.toFixed(2)} ml
                          </span>
                        )}
                        {record.ph_up_ml > 0 && (
                          <span className="px-2 py-1 bg-rose-500/10 text-rose-400 border border-rose-500/20 rounded text-xs font-semibold flex items-center gap-1">
                            <TrendingUp size={12} /> pH↑: {record.ph_up_ml.toFixed(2)} ml
                          </span>
                        )}
                        {record.ph_down_ml > 0 && (
                          <span className="px-2 py-1 bg-cyan-500/10 text-cyan-400 border border-cyan-500/20 rounded text-xs font-semibold flex items-center gap-1">
                            <TrendingUp size={12} className="rotate-180" /> pH↓: {record.ph_down_ml.toFixed(2)} ml
                          </span>
                        )}
                      </div>
                    </div>

                    {/* Tóm tắt chỉ số nhanh */}
                    {(dosing?.pre || dosing?.post_stable || dosing?.target_ec) && (
                      <div className="shrink-0 bg-slate-950/50 rounded-xl p-3 border border-slate-800/50 min-w-[180px]">
                        <div className="flex items-center gap-1.5 mb-2 text-slate-400 text-xs font-semibold uppercase">
                          <Target size={12} className="text-indigo-400" />
                          <span>Chỉ số</span>
                        </div>
                        <div className="grid grid-cols-2 gap-x-4 gap-y-1 text-xs">
                          {dosing.pre?.ec != null && (
                            <>
                              <span className="text-slate-500">EC trước:</span>
                              <span className="text-white font-medium text-right">{dosing.pre.ec.toFixed(2)}</span>
                            </>
                          )}
                          {dosing.post_stable?.ec != null && (
                            <>
                              <span className="text-slate-500">EC sau:</span>
                              <span className="text-white font-medium text-right">{dosing.post_stable.ec.toFixed(2)}</span>
                            </>
                          )}
                          {dosing.pre?.ph != null && (
                            <>
                              <span className="text-slate-500">pH trước:</span>
                              <span className="text-white font-medium text-right">{dosing.pre.ph.toFixed(2)}</span>
                            </>
                          )}
                          {dosing.post_stable?.ph != null && (
                            <>
                              <span className="text-slate-500">pH sau:</span>
                              <span className="text-white font-medium text-right">{dosing.post_stable.ph.toFixed(2)}</span>
                            </>
                          )}
                          {dosing.target_ec != null && (
                            <>
                              <span className="text-slate-500">Mục tiêu EC:</span>
                              <span className="text-cyan-400 font-medium text-right">{Number(dosing.target_ec).toFixed(2)}</span>
                            </>
                          )}
                          {dosing.target_ph != null && (
                            <>
                              <span className="text-slate-500">Mục tiêu pH:</span>
                              <span className="text-fuchsia-400 font-medium text-right">{Number(dosing.target_ph).toFixed(2)}</span>
                            </>
                          )}
                        </div>
                      </div>
                    )}
                  </div>

                  <DosingReportDetail record={record} />
                </div>
              </div>
            );
          })
        )}
      </div>
    </div>
  );
};

export default DosingHistory;
