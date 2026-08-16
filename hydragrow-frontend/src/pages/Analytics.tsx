import { useState, useEffect, useMemo } from 'react';
import {
  XAxis, YAxis, CartesianGrid, Tooltip,
  ResponsiveContainer, AreaChart, Area
} from 'recharts';
import {
  LineChart as ChartIcon, Clock, Filter,
  Thermometer, Droplets, ActivitySquare, Waves, Timer, Loader2, AlertTriangle, Activity
} from 'lucide-react';
import { useQuery } from '@tanstack/react-query';

// --- STORE, HOOKS & UTILS ---
import { useDeviceStore } from '../store/useDeviceStore';
import { useCropSeason } from '../hooks/useCropSeason';
import { PageHeader } from '../components/ui/PageHeader';
import { StateView } from '../components/ui/StateView';
import { httpFetch } from '../platform/http';
import { UnifiedDeviceConfig } from '../types/models';

// --- IMPORT LOGIC ĐÃ BIÊN DỊCH TỪ GLEAM ---
import { should_keep_sample } from '../../gleam_core/build/dev/javascript/gleam_core/analytics.mjs';

// Mức biểu đồ
const CHART_THEMES: Record<string, any> = {
  cyan: { stroke: '#0284c7', fill1: '#0284c7', fill2: '#e0f2fe', text: 'text-cyan-700', bg: 'bg-cyan-50' },
  fuchsia: { stroke: '#c026d3', fill1: '#c026d3', fill2: '#fae8ff', text: 'text-fuchsia-700', bg: 'bg-fuchsia-50' },
  orange: { stroke: '#ea580c', fill1: '#ea580c', fill2: '#ffedd5', text: 'text-orange-700', bg: 'bg-orange-50' },
  blue: { stroke: '#2563eb', fill1: '#2563eb', fill2: '#dbeafe', text: 'text-blue-700', bg: 'bg-blue-50' }
};

// --- FlatChartCard ---
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
        <div className="bg-white border border-emerald-200 px-3 py-2 rounded-lg shadow-xl">
          <p className="text-emerald-800/80 text-[11px] mb-1 font-medium">
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
    <div className="bg-white border border-emerald-100 rounded-xl p-5 transition-colors hover:border-emerald-300 shadow-sm shadow-emerald-950/5">
      <div className="flex items-start justify-between mb-4">
        <div className="flex items-center gap-3">
          <div className={`p-2 rounded-lg ${theme.bg}`}>
            <Icon size={18} className={theme.text} strokeWidth={2.5} />
          </div>
          <div>
            <h3 className="text-sm font-semibold text-emerald-950">{title}</h3>
            <div className="flex flex-wrap gap-x-3 mt-1 text-[11px] font-medium text-emerald-700/75">
              <p>Hiện tại: <span className="text-emerald-950">{stats.current}</span></p>
              <p>TB: <span className="text-emerald-950">{stats.avg}</span></p>
              <p>Min: <span className="text-emerald-950">{stats.min}</span></p>
              <p>Max: <span className="text-emerald-950">{stats.max}</span></p>
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
            <CartesianGrid strokeDasharray="3 3" stroke="rgba(21,128,61,0.12)" vertical={false} />
            <XAxis dataKey="time" stroke="rgba(21,128,61,0.18)" tick={{ fill: '#4b6354', fontSize: 10 }} tickLine={false} minTickGap={15} tickMargin={10} />
            <YAxis stroke="rgba(21,128,61,0.18)" tick={{ fill: '#4b6354', fontSize: 10 }} tickLine={false} axisLine={false} width={40}
              domain={[
                (dataMin: number) => Math.max(0, Math.floor(Number(dataMin) * 0.9)),
                (dataMax: number) => Math.ceil(Number(dataMax) * 1.1)
              ]}
              allowDecimals={false}
            />
            <Tooltip content={<CustomTooltip />} cursor={{ stroke: 'rgba(21,128,61,0.18)', strokeWidth: 1 }} />
            <Area type="monotone" dataKey={dataKey} stroke={theme.stroke} fill={`url(#gradient-${dataKey})`}
              strokeWidth={2} activeDot={{ r: 5, fill: theme.stroke, stroke: '#ffffff', strokeWidth: 2 }}
              isAnimationActive={data.length < 150} animationDuration={1000}
            />
          </AreaChart>
        </ResponsiveContainer>
      </div>
    </div>
  );
};

// Hàm kiểm tra trạng thái cảm biến
const isSensorEnabled = (val: any, defaultState: boolean = true) => {
  if (val === undefined || val === null) return defaultState;
  const strVal = String(val).toLowerCase().trim();
  if (strVal === 'false' || strVal === '0' || strVal === 'off') return false;
  return true;
};

// --- COMPONENT CHÍNH ANALYTICS ---
const Analytics = () => {
  // Lấy state trực tiếp từ Zustand Store
  const deviceId = useDeviceStore((s) => s.deviceId);
  const settings = useDeviceStore((s) => s.settings);

  const { activeSeason, history: seasonHistory } = useCropSeason();

  // Local Filter UI States
  const [selectedSeasonId, setSelectedSeasonId] = useState<string>('realtime');
  const [timeRange, setTimeRange] = useState<string>('24h');
  const [intervalMode, setIntervalMode] = useState<string>('default');
  const [customIntervalValue, setCustomIntervalValue] = useState<number>(60);

  // Danh sách mùa vụ
  const allSeasons = useMemo(() => {
    const list = [...seasonHistory];
    if (activeSeason && !list.find(s => s.id === activeSeason.id)) {
      list.unshift(activeSeason);
    }
    return list.sort((a, b) => new Date(b.start_time).getTime() - new Date(a.start_time).getTime());
  }, [activeSeason, seasonHistory]);

  const selectedSeason = useMemo(() => {
    if (selectedSeasonId === 'realtime') return null;
    return allSeasons.find(s => s.id.toString() === selectedSeasonId);
  }, [allSeasons, selectedSeasonId]);

  // Tự động chuyển TimeRange sang "Tất cả" khi chọn Mùa vụ cũ
  useEffect(() => {
    if (selectedSeasonId !== 'realtime') {
      setTimeRange('all');
    } else if (timeRange === 'all') {
      setTimeRange('24h');
    }
  }, [selectedSeasonId]);

  // 1. Query lấy Unified Device Config thông qua TanStack Query
  const { data: deviceConfig } = useQuery<UnifiedDeviceConfig | null>({
    queryKey: ['device-config-unified', deviceId],
    queryFn: async () => {
      if (!deviceId || !settings?.backend_url) return null;
      const res = await httpFetch(`${settings.backend_url}/api/devices/${deviceId}/config/unified`, {
        headers: {
          'X-API-Key': settings.api_key || '',
          'Cache-Control': 'no-cache'
        }
      });
      if (!res.ok) return null;
      const data = await res.json();
      return {
        ...(data.device_config || {}),
        ...(data.water_config || {}),
        ...(data.safety_config || {}),
        ...(data.dosing_calibration || {}),
        ...(data.sensor_calibration || {})
      } as UnifiedDeviceConfig;
    },
    enabled: Boolean(deviceId && settings?.backend_url),
  });

  // Tính toán khung thời gian ISO & độ phân giải (resolution)
  const { startIso, endIso, resolution } = useMemo(() => {
    const now = new Date();
    let start = '';
    let end = '';

    if (selectedSeasonId !== 'realtime' && selectedSeason) {
      const seasonStart = new Date(selectedSeason.start_time);
      const seasonEnd = selectedSeason.end_time ? new Date(selectedSeason.end_time) : now;
      end = seasonEnd.toISOString();

      if (timeRange === 'all') {
        start = seasonStart.toISOString();
      } else {
        const diffHours = timeRange === '24h' ? 24 : timeRange === '7d' ? 24 * 7 : 24 * 30;
        const computedStart = new Date(seasonEnd.getTime() - diffHours * 60 * 60 * 1000);
        start = (computedStart > seasonStart ? computedStart : seasonStart).toISOString();
      }
    } else {
      end = now.toISOString();
      const diffHours = timeRange === '24h' ? 24 : timeRange === '7d' ? 24 * 7 : 24 * 30;
      start = new Date(now.getTime() - diffHours * 60 * 60 * 1000).toISOString();
    }

    let res: string | undefined;
    if (timeRange === '24h') res = undefined;
    else if (timeRange === '7d') res = '5m';
    else if (timeRange === '30d') res = '1h';
    else if (timeRange === 'all') {
      const days = (new Date(end).getTime() - new Date(start).getTime()) / 86400000;
      if (days > 30) res = '1h';
      else if (days > 7) res = '30m';
      else res = '5m';
    }

    return { startIso: start, endIso: end, resolution: res };
  }, [selectedSeasonId, selectedSeason, timeRange]);

  // 2. Query lấy Lịch sử cảm biến thông qua TanStack Query (Tự động Abort Request khi chuyển tab)
  const {
    data: historyData = [],
    isLoading: isFetching,
    isError,
    error,
    refetch
  } = useQuery({
    queryKey: ['sensor-history', deviceId, selectedSeasonId, timeRange, startIso, endIso, resolution],
    queryFn: async ({ signal }) => {
      if (!deviceId || !settings?.backend_url) return [];
      const params = new URLSearchParams({ start: startIso, end: endIso });
      if (resolution) params.append('resolution', resolution);

      const response = await httpFetch(
        `${settings.backend_url}/api/devices/${deviceId}/sensors/history?${params.toString()}`,
        {
          method: 'GET',
          headers: { 'X-API-Key': settings.api_key || '' },
          signal, // Hỗ trợ ngắt request tự động từ TanStack Query
        }
      );

      if (!response.ok) {
        throw new Error(`HTTP ${response.status}`);
      }

      const text = await response.text();
      if (!text || text.trim() === '') return [];
      const res = JSON.parse(text);
      const rawList = res.data || res || [];

      return rawList.map((d: any) => {
        const dateObj = new Date(d.time);
        return {
          ...d,
          timestamp: dateObj.getTime(),
          fullTime: dateObj.toLocaleString('vi-VN', {
            day: '2-digit', month: '2-digit', year: 'numeric',
            hour: '2-digit', minute: '2-digit', second: '2-digit'
          }),
          time: (selectedSeasonId === 'realtime' && timeRange === '24h') || (timeRange === '24h')
            ? dateObj.toLocaleTimeString('vi-VN', { hour: '2-digit', minute: '2-digit' })
            : dateObj.toLocaleString('vi-VN', { day: '2-digit', month: '2-digit', hour: '2-digit', minute: '2-digit' })
        };
      });
    },
    enabled: Boolean(deviceId && settings?.backend_url),
  });

  // Tần suất lọc dữ liệu (Interval)

  const defaultIntervalSec = Number(settings?.publish_interval) ? Number(settings?.publish_interval) / 1000 : 5;

  const effectiveIntervalMs = useMemo(() => {
    let seconds = 0;
    if (intervalMode === 'default') seconds = 0;
    else if (intervalMode === 'custom') seconds = Math.max(customIntervalValue, defaultIntervalSec);
    else seconds = Number(intervalMode);
    return seconds * 1000;
  }, [intervalMode, customIntervalValue, defaultIntervalSec]);

  // 3. Lọc giảm mật độ điểm dữ liệu (Downsampling) bằng Module Gleam analytics.mjs
  const displayData = useMemo(() => {
    if (effectiveIntervalMs === 0 || historyData.length === 0) return historyData;
    const filtered = [];
    let lastTime = 0;

    for (let i = 0; i < historyData.length; i++) {
      const currentPoint = historyData[i];
      const isFirst = i === 0;
      const isLast = i === historyData.length - 1;

      // Gọi hàm Gleam kiểm tra điều kiện giữ điểm dữ liệu
      const keep = should_keep_sample(
        currentPoint.timestamp,
        lastTime,
        effectiveIntervalMs,
        isFirst,
        isLast
      );

      if (keep) {
        filtered.push(currentPoint);
        lastTime = currentPoint.timestamp;
      }
    }
    return filtered;
  }, [historyData, effectiveIntervalMs]);

  // --- RENDER GIAO DIỆN ---
  return (
    <div className="p-4 md:p-8 max-w-5xl mx-auto space-y-6 pb-28">
      <PageHeader
        icon={ChartIcon}
        title="Phân Tích Dữ Liệu"
        subtitle="Theo dõi biến động và khai thác chuỗi thời gian"
      />

      {/* Bộ Lọc Dashboard */}
      <div className="bg-white border border-emerald-100 rounded-xl p-4 md:p-5">
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          {/* Chọn Mùa Vụ */}
          <div className="flex flex-col gap-1.5">
            <label className="text-xs font-medium text-emerald-800/80 flex items-center gap-1.5 pl-1">
              <Filter size={14} className="text-emerald-500" /> Mùa vụ
            </label>
            <select
              value={selectedSeasonId}
              onChange={(e) => setSelectedSeasonId(e.target.value)}
              className="bg-white border border-emerald-100 text-emerald-950 text-sm rounded-lg px-3 py-2.5 outline-none focus:border-emerald-500"
            >
              <option value="realtime">Mùa hiện tại</option>
              {allSeasons.map((s) => (
                <option key={s.id} value={s.id.toString()}>
                  {s.name} {s.end_time ? '(Đã kết thúc)' : '(Đang chạy)'}
                </option>
              ))}
            </select>
          </div>

          {/* Chọn Khung Thời Gian */}
          <div className="flex flex-col gap-1.5">
            <label className="text-xs font-medium text-emerald-800/80 flex items-center gap-1.5 pl-1">
              <Clock size={14} className="text-blue-500" /> Khung thời gian
            </label>
            <select
              value={timeRange}
              onChange={(e) => setTimeRange(e.target.value)}
              className="bg-white border border-emerald-100 text-emerald-950 text-sm rounded-lg px-3 py-2.5 outline-none focus:border-emerald-600"
            >
              {selectedSeasonId !== 'realtime' && <option value="all">Tất cả</option>}
              <option value="24h">24 Giờ {selectedSeason?.end_time ? 'cuối' : 'vừa qua'}</option>
              <option value="7d">7 Ngày {selectedSeason?.end_time ? 'cuối' : 'vừa qua'}</option>
            </select>
          </div>

          {/* Chọn Tần Suất Lọc */}
          <div className="flex flex-col gap-1.5">
            <label className="text-xs font-medium text-emerald-800/80 flex items-center gap-1.5 pl-1">
              <Timer size={14} className="text-purple-500" /> Tần suất điểm
            </label>
            <div className="flex gap-2">
              <select
                value={intervalMode}
                onChange={(e) => setIntervalMode(e.target.value)}
                className="flex-1 bg-white border border-emerald-100 text-emerald-950 text-sm rounded-lg px-3 py-2.5 outline-none focus:border-purple-500"
              >
                <option value="default">Không Lọc (Mặc định)</option>
                <option value="60">1 Phút / điểm</option>
                <option value="300">5 Phút / điểm</option>
                <option value="900">15 Phút / điểm</option>
                <option value="1800">30 Phút / điểm</option>
                <option value="custom">Tùy chỉnh...</option>
              </select>
              {intervalMode === 'custom' && (
                <div className="relative w-20">
                  <input
                    type="number"
                    min={defaultIntervalSec}
                    value={customIntervalValue}
                    onChange={(e) => setCustomIntervalValue(Number(e.target.value))}
                    className="w-full h-full bg-white border border-purple-500/50 text-purple-950 text-sm rounded-lg px-2 text-center outline-none focus:border-purple-500"
                    placeholder="giây"
                  />
                </div>
              )}
            </div>
          </div>
        </div>
      </div>

      {/* Khu vực hiển thị Biểu đồ */}
      <div className="pt-2">
        {isFetching ? (
          <div className="h-[40vh] flex flex-col items-center justify-center gap-4">
            <Loader2 size={32} className="text-blue-500 animate-spin" />
            <p className="text-sm font-medium text-emerald-700/75">Đang trích xuất chuỗi thời gian...</p>
          </div>
        ) : isError ? (
          <div className="h-[40vh] flex flex-col items-center justify-center gap-4 text-center">
            <AlertTriangle size={32} className="text-amber-500" />
            <p className="text-sm font-medium text-emerald-950">Không thể tải dữ liệu lịch sử</p>
            <p className="text-xs text-emerald-700/75 max-w-md">{(error as Error)?.message || 'Lỗi không xác định'}</p>
            <button
              onClick={() => refetch()}
              className="mt-2 px-4 py-2 bg-blue-600 text-white text-xs rounded-lg hover:bg-blue-700 transition-colors"
            >
              Thử lại
            </button>
          </div>
        ) : displayData.length === 0 ? (
          <StateView
            icon={ActivitySquare}
            title="Chưa có dữ liệu"
            description="Không có ghi nhận nào trong khung thời gian này."
            className="h-[40vh]"
          />
        ) : (
          <div className="space-y-6">
            {/* EC Chart */}
            {isSensorEnabled(deviceConfig?.enable_ec_sensor) && (
              <FlatChartCard title="Chỉ số dinh dưỡng (EC)" data={displayData} dataKey="ec" color="cyan" unit="mS/cm" icon={Activity} />
            )}
            {/* pH Chart */}
            {isSensorEnabled(deviceConfig?.enable_ph_sensor) && (
              <FlatChartCard title="Chỉ số nồng độ (pH)" data={displayData} dataKey="ph" color="fuchsia" unit="pH" icon={Droplets} />
            )}
            {/* Nhiệt độ */}
            {isSensorEnabled(deviceConfig?.enable_temp_sensor) && (
              <FlatChartCard title="Nhiệt độ môi trường" data={displayData} dataKey="temp" color="orange" unit="°C" icon={Thermometer} />
            )}
            {/* Mực nước */}
            {isSensorEnabled(deviceConfig?.enable_water_level_sensor) && (
              <FlatChartCard title="Mực nước bồn" data={displayData} dataKey="water_level" color="blue" unit="%" icon={Waves} />
            )}
          </div>
        )}
      </div>
    </div>
  );
};

export default Analytics;
