import React, { useState } from 'react';
import {
  AlertCircle, AlertTriangle, FlaskConical, Waves, Settings2, Radio, UserCheck, Power, Wifi, Cpu, CheckCircle, Info, ChevronDown, ChevronUp
} from 'lucide-react';
import { MetadataRenderer } from './MetadataRenderers';
import { splitByMatch } from '../../lib/logs/highlightMatch';
import { EVENT_CATEGORY_THEME } from '../../lib/logs/eventCategoryTheme';

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
  resolved_at?: string | null;
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
    return { icon: AlertCircle, iconColor: 'text-red-700', borderColor: 'border-red-200', bgColor: 'from-primary-deep/5 to-transparent', dot: 'bg-primary-deep' };
  }
  if (level === 'warning') {
    return { icon: AlertTriangle, iconColor: EVENT_CATEGORY_THEME.warning.icon, borderColor: 'border-warn-deep/20', bgColor: 'from-warning-bg to-transparent', dot: 'bg-warn-deep' };
  }
  switch (category?.toLowerCase().replace('_', '')) {
    case 'dosing': return { icon: FlaskConical, iconColor: EVENT_CATEGORY_THEME.device.icon, borderColor: 'border-line', bgColor: 'from-soft to-transparent', dot: 'bg-primary' };
    case 'water': return { icon: Waves, iconColor: EVENT_CATEGORY_THEME.water.icon, borderColor: 'border-line', bgColor: 'from-soft to-transparent', dot: 'bg-primary' };
    case 'calibration': return { icon: Settings2, iconColor: EVENT_CATEGORY_THEME.phDosing.icon, borderColor: 'border-line', bgColor: 'from-soft to-transparent', dot: 'bg-primary' };
    case 'sensor': return { icon: Radio, iconColor: 'text-amber-800', borderColor: 'border-amber-500/10', bgColor: 'from-amber-500/5 to-transparent', dot: 'bg-amber-400' };
    case 'useraction': return { icon: UserCheck, iconColor: 'text-status', borderColor: 'border-line', bgColor: 'from-primary/5 to-transparent', dot: 'bg-status' };
    case 'system':
      if (title.includes('Offline') || title.includes('Mất')) {
        return { icon: Power, iconColor: 'text-primary/75', borderColor: 'border-line', bgColor: 'from-white to-transparent', dot: 'bg-status' };
      }
      if (title.includes('Trực tuyến') || title.includes('Online')) {
        return { icon: Wifi, iconColor: 'text-status', borderColor: 'border-line', bgColor: 'from-primary/5 to-transparent', dot: 'bg-status' };
      }
      return { icon: Cpu, iconColor: 'text-primary-deep/80', borderColor: 'border-line', bgColor: 'from-white to-transparent', dot: 'bg-status' };
    default:
      if (level === 'success') {
        return { icon: CheckCircle, iconColor: 'text-status', borderColor: 'border-line', bgColor: 'from-primary/5 to-transparent', dot: 'bg-status' };
      }
      return { icon: Info, iconColor: 'text-status', borderColor: 'border-line', bgColor: 'from-white to-transparent', dot: 'bg-status' };
  }
};

const FsmBadge = ({ message }: { message: string }) => {
  const stateMap: Record<string, { label: string; color: string }> = {
    'WaterRefilling': { label: 'Đang cấp nước', color: EVENT_CATEGORY_THEME.water.badge },
    'WaterDraining': { label: 'Đang xả nước', color: EVENT_CATEGORY_THEME.water.badge },
    'MimoDosing': { label: 'Đang châm MIMO', color: EVENT_CATEGORY_THEME.device.badge },
    'ActiveMixing': { label: 'Trộn tuần hoàn', color: EVENT_CATEGORY_THEME.phDosing.badge },
    'Monitoring': { label: 'Giám sát', color: 'text-primary-deep bg-soft border-line' },
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

const HighlightedText = ({ text, query }: { text: string; query?: string }) => (
  <>
    {splitByMatch(text, query ?? '').map((seg, i) =>
      seg.matched ? (
        <mark key={i} className="bg-warning-bg text-warn-deep rounded-sm px-0.5">
          {seg.text}
        </mark>
      ) : (
        <span key={i}>{seg.text}</span>
      ),
    )}
  </>
);

export const EventLogCard = ({
  ev,
  idx,
  search,
  onOpenDetail,
  onAcknowledge,
}: {
  ev: SystemEvent;
  idx: number;
  search?: string;
  onOpenDetail?: (ev: SystemEvent) => void;
  onAcknowledge?: (ev: SystemEvent) => void;
}) => {
  const [isExpanded, setIsExpanded] = useState(false);
  const style = getEventStyle(ev);
  const Icon = style.icon;
  const date = new Date(ev.timestamp > 1e12 ? ev.timestamp : ev.timestamp * 1000);
  const isResolved = Boolean(ev.resolved_at);

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

      <div className={`flex-1 min-w-0 border bg-gradient-to-r via-primary/5 to-transparent border-line rounded-2xl p-4 shadow-sm transition-all duration-300 hover:border-primary/40 ${style.bgColor} ${isResolved ? 'opacity-60' : ''}`}>
        <div className="flex items-start justify-between gap-4 mb-2">
          <div className="space-y-1 min-w-0">
            <div className="flex items-center gap-2 flex-wrap">
              <h4 className={`text-sm font-bold tracking-tight leading-snug ${style.iconColor}`}>
                <HighlightedText text={ev.title} query={search} />
              </h4>
              {isResolved && (
                <span className="px-2 py-0.5 rounded-full text-[10px] font-bold border border-status/30 bg-pill text-status">
                  ✓ Đã xử lý
                </span>
              )}
            </div>
            <div className="flex items-center gap-2 pt-0.5">
              <FsmBadge message={ev.message} />
            </div>
          </div>
          <time className="text-[10px] text-text-muted font-mono text-right whitespace-nowrap shrink-0 leading-tight">
            {date.toLocaleTimeString('vi-VN', { hour: '2-digit', minute: '2-digit', second: '2-digit' })}
            <span className="block font-semibold text-faint text-[9px] mt-0.5">
              {date.toLocaleDateString('vi-VN', { day: '2-digit', month: '2-digit' })}
            </span>
          </time>
        </div>

        {hasValidMsg && (
          <p className="text-xs text-primary-deep leading-relaxed font-medium opacity-95">
            {ev.message}
          </p>
        )}

        {ev.reason && (
          <div className="mt-2 flex items-center gap-1.5 text-[9px] text-red-700 bg-red-50 border border-red-200 rounded-md px-2 py-0.5 font-mono max-w-max">
            <AlertCircle size={10} /> Mã: {ev.reason}
          </div>
        )}

        {hasMetadata && (
          <div className="mt-2.5 pt-2 border-t border-line flex flex-wrap items-center gap-3">
            <button
              onClick={() => setIsExpanded(!isExpanded)}
              className="flex items-center gap-1 text-[10px] font-bold text-primary/75 hover:text-primary-deep tracking-wide uppercase transition-colors"
            >
              <span>{isExpanded ? 'Thu nhỏ thông số' : 'Xem thông số kỹ thuật'}</span>
              {isExpanded ? <ChevronUp size={12} /> : <ChevronDown size={12} />}
            </button>
            {onOpenDetail && (
              <button
                onClick={() => onOpenDetail(ev)}
                className="flex items-center gap-1 text-[10px] font-bold text-primary hover:text-primary-deep tracking-wide uppercase transition-colors"
              >
                <span>Xem JSON thô</span>
              </button>
            )}
            {onAcknowledge && (
              <button
                onClick={() => onAcknowledge(ev)}
                className="flex items-center gap-1 text-[10px] font-bold text-status hover:text-primary-deep tracking-wide uppercase transition-colors"
                title={isResolved ? 'Đánh dấu là chưa xử lý' : 'Đánh dấu là đã xử lý'}
              >
                <CheckCircle size={12} />
                <span>{isResolved ? 'Mở lại' : 'Đánh dấu đã xử lý'}</span>
              </button>
            )}
            {isExpanded && <MetadataRenderer metadata={ev.metadata} />}
          </div>
        )}
      </div>
    </div>
  );
};
