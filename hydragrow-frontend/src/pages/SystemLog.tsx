import { useMemo, useState } from 'react';
import { Clock, Filter, AlertTriangle, FlaskConical, Waves, UserCheck, Cpu, Download, Zap, ExternalLink } from 'lucide-react';
import toast from 'react-hot-toast';
import { useQuery } from '@tanstack/react-query';

// --- STORE, GLEAM & COMPONENTS ---
import { useDeviceStore } from '../store/useDeviceStore';
import { escape_field_str } from '../../gleam_core/build/dev/javascript/gleam_core/csv.mjs';
import { PageHeader } from '../components/ui/PageHeader';
import { StateView } from '../components/ui/StateView';
import { EventLogCard, type SystemEvent } from '../components/logs/EventLogCard';
import { HealthSummaryBar } from '../components/logs/HealthSummaryBar';
import { CycleEventCard } from '../components/logs/CycleEventCard';
import { EventDetailDrawer } from '../components/logs/EventDetailDrawer';
import { useSystemHealthSummary } from '../hooks/useSystemHealthSummary';
import { buildLogRows, filterEventsBySearch, type LogViewMode } from '../lib/logs/eventGrouping';
import { httpFetch } from '../platform/http';
import { saveTextFile } from '../platform/file';

const FILTERS = [
  { id: 'all', label: 'Tất cả', icon: Filter },
  { id: 'alert', label: 'Cảnh báo', icon: AlertTriangle },
  { id: 'dosing', label: 'Châm vi chất', icon: FlaskConical },
  { id: 'water', label: 'Nước', icon: Waves },
  { id: 'user_action', label: 'Người dùng', icon: UserCheck },
  { id: 'system', label: 'Hệ thống', icon: Cpu },
];

const SystemLog = ({ variant = 'standalone' }: { variant?: 'standalone' | 'embedded' }) => {
  const deviceId = useDeviceStore((s) => s.deviceId);
  const settings = useDeviceStore((s) => s.settings);
  const [filter, setFilter] = useState<string>('all');
  const [mode, setMode] = useState<LogViewMode>('important');
  const [search, setSearch] = useState('');
  const [selectedEvent, setSelectedEvent] = useState<SystemEvent | null>(null);

  const { data: healthSummary } = useSystemHealthSummary(deviceId || '');

  // TanStack Query tự động caching & cancellation
  const { data: systemEvents = [], isLoading } = useQuery({
    queryKey: ['system-events', deviceId, filter],
    queryFn: async () => {
      if (!deviceId || !settings?.backend_url) return [];
      let url = `${settings.backend_url}/api/devices/${deviceId}/events?limit=200`;
      if (filter !== 'all') {
        const category = filter === 'user_action' ? 'user_action,alert' : filter;
        url += `&category=${encodeURIComponent(category)}`;
      }
      const res = await httpFetch(url, { headers: { 'X-API-Key': settings.api_key || '' } });
      if (!res.ok) return [];
      const json = await res.json();
      return json.data ?? [];
    },
    enabled: Boolean(deviceId && settings?.backend_url)
  });

  const visibleRows = useMemo(() => {
    const filtered = filterEventsBySearch(systemEvents as SystemEvent[], search);
    return buildLogRows(filtered, mode);
  }, [systemEvents, search, mode]);

  // Xuất file CSV thông qua Module Gleam csv.mjs
  const handleExportCSV = async () => {
    if (systemEvents.length === 0) return toast.error("Không có nhật ký!");
    try {
      const headers = ["ID", "Thời Gian", "Mã Thiết Bị", "Cấp Độ", "Danh Mục", "Tiêu Đề", "Nội Dung Message"];
      const csvRows = (systemEvents as SystemEvent[]).map((ev) => {
        const date = new Date(ev.timestamp > 1e12 ? ev.timestamp : ev.timestamp * 1000).toLocaleString('vi-VN');
        return [
          escape_field_str(String(ev.id || '')),
          escape_field_str(date),
          escape_field_str(ev.device_id || ''),
          escape_field_str(ev.level || ''),
          escape_field_str(ev.category || ''),
          escape_field_str(ev.title || ''),
          escape_field_str(ev.message || '')
        ].join(",");
      });

      const csvContent = "\uFEFF" + [headers.join(","), ...csvRows].join("\n");
      const saved = await saveTextFile(`nhat-ky-${deviceId || 'all'}.csv`, csvContent);
      if (saved) toast.success("Xuất CSV thành công!");
    } catch { toast.error("Lỗi khi xuất file!"); }
  };

  const grafanaLink = (
    <a
      href="http://localhost:3000"
      target="_blank"
      rel="noopener noreferrer"
      className="inline-flex items-center gap-1.5 text-xs font-semibold text-sky-700 hover:text-sky-800"
    >
      <ExternalLink size={13} />
      <span>Mở Grafana</span>
    </a>
  );

  return (
    <div className={variant === "embedded" ? "" : "app-page"}>
      {variant !== 'embedded' && (
        <PageHeader
          icon={Clock}
          title="Nhật Ký Hành Trình"
          subtitle={`Dòng thời gian vận hành của trạm ${deviceId || ''}`}
          action={grafanaLink}
        />
      )}

      <HealthSummaryBar summary={healthSummary} mode={mode} onModeChange={setMode} search={search} onSearchChange={setSearch} />

      {/* Filter & CSV Export Bar */}
      <div className="bg-white/90 border border-emerald-100 rounded-3xl p-4 flex flex-col md:flex-row justify-between items-stretch md:items-center gap-4 relative z-10 backdrop-blur-md">
        <div className="flex flex-wrap gap-1.5 flex-1 min-w-0">
          {FILTERS.map(btn => {
            const Icon = btn.icon;
            const active = filter === btn.id;
            return (
              <button
                key={btn.id}
                onClick={() => setFilter(btn.id)}
                className={`flex items-center gap-1.5 px-3 py-1.5 rounded-xl text-xs font-semibold transition-all duration-200 border whitespace-nowrap ${
                  active ? 'bg-sky-600 text-white border-transparent shadow-md' : 'bg-white text-emerald-800 border-emerald-100 hover:bg-emerald-50'
                }`}
              >
                <Icon size={12} />
                {btn.label}
              </button>
            );
          })}
        </div>
        <button
          onClick={handleExportCSV}
          disabled={systemEvents.length === 0}
          className="flex items-center justify-center space-x-2 bg-emerald-100 hover:bg-emerald-200 disabled:opacity-40 text-emerald-900 px-4 py-1.5 rounded-xl border border-emerald-200 text-xs font-bold shrink-0"
        >
          <Download size={13} />
          <span>Xuất CSV</span>
        </button>
      </div>

      <div className="flex flex-col lg:flex-row gap-4 items-start">
        <div className={`flex-1 min-w-0 w-full ${selectedEvent ? 'lg:max-w-2xl' : ''}`}>
          {isLoading ? (
            <div className="flex items-center justify-center gap-2.5 py-24 text-emerald-700/75">
              <div className="w-4 h-4 border-2 border-emerald-100 border-t-blue-500 rounded-full animate-spin" />
              <span className="text-xs font-semibold uppercase tracking-wider text-emerald-800">Đang đồng bộ dòng thời gian...</span>
            </div>
          ) : visibleRows.length === 0 ? (
            <StateView
              icon={Zap}
              title="Dòng thời gian trống"
              description="Chưa ghi nhận khoảnh khắc nào khớp bộ lọc/tìm kiếm hiện tại."
            />
          ) : (
            <div className="relative pl-3">
              <div className="absolute left-[13px] top-4 bottom-4 w-0.5 bg-gradient-to-b from-emerald-200 via-emerald-300 to-transparent pointer-events-none" />
              <div className="space-y-4">
                {visibleRows.map((row, idx) => {
                  if (row.type === 'event') {
                    return <EventLogCard key={row.event.id} ev={row.event} idx={idx} onOpenDetail={setSelectedEvent} />;
                  }
                  if (row.type === 'cycle') {
                    return <CycleEventCard key={row.cycleId} cycleId={row.cycleId} events={row.events} onOpenDetail={setSelectedEvent} />;
                  }
                  return (
                    <div key={`merged-${row.title}-${row.latestTimestamp}`} className="flex items-center gap-3 pl-10">
                      <span className="log-neutral-badge">×{row.count}</span>
                      <span className="text-xs text-emerald-800/80 font-medium">{row.title} — gộp {row.count} sự kiện kỹ thuật lặp lại</span>
                    </div>
                  );
                })}
              </div>
            </div>
          )}
        </div>

        {selectedEvent && (
          <div className="relative z-20 w-full lg:w-[26rem] shrink-0 border border-emerald-100 rounded-2xl bg-white shadow-xl shadow-emerald-950/10">
            <EventDetailDrawer event={selectedEvent} onClose={() => setSelectedEvent(null)} />
          </div>
        )}
      </div>
    </div>
  );
};

export default SystemLog;
