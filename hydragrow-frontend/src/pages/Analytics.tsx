import { useState, useEffect, useMemo, useRef } from 'react';
import {
  XAxis, YAxis, CartesianGrid, Tooltip,
  ResponsiveContainer, AreaChart, Area
} from 'recharts';
import { LineChart as ChartIcon, Clock, Filter, Activity, Thermometer, Droplets, Cpu, ActivitySquare, Waves } from 'lucide-react';
import { useDeviceContext } from '../context/DeviceContext';
import { useCropSeason } from '../hooks/useCropSeason';
import { fetch } from '@tauri-apps/plugin-http';
import { PageHeader } from '../components/ui/PageHeader';
import { StateView } from '../components/ui/StateView';

const CHART_THEMES: Record<string, any> = {
  cyan: { stroke: '#22d3ee', fill1: '#06b6d4', fill2: '#164e63', text: 'text-cyan-400', bg: 'bg-cyan-500/10', border: 'border-cyan-500/30', glow: 'shadow-[0_0_15px_rgba(34,211,238,0.2)]' },
  fuchsia: { stroke: '#e879f9', fill1: '#d946ef', fill2: '#701a75', text: 'text-fuchsia-400', bg: 'bg-fuchsia-500/10', border: 'border-fuchsia-500/30', glow: 'shadow-[0_0_15px_rgba(232,121,249,0.2)]' },
  orange: { stroke: '#fb923c', fill1: '#f97316', fill2: '#7c2d12', text: 'text-orange-400', bg: 'bg-orange-500/10', border: 'border-orange-500/30', glow: 'shadow-[0_0_15px_rgba(251,146,60,0.2)]' },
  blue: {
    stroke: '#60a5fa',
    fill1: '#3b82f6',
    fill2: '#1e3a8a',
    text: 'text-blue-400',
    bg: 'bg-blue-500/10',
    border: 'border-blue-500/30',
    glow: 'shadow-[0_0_15px_rgba(96,165,250,0.2)]'
  }
};

// --- Component Thẻ Biểu Đồ 3D ---
const HologramChartCard = ({ title, data, dataKey, color, unit, icon: Icon }: any) => {
  const theme = CHART_THEMES[color];

  // 🟢 Tính toán thông số Min, Max, Avg để dễ theo dõi
  const stats = useMemo(() => {
    if (!data || data.length === 0) return { min: '--', max: '--', avg: '--', current: '--' };
    const values = data.map((d: any) => Number(d[dataKey])).filter((v: number) => !isNaN(v));
    if (values.length === 0) return { min: '--', max: '--', avg: '--', current: '--' };

    return {
      min: Math.min(...values).toFixed(2),
      max: Math.max(...values).toFixed(2),
      avg: (values.reduce((a, b) => a + b, 0) / values.length).toFixed(2),
      current: values[values.length - 1].toFixed(2)
    };
  }, [data, dataKey]);

  // 🟢 Custom Tooltip hiển thị thời gian đầy đủ
  const CustomTooltip = ({ active, payload }: any) => {
    if (active && payload && payload.length) {
      return (
        <div className="bg-slate-900/90 backdrop-blur-md border border-white/10 px-4 py-3 rounded-2xl shadow-2xl">
          <p className="text-slate-400 text-[10px] mb-1 font-bold uppercase tracking-wider">
            {payload[0].payload.fullTime}
          </p>
          <p className={`text-lg font-black ${theme.text}`}>
            {Number(payload[0].value).toFixed(2)} <span className="text-xs opacity-70">{unit}</span>
          </p>
        </div>
      );
    }
    return null;
  };

  return (
    <div className={`relative bg-slate-900/40 backdrop-blur-2xl border border-white/5 rounded-[2rem] p-5 transition-all duration-500 overflow-hidden group hover:border-${color}-500/30 hover:shadow-[0_10px_40px_rgba(0,0,0,0.5)]`}>

      {/* Luồng sáng nền (Glow Background) */}
      <div className={`absolute -top-20 -right-20 w-40 h-40 rounded-full blur-[80px] opacity-30 transition-opacity duration-500 group-hover:opacity-60 bg-${color}-500 pointer-events-none`}></div>

      {/* Header Biểu đồ */}
      <div className="relative z-10 flex items-start justify-between mb-4">
        <div className="flex items-center space-x-3">
          <div className={`p-3 rounded-xl ${theme.bg} ${theme.border} border ${theme.glow}`}>
            <Icon size={20} className={theme.text} />
          </div>
          <div>
            <h3 className={`text-sm font-black tracking-widest uppercase ${theme.text}`}>{title}</h3>

            {/* Thanh thông số Thống kê */}
            <div className="flex flex-wrap gap-x-4 mt-1.5 text-[10px] font-bold tracking-wider">
              <p className="text-slate-500">CUR: <span className="text-slate-200">{stats.current} {unit}</span></p>
              <p className="text-slate-500">AVG: <span className="text-slate-200">{stats.avg} {unit}</span></p>
              <p className="text-slate-500">MIN: <span className="text-slate-200">{stats.min} {unit}</span></p>
              <p className="text-slate-500">MAX: <span className="text-slate-200">{stats.max} {unit}</span></p>
            </div>
          </div>
        </div>
      </div>

      {/* Khu vực vẽ biểu đồ Recharts */}
      <div className="h-[220px] w-full relative z-10 mt-2">
        <ResponsiveContainer width="100%" height="100%">
          <AreaChart data={data} margin={{ top: 5, right: 0, left: -20, bottom: 0 }}>
            <defs>
              <linearGradient id={`gradient-${dataKey}`} x1="0" y1="0" x2="0" y2="1">
                <stop offset="5%" stopColor={theme.fill1} stopOpacity={0.6} />
                <stop offset="95%" stopColor={theme.fill2} stopOpacity={0} />
              </linearGradient>
              <filter id={`glow-${dataKey}`} x="-20%" y="-20%" width="140%" height="140%">
                <feGaussianBlur stdDeviation="4" result="blur" />
                <feMerge>
                  <feMergeNode in="blur" />
                  <feMergeNode in="SourceGraphic" />
                </feMerge>
              </filter>
            </defs>
            <CartesianGrid strokeDasharray="3 3" stroke="rgba(255,255,255,0.05)" vertical={false} />

            {/* 🟢 Hiển thị lại trục X và Y để dễ quan sát số liệu */}
            <XAxis
              dataKey="time"
              stroke="rgba(255,255,255,0.1)"
              tick={{ fill: '#64748b', fontSize: 10, fontWeight: 'bold' }}
              tickLine={false}
              minTickGap={30}
            />
            <YAxis
              stroke="rgba(255,255,255,0.1)"
              tick={{ fill: '#64748b', fontSize: 10, fontWeight: 'bold' }}
              tickLine={false}
              axisLine={false}
              domain={['auto', 'auto']}
              width={45}
            />

            <Tooltip content={<CustomTooltip />} />

            <Area
              type="monotone"
              dataKey={dataKey}
              stroke={theme.stroke}
              fill={`url(#gradient-${dataKey})`}
              strokeWidth={3}
              activeDot={{ r: 6, fill: theme.stroke, stroke: '#0f172a', strokeWidth: 3, filter: `url(#glow-${dataKey})` }}
              filter={`url(#glow-${dataKey})`}
              animationDuration={1500}
            />
          </AreaChart>
        </ResponsiveContainer>
      </div>
    </div>
  );
};

const Analytics = () => {
  const { deviceId, settings } = useDeviceContext();
  const { activeSeason, history } = useCropSeason();

  const allSeasons = useMemo(() => {
    const list = [...history];
    if (activeSeason && !list.find(s => s.id === activeSeason.id)) {
      list.unshift(activeSeason);
    }
    return list.sort((a, b) => new Date(b.start_time).getTime() - new Date(a.start_time).getTime());
  }, [activeSeason, history]);

  const [selectedSeasonId, setSelectedSeasonId] = useState<string>('realtime');
  const [timeRange, setTimeRange] = useState<string>('24h');
  const [historyData, setHistoryData] = useState<any[]>([]);
  const [isFetching, setIsFetching] = useState(false);

  // 🟢 Dùng Ref cho allSeasons để tránh kích hoạt useEffect re-render vô tận
  const allSeasonsRef = useRef(allSeasons);
  useEffect(() => {
    allSeasonsRef.current = allSeasons;
  }, [allSeasons]);

  useEffect(() => {
    const loadHistory = async () => {
      if (!deviceId || !settings) return;
      setIsFetching(true);

      let start: string;
      let end = new Date().toISOString();

      if (selectedSeasonId !== 'realtime') {
        const season = allSeasonsRef.current.find(s => s.id.toString() === selectedSeasonId);
        if (season) {
          start = season.start_time;
          end = season.end_time || new Date().toISOString();
        } else {
          start = new Date(Date.now() - 24 * 60 * 60 * 1000).toISOString();
        }
      } else {
        const now = Date.now();
        const diff = timeRange === '24h' ? 24 : timeRange === '7d' ? 24 * 7 : 24 * 30;
        start = new Date(now - diff * 60 * 60 * 1000).toISOString();
      }

      try {
        const url = `${settings.backend_url}/api/devices/${deviceId}/sensors/history?start=${start}&end=${end}`;
        const response = await fetch(url, { method: 'GET', headers: { 'X-API-Key': settings.api_key } });

        if (response.ok) {
          const text = await response.text();
          if (text && text.trim() !== '') {
            const res = JSON.parse(text);
            const formatted = (res.data || res).map((d: any) => {
              const dateObj = new Date(d.time);
              return {
                ...d,
                // 🟢 Lưu thêm fullTime cho tooltip
                fullTime: dateObj.toLocaleString('vi-VN', { day: '2-digit', month: '2-digit', year: 'numeric', hour: '2-digit', minute: '2-digit' }),
                time: selectedSeasonId === 'realtime' && timeRange === '24h'
                  ? dateObj.toLocaleTimeString('vi-VN', { hour: '2-digit', minute: '2-digit' })
                  : dateObj.toLocaleDateString('vi-VN', { day: '2-digit', month: '2-digit' })
              };
            });
            setHistoryData(formatted);
          } else {
            setHistoryData([]);
          }
        }
      } catch (error) {
        console.error("Lỗi fetch lịch sử:", error);
      } finally {
        setIsFetching(false);
      }
    };

    // 🟢 Áp dụng Debounce 300ms để tránh gọi API nhiều lần khi trang mới load
    const timer = setTimeout(() => {
      loadHistory();
    }, 300);

    return () => clearTimeout(timer);

    // Loại bỏ allSeasons khỏi dependency để dừng việc trigger vòng lặp
  }, [selectedSeasonId, timeRange, deviceId, settings?.backend_url, settings?.api_key]);

  return (
    <div className="app-page pb-32 relative">

      {/* Hiệu ứng nền Mesh Gradient */}
      <div className="absolute top-0 right-0 w-[60%] h-64 bg-gradient-to-bl from-cyan-500/10 via-transparent to-transparent pointer-events-none blur-3xl"></div>

      {/* HEADER */}
      <PageHeader
        className="animate-in slide-in-from-top-4 duration-500 mb-6"
        icon={ChartIcon}
        title="PHÂN TÍCH"
        subtitle="Khai thác dữ liệu chuỗi thời gian (Time-series)"
      />

      {/* BỘ LỌC TÌM KIẾM (NEON FILTER BAR) */}
      <div className="relative z-10 ui-card animate-in fade-in duration-700">
        <div className="grid grid-cols-2 gap-4">

          {/* Lọc Mùa Vụ */}
          <div className="space-y-2">
            <label className="text-[10px] font-black uppercase tracking-widest text-slate-500 flex items-center gap-1.5 ml-1">
              <Filter size={12} className="text-emerald-500" /> Nguồn Dữ Liệu
            </label>
            <div className="relative">
              <select
                value={selectedSeasonId}
                onChange={(e) => setSelectedSeasonId(e.target.value)}
                className="w-full bg-slate-950/50 border border-slate-700 text-slate-200 text-xs font-bold tracking-wide rounded-xl py-3 pl-4 pr-8 focus:ring-2 focus:ring-emerald-500 outline-none appearance-none shadow-inner transition-all hover:border-slate-500 cursor-pointer"
              >
                <option value="realtime">⚡ THỜI GIAN THỰC</option>
                {allSeasons.map((s) => (
                  <option key={s.id} value={s.id.toString()}>
                    {s.name} {s.end_time ? '(Lưu trữ)' : '(Đang chạy)'}
                  </option>
                ))}
              </select>
              <div className="absolute right-3 top-1/2 -translate-y-1/2 pointer-events-none">
                <div className="w-2 h-2 bg-emerald-500 rounded-full animate-pulse shadow-[0_0_8px_#10b981]"></div>
              </div>
            </div>
          </div>

          {/* Lọc Thời Gian */}
          <div className="space-y-2">
            <label className="text-[10px] font-black uppercase tracking-widest text-slate-500 flex items-center gap-1.5 ml-1">
              <Clock size={12} className="text-blue-500" /> Khung Thời Gian
            </label>
            <div className="relative">
              <select
                disabled={selectedSeasonId !== 'realtime'}
                value={timeRange}
                onChange={(e) => setTimeRange(e.target.value)}
                className="w-full bg-slate-950/50 border border-slate-700 text-slate-200 text-xs font-bold tracking-wide rounded-xl py-3 pl-4 pr-8 focus:ring-2 focus:ring-blue-500 outline-none appearance-none shadow-inner transition-all hover:border-slate-500 cursor-pointer disabled:opacity-30 disabled:cursor-not-allowed"
              >
                <option value="24h">24 GIỜ QUA</option>
                <option value="7d">7 NGÀY QUA</option>
                <option value="30d">30 NGÀY QUA</option>
              </select>
            </div>
          </div>

        </div>
      </div>

      {/* TRẠNG THÁI HIỂN THỊ DỮ LIỆU */}
      <div className="relative z-10 pt-2">
        {isFetching ? (
          // Loading Hologram
          <div className="h-[40vh] flex flex-col items-center justify-center space-y-6">
            <div className="relative w-24 h-24 flex items-center justify-center">
              <div className="absolute inset-0 rounded-full border-t-2 border-cyan-500 animate-[spin_2s_linear_infinite] shadow-[0_0_15px_rgba(6,182,212,0.5)]"></div>
              <div className="absolute inset-2 rounded-full border-r-2 border-blue-500 animate-[spin_3s_linear_infinite_reverse] shadow-[0_0_15px_rgba(59,130,246,0.5)]"></div>
              <Cpu size={28} className="text-cyan-400 animate-pulse" />
            </div>
            <p className="text-cyan-500/70 font-black tracking-widest text-[10px] uppercase animate-pulse">Đang trích xuất chuỗi thời gian...</p>
          </div>
        ) : historyData.length === 0 ? (
          // Empty State
          <StateView
            icon={ActivitySquare}
            title="Dữ liệu trống rỗng"
            description="Chưa có bản ghi nào trong khung thời gian này."
            className="h-[40vh] flex flex-col justify-center bg-slate-900/20"
          />
        ) : (
          // DANH SÁCH BIỂU ĐỒ 3D
          <div className="space-y-6">
            <div className="animate-in slide-in-from-bottom-8 fade-in duration-700 fill-mode-both" style={{ animationDelay: '0ms' }}>
              <HologramChartCard title="Mật Độ Dinh Dưỡng (EC)" data={historyData} dataKey="ec" color="cyan" unit="mS" icon={Activity} />
            </div>

            <div className="animate-in slide-in-from-bottom-8 fade-in duration-700 fill-mode-both" style={{ animationDelay: '150ms' }}>
              <HologramChartCard title="Chỉ Số Cân Bằng (pH)" data={historyData} dataKey="ph" color="fuchsia" unit="pH" icon={Droplets} />
            </div>

            <div className="animate-in slide-in-from-bottom-8 fade-in duration-700 fill-mode-both" style={{ animationDelay: '300ms' }}>
              <HologramChartCard title="Nhiệt Độ Môi Trường" data={historyData} dataKey="temp" color="orange" unit="°C" icon={Thermometer} />
            </div>

            <div className="animate-in slide-in-from-bottom-8 fade-in duration-700 fill-mode-both" style={{ animationDelay: '450ms' }}>
              <HologramChartCard title="Mực nước" data={historyData} dataKey="water_level" color="blue" unit="cm" icon={Waves} />
            </div>
          </div>
        )}
      </div>

    </div>
  );
};

export default Analytics;
