import { useState } from 'react';
import {
  AlertTriangle,
  CheckCircle,
  Info,
  Droplets,
  Filter,
  Clock,
  Zap,
  Power,
  Waves,
  RefreshCw,
  Database,
} from 'lucide-react';
import { useDeviceContext } from '../context/DeviceContext';

const SystemLog = () => {
  const { systemEvents, deviceId } = useDeviceContext();
  const [filter, setFilter] = useState<string>('all');

  const getEventStyle = (level: string, title: string) => {
    if (level === 'critical' || title.includes('Lỗi') || title.includes('Khẩn Cấp'))
      return {
        icon: AlertTriangle,
        color: 'text-rose-400',
        nodeBg: 'bg-rose-500/20 border-rose-400',
        shadow: 'shadow-[0_0_15px_rgba(244,63,94,0.6)]',
        cardBg: 'bg-gradient-to-r from-rose-500/10 to-transparent border-rose-500/20',
      };

    if (level === 'warning' || title.includes('Cảnh Báo'))
      return {
        icon: AlertTriangle,
        color: 'text-amber-400',
        nodeBg: 'bg-amber-500/20 border-amber-400',
        shadow: 'shadow-[0_0_15px_rgba(245,158,11,0.6)]',
        cardBg: 'bg-gradient-to-r from-amber-500/10 to-transparent border-amber-500/20',
      };

    if (title.includes('Cấp Nước') || title.includes('Xả Nước') || title.includes('Súc Rửa'))
      return {
        icon: Waves,
        color: 'text-blue-400',
        nodeBg: 'bg-blue-500/20 border-blue-400',
        shadow: 'shadow-[0_0_15px_rgba(59,130,246,0.6)]',
        cardBg: 'bg-gradient-to-r from-blue-500/10 to-transparent border-blue-500/20',
      };

    if (title.includes('Châm Phân') || title.includes('Điều Chỉnh pH'))
      return {
        icon: Droplets,
        color: 'text-cyan-400',
        nodeBg: 'bg-cyan-500/20 border-cyan-400',
        shadow: 'shadow-[0_0_15px_rgba(6,182,212,0.6)]',
        cardBg: 'bg-gradient-to-r from-cyan-500/10 to-transparent border-cyan-500/20',
      };

    if (title.includes('Sục Trộn'))
      return {
        icon: RefreshCw,
        color: 'text-purple-400',
        nodeBg: 'bg-purple-500/20 border-purple-400',
        shadow: 'shadow-[0_0_15px_rgba(168,85,247,0.6)]',
        cardBg: 'bg-gradient-to-r from-purple-500/10 to-transparent border-purple-500/20',
      };

    if (title.includes('Blockchain'))
      return {
        icon: Database,
        color: 'text-emerald-400',
        nodeBg: 'bg-emerald-500/20 border-emerald-400',
        shadow: 'shadow-[0_0_15px_rgba(16,185,129,0.6)]',
        cardBg: 'bg-gradient-to-r from-emerald-500/10 to-transparent border-emerald-500/20',
      };

    if (title.includes('Khởi Động'))
      return {
        icon: Power,
        color: 'text-emerald-400',
        nodeBg: 'bg-emerald-500/20 border-emerald-400',
        shadow: 'shadow-[0_0_15px_rgba(16,185,129,0.6)]',
        cardBg: 'bg-gradient-to-r from-emerald-500/10 to-transparent border-emerald-500/20',
      };

    if (level === 'success')
      return {
        icon: CheckCircle,
        color: 'text-emerald-400',
        nodeBg: 'bg-emerald-500/20 border-emerald-400',
        shadow: 'shadow-[0_0_15px_rgba(16,185,129,0.6)]',
        cardBg: 'bg-gradient-to-r from-emerald-500/10 to-transparent border-emerald-500/20',
      };

    return {
      icon: Info,
      color: 'text-indigo-400',
      nodeBg: 'bg-indigo-500/20 border-indigo-400',
      shadow: 'shadow-[0_0_15px_rgba(99,102,241,0.6)]',
      cardBg: 'bg-gradient-to-r from-indigo-500/10 to-transparent border-indigo-500/20',
    };
  };

  const filteredEvents = systemEvents.filter((ev) => {
    if (filter === 'all') return true;
    if (filter === 'error') return ev.level === 'critical' || ev.level === 'warning';
    if (filter === 'dosing')
      return (
        ev.title.includes('Châm') ||
        ev.title.includes('pH') ||
        ev.title.includes('Sục Trộn') ||
        ev.title.includes('Blockchain') ||
        ev.title.includes('Chu Trình')
      );
    if (filter === 'water') return ev.title.includes('Nước') || ev.title.includes('Súc Rửa');
    if (filter === 'info')
      return (
        ev.level === 'info' &&
        !ev.title.includes('Châm') &&
        !ev.title.includes('pH') &&
        !ev.title.includes('Nước') &&
        !ev.title.includes('Sục Trộn')
      );
    return true;
  });

  return (
    <div className="p-4 space-y-6 pb-24 min-h-screen">
      <div className="flex flex-col space-y-1 animate-in slide-in-from-top-4 duration-500">
        <h1 className="text-3xl font-extrabold flex items-center gap-3">
          <div className="p-2 bg-emerald-500/10 rounded-xl border border-emerald-500/30 shadow-[0_0_15px_rgba(16,185,129,0.3)]">
            <Clock size={24} className="text-emerald-400" />
          </div>
          <span className="bg-clip-text text-transparent bg-gradient-to-r from-emerald-400 via-cyan-400 to-blue-500">
            Nhật Ký Hệ Thống
          </span>
        </h1>
        <p className="text-sm text-slate-400 ml-1">
          Lịch sử vận hành trạm thủy canh {deviceId || ''}
        </p>
      </div>

      <div className="bg-slate-900/60 backdrop-blur-xl border border-slate-800 rounded-3xl p-4 flex flex-col space-y-3 shadow-lg animate-in fade-in duration-700">
        <label className="text-[10px] uppercase tracking-widest font-bold text-slate-500 flex items-center gap-1.5 ml-1">
          <Filter size={12} /> Bộ lọc thông minh
        </label>
        <div className="flex flex-wrap gap-2">
          <button
            onClick={() => setFilter('all')}
            className={`px-3 py-2 rounded-xl text-xs font-bold transition-all duration-300 ${filter === 'all' ? 'bg-emerald-500 text-slate-950 shadow-[0_0_15px_rgba(16,185,129,0.5)] scale-105' : 'bg-slate-800 text-slate-400 hover:bg-slate-700'}`}
          >
            Tất cả
          </button>
          <button
            onClick={() => setFilter('error')}
            className={`px-3 py-2 rounded-xl text-xs font-bold transition-all duration-300 ${filter === 'error' ? 'bg-rose-500 text-white shadow-[0_0_15px_rgba(244,63,94,0.5)] scale-105' : 'bg-slate-800 text-slate-400 hover:bg-slate-700'}`}
          >
            Lỗi & Cảnh báo
          </button>
          <button
            onClick={() => setFilter('dosing')}
            className={`px-3 py-2 rounded-xl text-xs font-bold transition-all duration-300 ${filter === 'dosing' ? 'bg-cyan-500 text-slate-950 shadow-[0_0_15px_rgba(6,182,212,0.5)] scale-105' : 'bg-slate-800 text-slate-400 hover:bg-slate-700'}`}
          >
            Dinh dưỡng
          </button>
          <button
            onClick={() => setFilter('water')}
            className={`px-3 py-2 rounded-xl text-xs font-bold transition-all duration-300 ${filter === 'water' ? 'bg-blue-500 text-white shadow-[0_0_15px_rgba(59,130,246,0.5)] scale-105' : 'bg-slate-800 text-slate-400 hover:bg-slate-700'}`}
          >
            Nước
          </button>
          <button
            onClick={() => setFilter('info')}
            className={`px-3 py-2 rounded-xl text-xs font-bold transition-all duration-300 ${filter === 'info' ? 'bg-indigo-500 text-white shadow-[0_0_15px_rgba(99,102,241,0.5)] scale-105' : 'bg-slate-800 text-slate-400 hover:bg-slate-700'}`}
          >
            Log khác
          </button>
        </div>
      </div>

      <div className="relative">
        <div className="absolute left-[19px] top-4 bottom-0 w-0.5 bg-gradient-to-b from-slate-700 via-slate-800 to-transparent"></div>

        <div className="space-y-6">
          {filteredEvents.length === 0 ? (
            <div className="pl-12 flex flex-col items-center justify-center py-16 opacity-50 animate-in zoom-in">
              <Zap size={48} className="text-slate-600 mb-3" />
              <p className="text-sm text-slate-400 font-medium">Hệ thống đang chờ lệnh mới...</p>
            </div>
          ) : (
            filteredEvents.map((ev, idx) => {
              const {
                icon: Icon,
                color,
                nodeBg,
                shadow,
                cardBg,
              } = getEventStyle(ev.level, ev.title);
              const date = new Date(ev.timestamp);

              return (
                <div
                  key={idx}
                  className="relative pl-12 group animate-in slide-in-from-bottom-4 fade-in"
                  style={{
                    animationFillMode: 'both',
                    animationDuration: '600ms',
                    animationDelay: `${idx * 50}ms`,
                  }}
                >
                  <div
                    className={`absolute left-2.5 top-3 p-1.5 rounded-full border-2 ${nodeBg} bg-slate-950 ${shadow} transition-all duration-300 group-hover:scale-125 z-10`}
                  >
                    <Icon size={12} className={color} strokeWidth={2.5} />
                  </div>

                  <div
                    className={`flex flex-col space-y-2 p-4 rounded-2xl border ${cardBg} backdrop-blur-md transition-all duration-300 group-hover:translate-x-1 hover:shadow-lg`}
                  >
                    <div className="flex items-start justify-between gap-2">
                      <h4 className={`text-sm font-extrabold tracking-wide ${color}`}>
                        {ev.title}
                      </h4>
                      <span className="text-[10px] text-slate-300 font-bold bg-slate-900/80 border border-slate-700/50 px-2 py-1 rounded-lg whitespace-nowrap shadow-sm">
                        {date.toLocaleTimeString('vi-VN', {
                          hour: '2-digit',
                          minute: '2-digit',
                          second: '2-digit',
                        })}
                      </span>
                    </div>

                    <div className="text-xs text-slate-300 leading-relaxed whitespace-pre-line font-medium opacity-90 group-hover:opacity-100 transition-opacity">
                      {ev.message}
                    </div>

                    {/* 🟢 BỔ SUNG: Hiển thị METADATA dưới dạng Code Block */}
                    {ev.metadata && Object.keys(ev.metadata).length > 0 && (
                      <div className="mt-3 p-3 bg-slate-950/80 rounded-xl border border-slate-800/80 shadow-inner overflow-x-auto custom-scrollbar">
                        <p className="text-[10px] font-black uppercase tracking-widest text-slate-500 mb-2 select-none">
                          Dữ Liệu Đi Kèm (Metadata):
                        </p>
                        <pre className="text-[11px] font-mono text-emerald-400/90 leading-tight">
                          {JSON.stringify(ev.metadata, null, 2)}
                        </pre>
                      </div>
                    )}
                  </div>
                </div>
              );
            })
          )}
        </div>
      </div>
    </div>
  );
};

export default SystemLog;
