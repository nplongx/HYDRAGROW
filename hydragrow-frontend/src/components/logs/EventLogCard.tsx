import React, { useState } from 'react';
import {
  AlertCircle, AlertTriangle, FlaskConical, Waves, Settings2, Radio, UserCheck, Power, Wifi, Cpu, CheckCircle, Info, ChevronDown, ChevronUp
} from 'lucide-react';
import { MetadataRenderer } from './MetadataRenderers';

export interface SystemEvent {
  id: number;
  device_id: string;
  level: string;
  category: string;
  title: string;
  message: string;
  reason?: string;
  metadata?: Record<string, unknown>;
  timestamp: number;
}

interface EventStyle {
  icon: React.ElementType;
  iconColor: string;
  borderColor: string;
  bgColor: string;
  dot: string;
}

const getEventStyle = (event: SystemEvent): EventStyle => {
  const { level, category, title } = event;
  if (level === 'critical' || title.toLowerCase().includes('khẩn') || title.toLowerCase().includes('emergency')) {
    return { icon: AlertCircle, iconColor: 'text-red-700', borderColor: 'border-red-200', bgColor: 'from-rose-500/5 to-transparent', dot: 'bg-rose-500' };
  }
  if (level === 'warning') {
    return { icon: AlertTriangle, iconColor: 'text-amber-800', borderColor: 'border-amber-500/10', bgColor: 'from-amber-500/5 to-transparent', dot: 'bg-amber-500' };
  }
  switch (category?.toLowerCase().replace('_', '')) {
    case 'dosing': return { icon: FlaskConical, iconColor: 'text-cyan-700', borderColor: 'border-cyan-500/10', bgColor: 'from-cyan-500/5 to-transparent', dot: 'bg-cyan-400' };
    case 'water': return { icon: Waves, iconColor: 'text-sky-700', borderColor: 'border-sky-200', bgColor: 'from-sky-500/5 to-transparent', dot: 'bg-sky-400' };
    case 'calibration': return { icon: Settings2, iconColor: 'text-purple-700', borderColor: 'border-purple-500/10', bgColor: 'from-purple-500/5 to-transparent', dot: 'bg-purple-400' };
    case 'sensor': return { icon: Radio, iconColor: 'text-amber-800', borderColor: 'border-amber-500/10', bgColor: 'from-amber-500/5 to-transparent', dot: 'bg-amber-400' };
    case 'useraction': return { icon: UserCheck, iconColor: 'text-emerald-700', borderColor: 'border-emerald-500/10', bgColor: 'from-emerald-500/5 to-transparent', dot: 'bg-emerald-500' };
    case 'system':
      if (title.includes('Offline') || title.includes('Mất')) {
        return { icon: Power, iconColor: 'text-emerald-700/75', borderColor: 'border-emerald-100', bgColor: 'from-white to-transparent', dot: 'bg-emerald-500' };
      }
      if (title.includes('Trực tuyến') || title.includes('Online')) {
        return { icon: Wifi, iconColor: 'text-emerald-700', borderColor: 'border-emerald-500/10', bgColor: 'from-emerald-500/5 to-transparent', dot: 'bg-emerald-400' };
      }
      return { icon: Cpu, iconColor: 'text-emerald-800/80', borderColor: 'border-emerald-100', bgColor: 'from-white to-transparent', dot: 'bg-emerald-500' };
    default:
      if (level === 'success') {
        return { icon: CheckCircle, iconColor: 'text-emerald-700', borderColor: 'border-emerald-500/10', bgColor: 'from-emerald-500/5 to-transparent', dot: 'bg-emerald-400' };
      }
      return { icon: Info, iconColor: 'text-emerald-700', borderColor: 'border-emerald-100', bgColor: 'from-white to-transparent', dot: 'bg-emerald-600' };
  }
};

const FsmBadge = ({ message }: { message: string }) => {
  const stateMap: Record<string, { label: string; color: string }> = {
    'WaterRefilling': { label: 'Đang cấp nước', color: 'text-sky-700 bg-sky-50 border-sky-200' },
    'WaterDraining': { label: 'Đang xả nước', color: 'text-sky-700 bg-sky-50 border-sky-200' },
    'MimoDosing': { label: 'Đang châm MIMO', color: 'text-cyan-700 bg-cyan-50 border-cyan-200' },
    'ActiveMixing': { label: 'Trộn tuần hoàn', color: 'text-purple-700 bg-purple-50 border-purple-200' },
    'Monitoring': { label: 'Giám sát', color: 'text-emerald-800 bg-emerald-100 border-emerald-200' },
    'EmergencyStop': { label: 'Dừng khẩn cấp', color: 'text-red-700 bg-red-50 border-red-200' },
  };
  const matched = stateMap[message];
  if (!matched) return null;
  return (
    <span className={`px-2 py-0.5 rounded-full text-[10px] font-bold border ${matched.color}`}>
      {matched.label}
    </span>
  );
};

export const EventLogCard = ({
  ev,
  idx,
  onOpenDetail,
}: {
  ev: SystemEvent;
  idx: number;
  onOpenDetail?: (ev: SystemEvent) => void;
}) => {
  const [isExpanded, setIsExpanded] = useState(false);
  const style = getEventStyle(ev);
  const Icon = style.icon;
  const date = new Date(ev.timestamp > 1e12 ? ev.timestamp : ev.timestamp * 1000);

  const hasValidMsg = ev.message && ev.message !== ev.title && !ev.message.startsWith('Monitoring') && ev.level !== 'FSM_UPDATE';
  const hasMetadata = ev.metadata && Object.keys(ev.metadata).length > 0;

  return (
    <div
      className="relative flex gap-4 animate-in slide-in-from-bottom-3 duration-500"
      style={{ animationDelay: `${Math.min(idx * 20, 200)}ms`, animationFillMode: 'both' }}
    >
      <div className="relative z-10 shrink-0 mt-3.5">
        <div className={`w-7 h-7 rounded-full border-4 border-white flex items-center justify-center shadow-md ${style.dot}`}>
          <Icon size={11} className="text-white" strokeWidth={3} />
        </div>
      </div>

      <div className={`flex-1 min-w-0 border bg-gradient-to-r via-emerald-950/5 to-transparent border-emerald-100 rounded-2xl p-4 shadow-sm transition-all duration-300 hover:border-emerald-200 ${style.bgColor}`}>
        <div className="flex items-start justify-between gap-4 mb-2">
          <div className="space-y-1 min-w-0">
            <h4 className={`text-sm font-bold tracking-tight leading-snug ${style.iconColor}`}>
              {ev.title}
            </h4>
            <div className="flex items-center gap-2 pt-0.5">
              <FsmBadge message={ev.message} />
            </div>
          </div>
          <time className="text-[10px] text-emerald-700/75 font-mono text-right whitespace-nowrap shrink-0 leading-tight">
            {date.toLocaleTimeString('vi-VN', { hour: '2-digit', minute: '2-digit', second: '2-digit' })}
            <span className="block font-semibold text-emerald-700/60 text-[9px] mt-0.5">
              {date.toLocaleDateString('vi-VN', { day: '2-digit', month: '2-digit' })}
            </span>
          </time>
        </div>

        {hasValidMsg && (
          <p className="text-xs text-emerald-900 leading-relaxed font-medium opacity-95">
            {ev.message}
          </p>
        )}

        {ev.reason && (
          <div className="mt-2 flex items-center gap-1.5 text-[9px] text-red-700 bg-red-50 border border-red-200 rounded-md px-2 py-0.5 font-mono max-w-max">
            <AlertCircle size={10} /> Mã: {ev.reason}
          </div>
        )}

        {hasMetadata && (
          <div className="mt-2.5 pt-2 border-t border-emerald-100 flex flex-wrap items-center gap-3">
            <button
              onClick={() => setIsExpanded(!isExpanded)}
              className="flex items-center gap-1 text-[10px] font-bold text-emerald-700/75 hover:text-emerald-800 tracking-wide uppercase transition-colors"
            >
              <span>{isExpanded ? 'Thu nhỏ thông số' : 'Xem thông số kỹ thuật'}</span>
              {isExpanded ? <ChevronUp size={12} /> : <ChevronDown size={12} />}
            </button>
            {onOpenDetail && (
              <button
                onClick={() => onOpenDetail(ev)}
                className="flex items-center gap-1 text-[10px] font-bold text-sky-700 hover:text-sky-800 tracking-wide uppercase transition-colors"
              >
                <span>Xem JSON thô</span>
              </button>
            )}
            {isExpanded && <MetadataRenderer metadata={ev.metadata} />}
          </div>
        )}
      </div>
    </div>
  );
};
