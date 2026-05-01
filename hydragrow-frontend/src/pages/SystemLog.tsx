import { useState } from 'react';
import {
  AlertTriangle, CheckCircle, Info, Droplets,
  Filter, Clock, Zap, Power, Waves, RefreshCw, Database
} from 'lucide-react';
import { useDeviceContext } from '../context/DeviceContext';
import { PageHeader } from '../components/ui/PageHeader';
import { StateView } from '../components/ui/StateView';

const SystemLog = () => {
  const { systemEvents, deviceId } = useDeviceContext();
  const [filter, setFilter] = useState<string>('all');

  // Đã chuyển sang hệ màu Flat, không còn shadow hay gradient
  const getEventStyle = (level: string, title: string) => {
    if (level === 'critical' || title.includes('Lỗi') || title.includes('Khẩn Cấp')) return {
      icon: AlertTriangle, color: 'text-red-500',
      iconBorder: 'border-red-500/30', cardBg: 'bg-red-500/5 border-red-500/20'
    };

    if (level === 'warning' || title.includes('Cảnh Báo')) return {
      icon: AlertTriangle, color: 'text-amber-500',
      iconBorder: 'border-amber-500/30', cardBg: 'bg-amber-500/5 border-amber-500/20'
    };

    if (title.includes('Cấp Nước') || title.includes('Xả Nước') || title.includes('Súc Rửa')) return {
      icon: Waves, color: 'text-blue-500',
      iconBorder: 'border-blue-500/30', cardBg: 'bg-blue-500/5 border-blue-500/20'
    };

    if (title.includes('Châm Phân') || title.includes('Điều Chỉnh pH')) return {
      icon: Droplets, color: 'text-fuchsia-500',
      iconBorder: 'border-fuchsia-500/30', cardBg: 'bg-fuchsia-500/5 border-fuchsia-500/20'
    };

    if (title.includes('Sục Trộn')) return {
      icon: RefreshCw, color: 'text-purple-500',
      iconBorder: 'border-purple-500/30', cardBg: 'bg-purple-500/5 border-purple-500/20'
    };

    if (title.includes('Blockchain')) return {
      icon: Database, color: 'text-emerald-500',
      iconBorder: 'border-emerald-500/30', cardBg: 'bg-emerald-500/5 border-emerald-500/20'
    };

    if (title.includes('Khởi Động') || level === 'success') return {
      icon: level === 'success' ? CheckCircle : Power, color: 'text-emerald-500',
      iconBorder: 'border-emerald-500/30', cardBg: 'bg-emerald-500/5 border-emerald-500/20'
    };

    return {
      icon: Info, color: 'text-slate-400',
      iconBorder: 'border-slate-700', cardBg: 'bg-slate-900 border-slate-800'
    };
  };

  const filteredEvents = systemEvents.filter(ev => {
    if (filter === 'all') return true;
    if (filter === 'error') return ev.level === 'critical' || ev.level === 'warning';
    if (filter === 'dosing') return ev.title.includes('Châm') || ev.title.includes('pH') || ev.title.includes('Sục Trộn') || ev.title.includes('Blockchain') || ev.title.includes('Chu Trình');
    if (filter === 'water') return ev.title.includes('Nước') || ev.title.includes('Súc Rửa');
    if (filter === 'info') return ev.level === 'info' && !ev.title.includes('Châm') && !ev.title.includes('pH') && !ev.title.includes('Nước') && !ev.title.includes('Sục Trộn');
    return true;
  });

  return (
    <div className="p-4 md:p-8 max-w-4xl mx-auto pb-28">
      <PageHeader
        icon={Clock}
        title="Nhật Ký Hệ Thống"
        subtitle={`Lịch sử vận hành trạm thủy canh ${deviceId || ''}`}
      />

      <div className="bg-slate-900 border border-slate-800 rounded-xl p-4 md:p-5 mb-8">
        <label className="text-xs font-semibold text-slate-500 flex items-center gap-1.5 mb-3">
          <Filter size={14} /> Bộ lọc sự kiện
        </label>
        <div className="flex flex-wrap gap-2">
          {[
            { id: 'all', label: 'Tất cả' },
            { id: 'error', label: 'Lỗi & Cảnh báo' },
            { id: 'dosing', label: 'Dinh dưỡng' },
            { id: 'water', label: 'Nước' },
            { id: 'info', label: 'Log khác' }
          ].map(btn => (
            <button
              key={btn.id}
              onClick={() => setFilter(btn.id)}
              className={`px-3.5 py-2 rounded-lg text-xs font-medium transition-colors border ${filter === btn.id
                  ? 'bg-blue-600 text-white border-blue-500'
                  : 'bg-slate-950 text-slate-400 border-slate-800 hover:bg-slate-800'
                }`}
            >
              {btn.label}
            </button>
          ))}
        </div>
      </div>

      <div className="relative pl-3">
        {/* Thanh Timeline mảnh 1px */}
        <div className="absolute left-[19px] top-2 bottom-0 w-[1px] bg-slate-800"></div>

        <div className="space-y-5">
          {filteredEvents.length === 0 ? (
            <div className="pl-8">
              <StateView
                icon={Zap}
                title="Đang chờ dữ liệu..."
                description="Hệ thống chưa ghi nhận sự kiện nào theo bộ lọc này."
                className="bg-slate-900/50 border-slate-800"
              />
            </div>
          ) : (
            filteredEvents.map((ev, idx) => {
              const { icon: Icon, color, iconBorder, cardBg } = getEventStyle(ev.level, ev.title);
              const date = new Date(ev.timestamp);

              return (
                <div key={idx} className="relative pl-10 group">

                  {/* Icon Điểm Timeline */}
                  <div className={`absolute left-0 top-3 p-1.5 rounded-full border bg-slate-950 z-10 transition-colors ${iconBorder}`}>
                    <Icon size={14} className={color} strokeWidth={2.5} />
                  </div>

                  {/* Nội dung Card */}
                  <div className={`flex flex-col gap-2 p-4 rounded-xl border transition-colors hover:border-slate-600 ${cardBg}`}>
                    <div className="flex items-start justify-between gap-3">
                      <h4 className={`text-sm font-semibold leading-tight ${color}`}>{ev.title}</h4>
                      <span className="text-[11px] text-slate-500 font-medium whitespace-nowrap bg-slate-950/50 px-2 py-0.5 rounded border border-slate-800/50">
                        {date.toLocaleTimeString('vi-VN', { hour: '2-digit', minute: '2-digit', second: '2-digit' })}
                      </span>
                    </div>

                    <div className="text-sm text-slate-300 leading-relaxed font-medium">
                      {ev.message}
                    </div>

                    {/* Khối Metadata (Nếu có) */}
                    {ev.metadata && Object.keys(ev.metadata).length > 0 && (
                      <div className="mt-2 p-3 bg-slate-950 rounded-lg border border-slate-800 overflow-x-auto custom-scrollbar">
                        <p className="text-[10px] font-semibold text-slate-500 mb-1.5">DỮ LIỆU ĐÍNH KÈM:</p>
                        <pre className="text-[11px] font-mono text-slate-400 leading-relaxed">
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
