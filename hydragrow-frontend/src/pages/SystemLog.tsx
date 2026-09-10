import { useMemo, useState } from 'react';
import { Clock, Filter, AlertTriangle, FlaskConical, Waves, UserCheck, Cpu, CheckCircle, Workflow, Download, Zap, ExternalLink } from 'lucide-react';
import toast from 'react-hot-toast';
import { useInfiniteQuery } from '@tanstack/react-query';

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

const PAGE_SIZE = 200;

const FILTERS = [
  { id: 'all', label: 'Tất cả', icon: Filter },
  { id: 'unresolved', label: 'Chưa xử lý', icon: CheckCircle },
  { id: 'alert', label: 'Cảnh báo', icon: AlertTriangle },
  { id: 'dosing', label: 'Châm vi chất', icon: FlaskConical },
  { id: 'water', label: 'Nước', icon: Waves },
  { id: 'device', label: 'Thiết bị', icon: Cpu },
  { id: 'automation', label: 'Tự động hóa', icon: Workflow },
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

  // TanStack Query tự động caching & cancellation. Mỗi trang tối đa PAGE_SIZE sự
  // kiện; trang tiếp theo dùng before_timestamp = timestamp của event cũ nhất
  // trong trang trước (API đã hỗ trợ cursor này, xem hydragrow-backend/src/api/alert.rs).
  const { data, isLoading, fetchNextPage, hasNextPage, isFetchingNextPage, refetch } = useInfiniteQuery({
    queryKey: ['system-events', deviceId, filter],
    initialPageParam: undefined as number | undefined,
    queryFn: async ({ pageParam }) => {
      if (!deviceId || !settings?.backend_url) return [];
      let url = `${settings.backend_url}/api/devices/${deviceId}/events?limit=${PAGE_SIZE}`;
      if (filter !== 'all' && filter !== 'unresolved') {
        const category = filter === 'user_action' ? 'user_action,alert' : filter;
        url += `&category=${encodeURIComponent(category)}`;
      }
      if (pageParam) url += `&before_timestamp=${pageParam}`;
      const res = await httpFetch(url, { headers: { 'X-API-Key': settings.api_key || '' } });
      if (!res.ok) return [];
      const json = await res.json();
      return (json.data ?? []) as SystemEvent[];
    },
    getNextPageParam: (lastPage) => {
      if (!lastPage || lastPage.length < PAGE_SIZE) return undefined;
      return lastPage[lastPage.length - 1]?.timestamp;
    },
    enabled: Boolean(deviceId && settings?.backend_url)
  });

  const systemEvents = useMemo(() => (data?.pages ?? []).flat(), [data]);

  const visibleRows = useMemo(() => {
    let filtered = filterEventsBySearch(systemEvents as SystemEvent[], search);
    if (filter === 'unresolved') {
      filtered = filtered.filter((ev) => !ev.resolved_at);
    }
    return buildLogRows(filtered, mode);
  }, [systemEvents, search, mode, filter]);

  const handleAcknowledge = async (ev: SystemEvent) => {
    if (!deviceId || !settings?.backend_url) return;
    try {
      const res = await httpFetch(
        `${settings.backend_url}/api/devices/${deviceId}/events/${ev.id}/acknowledge`,
        {
          method: 'PUT',
          headers: { 'X-API-Key': settings.api_key || '', 'Content-Type': 'application/json' },
          body: JSON.stringify({ resolved: !ev.resolved_at }),
        },
      );
      if (res.ok) {
        toast.success(ev.resolved_at ? 'Đã mở lại sự kiện.' : 'Đã đánh dấu xử lý xong.');
        refetch();
      } else {
        toast.error('Không thể cập nhật trạng thái sự kiện.');
      }
    } catch {
      toast.error('Lỗi mạng khi cập nhật trạng thái sự kiện.');
    }
  };

  const toMs = (ts: number) => (ts > 1e12 ? ts : ts * 1000);

  const dayGroups = useMemo(() => {
    const today = new Date();
    today.setHours(0, 0, 0, 0);
    const yesterday = new Date(today.getTime() - 86400000);
    const groups: { key: string; label: string; rows: typeof visibleRows }[] = [];

    visibleRows.forEach((row) => {
      const ts = row.type === 'event'
        ? row.event.timestamp
        : row.type === 'cycle'
          ? (row.events[0]?.timestamp ?? Date.now())
          : row.latestTimestamp;
      const date = new Date(toMs(Number(ts ?? 0)));
      const key = `${date.getFullYear()}-${date.getMonth()}-${date.getDate()}`;
      let label: string;
      if (date.toDateString() === today.toDateString()) {
        label = 'HÔM NAY';
      } else if (date.toDateString() === yesterday.toDateString()) {
        label = 'HÔM QUA';
      } else {
        label = date.toLocaleDateString('vi-VN', { weekday: 'long', day: '2-digit', month: '2-digit', year: 'numeric' });
      }
      const last = groups[groups.length - 1];
      if (last && last.key === key) {
        last.rows.push(row);
      } else {
        groups.push({ key, label, rows: [row] });
      }
    });
    return groups;
  }, [visibleRows]);

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
      className="inline-flex items-center gap-1.5 text-xs font-semibold text-primary hover:text-primary-deep"
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
      <div className="bg-white/90 border border-line rounded-3xl p-4 flex flex-col md:flex-row justify-between items-stretch md:items-center gap-4 relative z-10 backdrop-blur-md">
        <div className="flex flex-wrap gap-1.5 flex-1 min-w-0">
          {FILTERS.map(btn => {
            const Icon = btn.icon;
            const active = filter === btn.id;
            return (
              <button
                key={btn.id}
                onClick={() => setFilter(btn.id)}
                className={`flex items-center gap-1.5 px-3 py-1.5 rounded-xl text-xs font-semibold transition-all duration-200 border whitespace-nowrap ${
                  active ? 'bg-primary-deep text-white border-transparent shadow-md' : 'bg-white text-text-muted border-line hover:bg-pill'
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
          className="flex items-center justify-center space-x-2 bg-soft hover:bg-pill disabled:opacity-40 text-primary-deep px-4 py-1.5 rounded-xl border border-line text-xs font-bold shrink-0"
        >
          <Download size={13} />
          <span>Xuất CSV</span>
        </button>
      </div>

      <div className="flex flex-col lg:flex-row gap-4 items-start">
        <div className={`flex-1 min-w-0 w-full ${selectedEvent ? 'lg:max-w-2xl' : ''}`}>
          {isLoading ? (
            <div className="flex items-center justify-center gap-2.5 py-24 text-text-muted">
              <div className="w-4 h-4 border-2 border-line border-t-primary rounded-full animate-spin" />
              <span className="text-xs font-semibold uppercase tracking-wider text-primary-deep">Đang đồng bộ dòng thời gian...</span>
            </div>
          ) : visibleRows.length === 0 ? (
            <StateView
              icon={Zap}
              title="Dòng thời gian trống"
              description="Chưa ghi nhận khoảnh khắc nào khớp bộ lọc/tìm kiếm hiện tại."
            />
          ) : (
            <div className="relative pl-3">
              <div className="absolute left-[13px] top-4 bottom-4 w-0.5 bg-gradient-to-b from-primary/30 via-primary/15 to-transparent pointer-events-none" />
              <div className="space-y-5">
                {dayGroups.map((group, groupIdx) => (
                  <div key={group.key} className="space-y-4">
                    <div className="flex items-center gap-2.5 pt-2">
                      <span className="w-2 h-2 rounded-full bg-primary shrink-0" />
                      <span className="text-[11px] font-bold uppercase tracking-wider text-primary-deep">{group.label}</span>
                      <span className="text-[10px] text-faint font-medium">{group.rows.length} sự kiện</span>
                      <div className="flex-1 h-px bg-line last:hidden" />
                    </div>
                    {group.rows.map((row, idx) => {
                      const globalIdx = groupIdx * 1000 + idx;
                      if (row.type === 'event') {
                        return <EventLogCard key={row.event.id} ev={row.event} idx={globalIdx} onOpenDetail={setSelectedEvent} onAcknowledge={handleAcknowledge} />;
                      }
                      if (row.type === 'cycle') {
                        return <CycleEventCard key={row.cycleId} cycleId={row.cycleId} events={row.events} onOpenDetail={setSelectedEvent} />;
                      }
                      return (
                        <div key={`merged-${row.title}-${row.latestTimestamp}`} className="flex items-center gap-3 pl-10">
                          <span className="log-neutral-badge">×{row.count}</span>
                          <span className="text-xs text-text-muted font-medium">{row.title} — gộp {row.count} sự kiện kỹ thuật lặp lại</span>
                        </div>
                      );
                    })}
                  </div>
                ))}
              </div>
              {hasNextPage && (
                <div className="flex justify-center pt-4">
                  <button
                    onClick={() => fetchNextPage()}
                    disabled={isFetchingNextPage}
                    className="text-xs font-semibold text-primary hover:text-primary-deep disabled:opacity-50 px-4 py-2"
                  >
                    {isFetchingNextPage ? 'Đang tải...' : 'Tải thêm sự kiện cũ hơn'}
                  </button>
                </div>
              )}
            </div>
          )}
        </div>

        {selectedEvent && (
          <div className="relative z-20 w-full lg:w-[26rem] shrink-0 border border-line rounded-2xl bg-white shadow-xl shadow-primary/10">
            <EventDetailDrawer event={selectedEvent} onClose={() => setSelectedEvent(null)} />
          </div>
        )}
      </div>
    </div>
  );
};

export default SystemLog;
