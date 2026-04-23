import { useState, useEffect, useMemo } from 'react';
import {
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  AreaChart,
  Area,
} from 'recharts';
import {
  LineChart as ChartIcon,
  Clock,
  Filter,
  Activity,
  Thermometer,
  Droplets,
  Cpu,
  ActivitySquare,
  Waves,
} from 'lucide-react';
import { useDeviceContext } from '../context/DeviceContext';
import { useCropSeason } from '../hooks/useCropSeason';
import { fetch } from '@tauri-apps/plugin-http';

const CHART_THEMES: Record<string, any> = {
  cyan: {
    stroke: '#22d3ee',
    fill1: '#06b6d4',
    fill2: '#164e63',
    text: 'text-cyan-400',
    bg: 'bg-cyan-500/10',
    border: 'border-cyan-500/30',
    glow: 'shadow-[0_0_15px_rgba(34,211,238,0.2)]',
  },
  fuchsia: {
    stroke: '#e879f9',
    fill1: '#d946ef',
    fill2: '#701a75',
    text: 'text-fuchsia-400',
    bg: 'bg-fuchsia-500/10',
    border: 'border-fuchsia-500/30',
    glow: 'shadow-[0_0_15px_rgba(232,121,249,0.2)]',
  },
  orange: {
    stroke: '#fb923c',
    fill1: '#f97316',
    fill2: '#7c2d12',
    text: 'text-orange-400',
    bg: 'bg-orange-500/10',
    border: 'border-orange-500/30',
    glow: 'shadow-[0_0_15px_rgba(251,146,60,0.2)]',
  },
  blue: {
    stroke: '#60a5fa',
    fill1: '#3b82f6',
    fill2: '#1e3a8a',
    text: 'text-blue-400',
    bg: 'bg-blue-500/10',
    border: 'border-blue-500/30',
    glow: 'shadow-[0_0_15px_rgba(96,165,250,0.2)]',
  },
};

// --- Component Thẻ Biểu Đồ 3D ---
const HologramChartCard = ({ title, data, dataKey, color, unit, icon: Icon }: any) => {
  const theme = CHART_THEMES[color];

  return (
    <div
      className={`relative bg-slate-900/40 backdrop-blur-2xl border border-white/5 rounded-[2rem] p-5 transition-all duration-500 overflow-hidden group hover:border-${color}-500/30 hover:shadow-[0_10px_40px_rgba(0,0,0,0.5)]`}
    >
      {/* Luồng sáng nền (Glow Background) */}
      <div
        className={`absolute -top-20 -right-20 w-40 h-40 rounded-full blur-[80px] opacity-30 transition-opacity duration-500 group-hover:opacity-60 bg-${color}-500 pointer-events-none`}
      ></div>

      {/* Header Biểu đồ */}
      <div className="relative z-10 flex items-center justify-between mb-6">
        <div className="flex items-center space-x-3">
          <div className={`p-2.5 rounded-xl ${theme.bg} ${theme.border} border ${theme.glow}`}>
            <Icon size={18} className={theme.text} />
          </div>
          <div>
            <h3 className={`text-sm font-black tracking-widest uppercase ${theme.text}`}>
              {title}
            </h3>
            {/* Hiển thị giá trị hiện tại (lấy điểm data cuối cùng nếu có) */}
            <p className="text-[10px] text-slate-400 font-bold mt-0.5">
              CURRENT:{' '}
              <span className="text-slate-200">
                {data.length > 0 ? Number(data[data.length - 1][dataKey]).toFixed(2) : '--'} {unit}
              </span>
            </p>
          </div>
        </div>
      </div>

      {/* Khu vực vẽ biểu đồ Recharts */}
      <div className="h-[180px] w-full relative z-10">
        <ResponsiveContainer width="100%" height="100%">
          <AreaChart data={data} margin={{ top: 5, right: 0, left: 0, bottom: 0 }}>
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
            <XAxis dataKey="time" hide />
            <YAxis hide domain={['auto', 'auto']} />
            <Tooltip
              contentStyle={{
                backgroundColor: 'rgba(15, 23, 42, 0.85)',
                backdropFilter: 'blur(12px)',
                borderColor: 'rgba(255,255,255,0.1)',
                borderRadius: '16px',
                boxShadow: '0 10px 25px rgba(0,0,0,0.5)',
                color: '#fff',
                fontWeight: 'bold',
              }}
              itemStyle={{ color: theme.stroke, fontWeight: 900 }}
              labelStyle={{ color: '#94a3b8', fontSize: '12px', marginBottom: '4px' }}
            />
            <Area
              type="monotone"
              dataKey={dataKey}
              stroke={theme.stroke}
              fill={`url(#gradient-${dataKey})`}
              strokeWidth={3}
              activeDot={{
                r: 6,
                fill: theme.stroke,
                stroke: '#0f172a',
                strokeWidth: 3,
                filter: `url(#glow-${dataKey})`,
              }}
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
    if (activeSeason && !list.find((s) => s.id === activeSeason.id)) {
      list.unshift(activeSeason);
    }
    return list.sort((a, b) => new Date(b.start_time).getTime() - new Date(a.start_time).getTime());
  }, [activeSeason, history]);

  const [selectedSeasonId, setSelectedSeasonId] = useState<string>('realtime');
  const [timeRange, setTimeRange] = useState<string>('24h');
  const [historyData, setHistoryData] = useState<any[]>([]);
  const [isFetching, setIsFetching] = useState(false);

  useEffect(() => {
    const loadHistory = async () => {
      if (!deviceId || !settings) return;
      setIsFetching(true);

      let start: string;
      let end = new Date().toISOString();

      if (selectedSeasonId !== 'realtime') {
        const season = allSeasons.find((s) => s.id.toString() === selectedSeasonId);
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
        const response = await fetch(url, {
          method: 'GET',
          headers: { 'X-API-Key': settings.api_key },
        });

        if (response.ok) {
          const text = await response.text();
          if (text && text.trim() !== '') {
            const res = JSON.parse(text);
            const formatted = (res.data || res).map((d: any) => {
              const dateObj = new Date(d.time);
              return {
                ...d,
                time:
                  selectedSeasonId === 'realtime' && timeRange === '24h'
                    ? dateObj.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
                    : dateObj.toLocaleDateString('vi-VN', { day: '2-digit', month: '2-digit' }),
              };
            });
            setHistoryData(formatted);
          } else {
            setHistoryData([]);
          }
        }
      } catch (error) {
        console.error('Lỗi fetch lịch sử:', error);
      } finally {
        setIsFetching(false);
      }
    };

    loadHistory();
  }, [selectedSeasonId, timeRange, deviceId, settings, allSeasons]);

  return (
    <div className="p-4 space-y-6 pb-32 min-h-screen relative">
      {/* 🟢 Hiệu ứng nền Mesh Gradient */}
      <div className="absolute top-0 right-0 w-[60%] h-64 bg-gradient-to-bl from-cyan-500/10 via-transparent to-transparent pointer-events-none blur-3xl"></div>

      {/* HEADER */}
      <div className="relative z-10 flex flex-col space-y-1 animate-in slide-in-from-top-4 duration-500 mb-6">
        <h1 className="text-3xl font-black flex items-center gap-3">
          <div className="p-2.5 bg-slate-800/50 backdrop-blur-md rounded-2xl border border-slate-700 shadow-[0_0_20px_rgba(6,182,212,0.15)]">
            <ChartIcon size={24} className="text-cyan-400" />
          </div>
          <span className="bg-clip-text text-transparent bg-gradient-to-r from-cyan-400 to-blue-500 tracking-tight">
            PHÂN TÍCH
          </span>
        </h1>
        <p className="text-xs text-slate-400 ml-[52px] font-medium tracking-wide uppercase">
          Khai thác dữ liệu chuỗi thời gian (Time-series)
        </p>
      </div>

      {/* BỘ LỌC TÌM KIẾM (NEON FILTER BAR) */}
      <div className="relative z-10 bg-slate-900/60 backdrop-blur-xl border border-white/5 rounded-3xl p-4 shadow-lg animate-in fade-in duration-700">
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
            <p className="text-cyan-500/70 font-black tracking-widest text-[10px] uppercase animate-pulse">
              Đang trích xuất chuỗi thời gian...
            </p>
          </div>
        ) : historyData.length === 0 ? (
          // Empty State
          <div className="h-[40vh] flex flex-col items-center justify-center border border-dashed border-slate-700/50 rounded-[2rem] bg-slate-900/20 backdrop-blur-sm">
            <ActivitySquare className="text-slate-700 mb-4" size={56} strokeWidth={1} />
            <p className="text-slate-500 text-xs font-bold uppercase tracking-widest">
              Dữ liệu trống rỗng
            </p>
            <p className="text-slate-600 text-[10px] mt-2">
              Chưa có bản ghi nào trong khung thời gian này.
            </p>
          </div>
        ) : (
          // 🟢 DANH SÁCH BIỂU ĐỒ 3D
          <div className="space-y-6">
            {/* Thêm style để stagger animation trượt lên tuần tự */}
            <div
              className="animate-in slide-in-from-bottom-8 fade-in duration-700 fill-mode-both"
              style={{ animationDelay: '0ms' }}
            >
              <HologramChartCard
                title="Mật Độ Dinh Dưỡng (EC)"
                data={historyData}
                dataKey="ec_value"
                color="cyan"
                unit="mS"
                icon={Activity}
              />
            </div>

            <div
              className="animate-in slide-in-from-bottom-8 fade-in duration-700 fill-mode-both"
              style={{ animationDelay: '150ms' }}
            >
              <HologramChartCard
                title="Chỉ Số Cân Bằng (pH)"
                data={historyData}
                dataKey="ph_value"
                color="fuchsia"
                unit="pH"
                icon={Droplets}
              />
            </div>

            <div
              className="animate-in slide-in-from-bottom-8 fade-in duration-700 fill-mode-both"
              style={{ animationDelay: '300ms' }}
            >
              <HologramChartCard
                title="Nhiệt Độ Môi Trường"
                data={historyData}
                dataKey="temp_value"
                color="orange"
                unit="°C"
                icon={Thermometer}
              />
            </div>

            <div
              className="animate-in slide-in-from-bottom-8 fade-in duration-700 fill-mode-both"
              style={{ animationDelay: '300ms' }}
            >
              <HologramChartCard
                title="Mực nước"
                data={historyData}
                dataKey="water_level"
                color="blue"
                unit="cm"
                icon={Waves}
              />
            </div>
          </div>
        )}
      </div>
    </div>
  );
};

export default Analytics;
