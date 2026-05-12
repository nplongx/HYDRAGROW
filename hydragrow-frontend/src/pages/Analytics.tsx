import { useState, useEffect, useMemo, useCallback, useRef } from 'react';
import {
  XAxis, YAxis, CartesianGrid, Tooltip,
  ResponsiveContainer, AreaChart, Area
} from 'recharts';
import {
  LineChart as ChartIcon, Clock, Filter,
  Thermometer, Droplets, ActivitySquare, Waves, Timer, Loader2, AlertTriangle, Activity
} from 'lucide-react';
import { useCropSeason } from '../hooks/useCropSeason';
import { PageHeader } from '../components/ui/PageHeader';
import { StateView } from '../components/ui/StateView';
import { loadAppSettings } from '../platform/settings';
import { UnifiedDeviceConfig } from '../types/models';

// Màu sắc Minimalist
const CHART_THEMES: Record<string, any> = {
  cyan: { stroke: '#06b6d4', fill1: '#06b6d4', fill2: '#083344', text: 'text-cyan-400', bg: 'bg-cyan-500/10' },
  fuchsia: { stroke: '#d946ef', fill1: '#d946ef', fill2: '#4a044e', text: 'text-fuchsia-400', bg: 'bg-fuchsia-500/10' },
  orange: { stroke: '#f97316', fill1: '#f97316', fill2: '#431407', text: 'text-orange-400', bg: 'bg-orange-500/10' },
  blue: { stroke: '#3b82f6', fill1: '#3b82f6', fill2: '#172554', text: 'text-blue-400', bg: 'bg-blue-500/10' }
};

// --- FlatChartCard (không đổi) ---
const FlatChartCard = ({ title, data, dataKey, color, unit, icon: Icon }: any) => {
  const theme = CHART_THEMES[color];

  const stats = useMemo(() => {
    if (!data || data.length === 0) return { min: '--', max: '--', avg: '--', current: '--' };
    const values = data.map((d: any) => Number(d[dataKey])).filter((v: number) => !isNaN(v));
    if (values.length === 0) return { min: '--', max: '--', avg: '--', current: '--' };

    return {
      min: Math.min(...values).toFixed(2),
      max: Math.max(...values).toFixed(2),
      avg: (values.reduce((a: number, b: number) => a + b, 0) / values.length).toFixed(2),
      current: values[values.length - 1].toFixed(2)
    };
  }, [data, dataKey]);

  const CustomTooltip = ({ active, payload }: any) => {
    if (active && payload && payload.length) {
      return (
        <div className="bg-slate-900 border border-slate-700 px-3 py-2 rounded-lg shadow-xl">
          <p className="text-slate-400 text-[11px] mb-1 font-medium">
            {payload[0].payload.fullTime}
          </p>
          <p className={`text-base font-semibold ${theme.text}`}>
            {Number(payload[0].value).toFixed(2)} <span className="text-xs opacity-75 font-normal">{unit}</span>
          </p>
        </div>
      );
    }
    return null;
  };

  return (
    <div className="bg-slate-900 border border-slate-800 rounded-xl p-5 transition-colors hover:border-slate-700">
      <div className="flex items-start justify-between mb-4">
        <div className="flex items-center gap-3">
          <div className={`p-2 rounded-lg ${theme.bg}`}>
            <Icon size={18} className={theme.text} strokeWidth={2.5} />
          </div>
          <div>
            <h3 className="text-sm font-semibold text-slate-100">{title}</h3>
            <div className="flex flex-wrap gap-x-3 mt-1 text-[11px] font-medium text-slate-500">
              <p>Hiện tại: <span className="text-slate-200">{stats.current}</span></p>
              <p>TB: <span className="text-slate-200">{stats.avg}</span></p>
              <p>Min: <span className="text-slate-200">{stats.min}</span></p>
              <p>Max: <span className="text-slate-200">{stats.max}</span></p>
            </div>
          </div>
        </div>
      </div>
      <div className="h-[200px] w-full mt-2">
        <ResponsiveContainer width="100%" height="100%">
          <AreaChart data={data} margin={{ top: 5, right: 0, left: -20, bottom: 0 }}>
            <defs>
              <linearGradient id={`gradient-${dataKey}`} x1="0" y1="0" x2="0" y2="1">
                <stop offset="5%" stopColor={theme.fill1} stopOpacity={0.3} />
                <stop offset="95%" stopColor={theme.fill2} stopOpacity={0} />
              </linearGradient>
            </defs>
            <CartesianGrid strokeDasharray="3 3" stroke="rgba(255,255,255,0.05)" vertical={false} />
            <XAxis dataKey="time" stroke="rgba(255,255,255,0.1)" tick={{ fill: '#64748b', fontSize: 10 }} tickLine={false} minTickGap={15} tickMargin={10} />
            <YAxis stroke="rgba(255,255,255,0.1)" tick={{ fill: '#64748b', fontSize: 10 }} tickLine={false} axisLine={false} width={40}
              domain={[
                (dataMin: number) => Math.max(0, Math.floor(Number(dataMin) * 0.9)),
                (dataMax: number) => Math.ceil(Number(dataMax) * 1.1)
              ]}
              allowDecimals={false}
            />
            <Tooltip content={<CustomTooltip />} cursor={{ stroke: 'rgba(255,255,255,0.1)', strokeWidth: 1 }} />
            <Area type="monotone" dataKey={dataKey} stroke={theme.stroke} fill={`url(#gradient-${dataKey})`}
              strokeWidth={2} activeDot={{ r: 5, fill: theme.stroke, stroke: '#0f172a', strokeWidth: 2 }}
              isAnimationActive={data.length < 150} animationDuration={1000}
            />
          </AreaChart>
        </ResponsiveContainer>
      </div>
    </div>
  );
};

// Component hiển thị khi cảm biến bị vô hiệu
const SensorDisabledCard = ({ title, icon: Icon }: { title: string; icon: any }) => (
  <div className="bg-slate-900 border border-slate-800 rounded-xl p-5 flex flex-col items-center justify-center h-[280px]">
    <Icon size={32} className="text-slate-600 mb-3" />
    <h3 className="text-sm font-semibold text-slate-400">{title}</h3>
    <p className="text-xs text-slate-500 mt-1">Cảm biến chưa được kích hoạt</p>
  </div>
);

// Hàm xác định độ phân giải
const getResolutionForTimeRange = (range: string): string | undefined => {
  switch (range) {
    case '24h': return undefined;   // backend limit 2000 điểm
    case '7d': return '5m';
    case '30d': return '1h';
    default: return undefined;
  }
};

// Hàm tạo UTC ISO string
const getUtcIsoString = (date: Date) => date.toISOString();

// Hàm kiểm tra trạng thái cảm biến để render chính xác (chống lỗi Type Mismatch hoặc undefined)
const isSensorEnabled = (val: any, defaultState: boolean = true) => {
  if (val === undefined || val === null) return defaultState;

  // Ép mọi thứ về chuỗi viết thường để so sánh
  const strVal = String(val).toLowerCase().trim();

  if (strVal === 'false' || strVal === '0' || strVal === 'off') return false;
  return true;
};

// --- Component chính Analytics ---
const Analytics = () => {
  const { activeSeason, history } = useCropSeason();

  const [appConfig, setAppConfig] = useState<any>(null);
  const [deviceId, setDeviceId] = useState<string | null>(null);
  const [deviceConfig, setDeviceConfig] = useState<UnifiedDeviceConfig | null>(null);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    const init = async () => {
      try {
        const settings: any = await loadAppSettings();
        if (settings && settings.device_id) {
          setAppConfig(settings);
          setDeviceId(settings.device_id);
          await fetchDeviceConfig(settings.device_id, settings);
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

  const fetchDeviceConfig = async (id: string, settings: any) => {
    try {
      const res = await fetch(`${settings.backend_url}/api/devices/${id}/config`, {
        method: 'GET',
        headers: {
          'X-API-Key': settings.api_key,
          'Cache-Control': 'no-cache', // Chặn cache
          'Pragma': 'no-cache'
        },
        cache: 'no-store' // Ép trình duyệt luôn tải dữ liệu mới nhất
      });
      if (res.ok) {
        const data = await res.json();
        const config = data.data || data;

        console.log("🛠 [Debug Analytics] Cấu hình thiết bị:", config);

        setDeviceConfig(config);
      }
    } catch (e) {
      console.error("Lỗi tải cấu hình thiết bị:", e);
    } finally {
      setIsLoading(false);
    }
  };

  const defaultIntervalSec = useMemo(
    () => (appConfig?.publish_interval ? appConfig.publish_interval / 1000 : 5),
    [appConfig]
  );

  const allSeasons = useMemo(() => {
    const list = [...history];
    if (activeSeason && !list.find(s => s.id === activeSeason.id)) {
      list.unshift(activeSeason);
    }
    return list.sort((a, b) => new Date(b.start_time).getTime() - new Date(a.start_time).getTime());
  }, [activeSeason, history]);

  const [selectedSeasonId, setSelectedSeasonId] = useState<string>('realtime');
  const [timeRange, setTimeRange] = useState<string>('24h');
  const [intervalMode, setIntervalMode] = useState<string>('default');
  const [customIntervalValue, setCustomIntervalValue] = useState<number>(60);
  const [historyData, setHistoryData] = useState<any[]>([]);
  const [isFetching, setIsFetching] = useState(false);
  const [fetchError, setFetchError] = useState<string | null>(null);

  const selectedSeason = useMemo(() => {
    if (selectedSeasonId === 'realtime') return null;
    return allSeasons.find(s => s.id.toString() === selectedSeasonId);
  }, [allSeasons, selectedSeasonId]);

  const abortControllerRef = useRef<AbortController | null>(null);

  const loadHistory = useCallback(async () => {
    if (!deviceId || !appConfig) return;

    if (abortControllerRef.current) {
      abortControllerRef.current.abort();
    }
    const controller = new AbortController();
    abortControllerRef.current = controller;

    setIsFetching(true);
    setFetchError(null);

    let startIso = '';
    let endIso = '';

    if (selectedSeasonId !== 'realtime') {
      if (selectedSeason) {
        startIso = getUtcIsoString(new Date(selectedSeason.start_time));
        endIso = selectedSeason.end_time
          ? getUtcIsoString(new Date(selectedSeason.end_time))
          : getUtcIsoString(new Date());
      }
    } else {
      const now = new Date();
      const diffHours = timeRange === '24h' ? 24 : timeRange === '7d' ? 24 * 7 : 24 * 30;
      const startDate = new Date(now.getTime() - diffHours * 60 * 60 * 1000);
      startIso = getUtcIsoString(startDate);
      endIso = getUtcIsoString(now);
    }

    let resolution: string | undefined;
    if (selectedSeasonId === 'realtime') {
      resolution = getResolutionForTimeRange(timeRange);
    } else {
      if (selectedSeason) {
        const seasonDays = Math.ceil(
          (new Date(endIso).getTime() - new Date(startIso).getTime()) / (1000 * 60 * 60 * 24)
        );
        if (seasonDays > 30) resolution = '1h';
        else if (seasonDays > 7) resolution = '30m';
        else resolution = '5m';
      }
    }

    const fetchWithRetry = async (attempt = 1): Promise<any> => {
      try {
        const params = new URLSearchParams();
        params.append('start', startIso);
        params.append('end', endIso);
        if (resolution) params.append('resolution', resolution);

        const url = `${appConfig.backend_url}/api/devices/${deviceId}/sensors/history?${params.toString()}`;
        console.log(`Fetching history with resolution=${resolution}, attempt=${attempt}`);

        const response = await fetch(url, {
          method: 'GET',
          headers: { 'X-API-Key': appConfig.api_key },
          signal: controller.signal
        });

        if (response.ok) {
          const text = await response.text();
          if (text && text.trim() !== '') {
            const res = JSON.parse(text);
            return res.data || res;
          }
          return [];
        } else if ((response.status === 502 || response.status === 503) && attempt < 3) {
          console.warn(`Lỗi ${response.status}, thử lại lần ${attempt + 1}...`);
          await new Promise(resolve => setTimeout(resolve, 2000));
          return fetchWithRetry(attempt + 1);
        } else {
          throw new Error(`HTTP ${response.status}`);
        }
      } catch (error) {
        if ((error as Error).name === 'AbortError') throw error;
        throw error;
      }
    };

    try {
      const data = await fetchWithRetry();
      const formatted = (data || []).map((d: any) => {
        const dateObj = new Date(d.time);
        return {
          ...d,
          timestamp: dateObj.getTime(),
          fullTime: dateObj.toLocaleString('vi-VN', {
            day: '2-digit', month: '2-digit', year: 'numeric',
            hour: '2-digit', minute: '2-digit', second: '2-digit'
          }),
          time: selectedSeasonId === 'realtime' && timeRange === '24h'
            ? dateObj.toLocaleTimeString('vi-VN', { hour: '2-digit', minute: '2-digit' })
            : dateObj.toLocaleString('vi-VN', { day: '2-digit', month: '2-digit', hour: '2-digit', minute: '2-digit' })
        };
      });
      setHistoryData(formatted);
    } catch (error: any) {
      if (error.name === 'AbortError') { /* bỏ qua */ } else {
        console.error("Fetch history error:", error);
        setHistoryData([]);
        setFetchError(error.message || 'Lỗi không xác định');
      }
    } finally {
      if (abortControllerRef.current === controller) {
        setIsFetching(false);
      }
    }
  }, [deviceId, appConfig, selectedSeasonId, timeRange, selectedSeason]);

  useEffect(() => {
    const timer = setTimeout(loadHistory, 300);
    return () => clearTimeout(timer);
  }, [loadHistory]);

  const effectiveIntervalMs = useMemo(() => {
    let seconds = 0;
    if (intervalMode === 'default') seconds = 0;
    else if (intervalMode === 'custom') seconds = Math.max(customIntervalValue, defaultIntervalSec);
    else seconds = Number(intervalMode);
    return seconds * 1000;
  }, [intervalMode, customIntervalValue, defaultIntervalSec]);

  const displayData = useMemo(() => {
    if (effectiveIntervalMs === 0 || historyData.length === 0) return historyData;
    const filtered = [];
    let lastTime = 0;
    for (let i = 0; i < historyData.length; i++) {
      const currentPoint = historyData[i];
      if (i === 0 || i === historyData.length - 1 || currentPoint.timestamp - lastTime >= effectiveIntervalMs) {
        filtered.push(currentPoint);
        lastTime = currentPoint.timestamp;
      }
    }
    return filtered;
  }, [historyData, effectiveIntervalMs]);

  if (isLoading) {
    return (
      <div className="h-screen flex flex-col items-center justify-center gap-4">
        <Loader2 size={32} className="text-blue-500 animate-spin" />
        <p className="text-sm font-medium text-slate-500">Đang tải cấu hình...</p>
      </div>
    );
  }

  return (
    <div className="p-4 md:p-8 max-w-5xl mx-auto space-y-6 pb-28">
      <PageHeader
        icon={ChartIcon}
        title="Phân Tích"
        subtitle="Theo dõi biến động và khai thác dữ liệu chuỗi thời gian"
      />

      <div className="bg-slate-900 border border-slate-800 rounded-xl p-4 md:p-5">
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          {/* Lọc Mùa Vụ */}
          <div className="flex flex-col gap-1.5">
            <label className="text-xs font-medium text-slate-400 flex items-center gap-1.5 pl-1">
              <Filter size={14} className="text-emerald-500" /> Mùa vụ
            </label>
            <select
              value={selectedSeasonId}
              onChange={(e) => setSelectedSeasonId(e.target.value)}
              className="bg-slate-950 border border-slate-800 text-slate-200 text-sm rounded-lg px-3 py-2.5 outline-none focus:border-emerald-500"
            >
              <option value="realtime">Mùa hiện tại</option>
              {allSeasons.map((s) => (
                <option key={s.id} value={s.id.toString()}>
                  {s.name} {s.end_time ? '(Đã lưu)' : '(Đang chạy)'}
                </option>
              ))}
            </select>
          </div>

          {/* Khung Thời Gian */}
          <div className="flex flex-col gap-1.5">
            <label className="text-xs font-medium text-slate-400 flex items-center gap-1.5 pl-1">
              <Clock size={14} className="text-blue-500" /> Thời gian
            </label>
            <select
              disabled={selectedSeasonId !== 'realtime'}
              value={timeRange}
              onChange={(e) => setTimeRange(e.target.value)}
              className="bg-slate-950 border border-slate-800 text-slate-200 text-sm rounded-lg px-3 py-2.5 outline-none focus:border-blue-500 disabled:opacity-50 disabled:cursor-not-allowed"
            >
              <option value="24h">24 Giờ Qua</option>
              <option value="7d">7 Ngày Qua</option>
            </select>
          </div>

          {/* Tần Suất Lọc */}
          <div className="flex flex-col gap-1.5">
            <label className="text-xs font-medium text-slate-400 flex items-center gap-1.5 pl-1">
              <Timer size={14} className="text-purple-500" /> Tần suất điểm
            </label>
            <div className="flex gap-2">
              <select
                value={intervalMode}
                onChange={(e) => setIntervalMode(e.target.value)}
                className="flex-1 bg-slate-950 border border-slate-800 text-slate-200 text-sm rounded-lg px-3 py-2.5 outline-none focus:border-purple-500"
              >
                <option value="default">Không Lọc (Mặc định)</option>
                <option value="60">1 Phút / Điểm</option>
                <option value="300">5 Phút / Điểm</option>
                <option value="900">15 Phút / Điểm</option>
                <option value="1800">30 Phút / Điểm</option>
                <option value="custom">Tùy chỉnh...</option>
              </select>
              {intervalMode === 'custom' && (
                <div className="relative w-20">
                  <input
                    type="number"
                    min={defaultIntervalSec}
                    value={customIntervalValue}
                    onChange={(e) => setCustomIntervalValue(Number(e.target.value))}
                    className="w-full h-full bg-slate-950 border border-purple-500/50 text-purple-300 text-sm rounded-lg px-2 text-center outline-none focus:border-purple-500"
                    placeholder="giây"
                  />
                </div>
              )}
            </div>
          </div>
        </div>
      </div>

      <div className="pt-2">
        {isFetching ? (
          <div className="h-[40vh] flex flex-col items-center justify-center gap-4">
            <Loader2 size={32} className="text-blue-500 animate-spin" />
            <p className="text-sm font-medium text-slate-500">Đang trích xuất dữ liệu chuỗi thời gian...</p>
          </div>
        ) : fetchError ? (
          <div className="h-[40vh] flex flex-col items-center justify-center gap-4 text-center">
            <AlertTriangle size={32} className="text-amber-500" />
            <p className="text-sm font-medium text-slate-300">Lỗi tải dữ liệu</p>
            <p className="text-xs text-slate-500 max-w-md">{fetchError}</p>
            <button
              onClick={() => loadHistory()}
              className="mt-2 px-4 py-2 bg-blue-500 text-white text-xs rounded-lg hover:bg-blue-600"
            >
              Thử lại
            </button>
          </div>
        ) : displayData.length === 0 ? (
          <StateView
            icon={ActivitySquare}
            title="Dữ liệu trống"
            description="Chưa có bản ghi nào trong khung thời gian này."
            className="h-[40vh]"
          />
        ) : (
          <div className="space-y-6">
            {/* EC Chart */}
            {isSensorEnabled(deviceConfig?.enable_ec_sensor) ? (
              <FlatChartCard title="Chỉ số dinh dưỡng (EC)" data={displayData} dataKey="ec" color="cyan" unit="mS" icon={Activity} />
            ) : (
              <SensorDisabledCard title="Chỉ Số dinh dưỡng (EC)" icon={Activity} />
            )}

            {/* pH Chart */}
            {isSensorEnabled(deviceConfig?.enable_ph_sensor) ? (
              <FlatChartCard title="Chỉ Số cân bằng (pH)" data={displayData} dataKey="ph" color="fuchsia" unit="pH" icon={Droplets} />
            ) : (
              <SensorDisabledCard title="Chỉ Số cân bằng (pH)" icon={Droplets} />
            )}

            {/* Nhiệt độ */}
            {isSensorEnabled(deviceConfig?.enable_temp_sensor) ? (
              <FlatChartCard title="Nhiệt Độ môi trường" data={displayData} dataKey="temp" color="orange" unit="°C" icon={Thermometer} />
            ) : (
              <SensorDisabledCard title="Nhiệt độ môi trường" icon={Thermometer} />
            )}

            {/* Mực nước */}
            {isSensorEnabled(deviceConfig?.enable_water_level_sensor) ? (
              <FlatChartCard title="Mực nước (%)" data={displayData} dataKey="water_level" color="blue" unit="%" icon={Waves} />
            ) : (
              <SensorDisabledCard title="Mực mước (%)" icon={Waves} />
            )}
          </div>
        )}
      </div>
    </div>
  );
};

export default Analytics;
