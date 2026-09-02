// src/components/logs/EventDetailDrawer.tsx
import { X, Code2 } from 'lucide-react';
import { AccordionSection } from '../ui/AccordionSection';
import { MetadataRenderer } from './MetadataRenderers';
import type { SystemEvent } from './EventLogCard';

export interface EventDetailDrawerProps {
  event: SystemEvent | null;
  onClose: () => void;
}

export const EventDetailDrawer = ({ event, onClose }: EventDetailDrawerProps) => {
  if (!event) return null;
  const date = new Date(event.timestamp > 1e12 ? event.timestamp : event.timestamp * 1000);
  const hasMetadata = event.metadata && Object.keys(event.metadata).length > 0;

  return (
    <div className="flex h-full flex-col gap-4 p-4">
      <div className="flex items-center justify-between">
        <h2 className="text-base font-bold text-emerald-950">Chi tiết sự kiện</h2>
        <button type="button" onClick={onClose} className="text-emerald-700/70 hover:text-emerald-900" aria-label="Đóng chi tiết">
          <X size={18} />
        </button>
      </div>

      {/* Tóm tắt dễ hiểu — dành cho người dùng thường */}
      <div className="ui-card">
        <h3 className="text-sm font-bold text-emerald-950">{event.title}</h3>
        <p className="text-xs text-emerald-700/75 font-mono mt-1">{date.toLocaleString('vi-VN')}</p>
        {event.message && event.message !== event.title && (
          <p className="text-xs text-emerald-900 leading-relaxed mt-2">{event.message}</p>
        )}
        {event.reason && <p className="text-xs text-red-700 mt-2">Mã lỗi: {event.reason}</p>}
      </div>

      {hasMetadata && <MetadataRenderer metadata={event.metadata} />}

      {/* JSON thô — dành cho người dùng kỹ thuật, thu gọn mặc định */}
      {hasMetadata && (
        <AccordionSection title="JSON thô" icon={Code2}>
          <pre className="text-[10px] font-mono text-emerald-900 bg-emerald-50/80 rounded-xl p-3 overflow-x-auto whitespace-pre-wrap break-all">
            {JSON.stringify(event, null, 2)}
          </pre>
        </AccordionSection>
      )}
    </div>
  );
};
