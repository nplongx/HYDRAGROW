import { useMemo, useState } from 'react';
import { Clock, Filter, AlertTriangle, FlaskConical, Waves, UserCheck, Cpu, CheckCircle, Workflow, Download, Zap, ExternalLink, Radio } from 'lucide-react';
import toast from 'react-hot-toast';

// --- STORE, GLEAM & COMPONENTS ---
import { useStationContext } from '../contexts/StationContext';
import { escape_field_str } from '../../gleam_core/build/dev/javascript/gleam_core/csv.mjs';
import { PageHeader } from '../components/ui/PageHeader';
import { StateView } from '../components/ui/StateView';
import { EventLogCard, type SystemEvent } from '../components/logs/EventLogCard';
import { HealthSummaryBar } from '../components/logs/HealthSummaryBar';
import { CycleEventCard } from '../components/logs/CycleEventCard';
import { EventDetailDrawer } from '../components/logs/EventDetailDrawer';
import { useSystemHealthSummary } from '../hooks/useSystemHealthSummary';
import { buildLogRows, filterEventsBySearch, type LogViewMode } from '../lib/logs/eventGrouping';
import { useAcknowledgeJournalEvent, useJournalEvents } from '../hooks/useSystemEvents';
import { saveTextFile } from '../platform/file';

const FILTERS = [
  { id: 'all', label: 'Tất cả', icon: Filter },
  { id: 'unresolved', label: 'Chưa xử lý', icon: CheckCircle },
  { id: 'alert', label: 'Cảnh báo', icon: AlertTriangle },
  { id: 'dosing', label: 'Châm vi chất', icon: FlaskConical },
  { id: 'water', label: 'Nước', icon: Waves },
  { id: 'device', label: 'Thiết bị', icon: Cpu },
  { id: 'sensor', label: 'Cảm biến', icon: Radio },
  { id: 'automation', label: 'Tự động hóa', icon: Workflow },
  { id: 'user_action', label: 'Người dùng', icon: UserCheck },
  { id: 'system', label: 'Hệ thống', icon: Cpu },
];

const SystemLog = ({ variant = 'standalone' }: { variant?: 'standalone' | 'embedded' }) => {
  const { selectedDeviceId: deviceId } = useStationContext();
  const [filter, setFilter] = useState<string>('all');
  const [level, setLevel] = useState<string>('all');
  const [unresolved, setUnresolved] = useState(false);
  const [mode, setMode] = useState<LogViewMode>('important');
  const [search, setSearch] = useState('');
  const [selectedEvent, setSelectedEvent] = useState<SystemEvent | null>(null);

  const { data: healthSummary } = useSystemHealthSummary(deviceId || '');

  const journalFilters = useMemo(() => ({
    deviceId,
    category: filter !== 'all' && filter !== 'unresolved' ? filter : undefined,
    level: level !== 'all' ? level : undefined,
    unresolved,
    search: search.trim() || undefined,
  }), [deviceId, filter, level, unresolved, search]);

  const {
    data,
    isLoading,
    isError,
    error,
    fetchNextPage,
    hasNextPage,
    isFetchingNextPage,
    refetch,
  } = useJournalEvents(journalFilters);
  const acknowledgeMutation = useAcknowledgeJournalEvent(journalFilters);

  const systemEvents = useMemo(() => (data?.pages ?? []).flatMap((page) => page.data ?? []), [data]);

  const visibleRows = useMemo(() => {
    let filtered = filterEventsBySearch(systemEvents, search);
    if (unresolved) {
      filtered = filtered.filter((ev) => !ev.resolved_at);
    }
    return buildLogRows(filtered, mode);
  }, [systemEvents, search, mode, unresolved]);

  const handleAcknowledge = async (ev: SystemEvent) => {
    if (!deviceId) return;
    try {
      await acknowledgeMutation.mutateAsync({ eventId: String(ev.id), resolved: !ev.resolved_at });
      toast.success(ev.resolved_at ? 'Đã mở lại sự kiện.' : 'Đã đánh dấu xử lý xong.');
      await refetch();
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
      const csvRows = systemEvents.map((ev) => {
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

      <HealthSummaryBar
        summary={healthSummary}
        mode={mode}
        onModeChange={setMode}
        search={search}
        onSearchChange={setSearch}
        resultCount={search ? visibleRows.length : undefined}
      />

      {/* Filter & CSV Export Bar */}
      <div className="bg-white/90 border border-line rounded-3xl p-4 flex flex-col md:flex-row justify-between items-stretch md:items-center gap-4 relative z-10 backdrop-blur-md">
        <div className="flex flex-wrap gap-1.5 flex-1 min-w-0">
          {FILTERS.map(btn => {
            const Icon = btn.icon;
            const active = filter === btn.id;
            return (
              <button
                key={btn.id}
                onClick={() => {
                  setFilter(btn.id);
                  if (btn.id === 'unresolved') setUnresolved(true);
                  else if (filter === 'unresolved') setUnresolved(false);
                }}
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
        <label className="flex items-center gap-2 text-xs font-semibold text-text-muted shrink-0">
          Cấp độ
          <select
            aria-label="Lọc cấp độ nhật ký"
            value={level}
            onChange={(event) => setLevel(event.target.value)}
            className="rounded-xl border border-line bg-white px-2.5 py-1.5 text-xs"
          >
            <option value="all">Tất cả</option>
            <option value="info">Info</option>
            <option value="success">Success</option>
            <option value="warning">Warning</option>
            <option value="error">Error</option>
            <option value="critical">Critical</option>
          </select>
        </label>
        <label className="inline-flex items-center gap-2 text-xs font-semibold text-text-muted shrink-0">
          <input type="checkbox" checked={unresolved} onChange={(event) => setUnresolved(event.target.checked)} />
          Chưa xử lý
        </label>
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
          ) : isError ? (
            <StateView
              icon={AlertTriangle}
              title="Không thể tải nhật ký"
              description={error instanceof Error ? error.message : 'Đã xảy ra lỗi khi truy vấn nhật ký.'}
              action={<button type="button" onClick={() => refetch()} className="text-xs font-semibold text-primary hover:text-primary-deep">Thử lại</button>}
            />
          ) : visibleRows.length === 0 ? (
            <StateView
              icon={filter === 'sensor' && !search ? Radio : Zap}
              title={
                search
                  ? `Không tìm thấy kết quả cho "${search}"`
                  : filter === 'sensor'
                    ? 'Không có cảnh báo cảm biến — hệ thống ổn định'
                    : 'Dòng thời gian trống'
              }
              description={
                search
                  ? 'Thử từ khoá ngắn hơn, kiểm tra chính tả, hoặc đổi bộ lọc danh mục đang chọn.'
                  : filter === 'sensor'
                    ? 'Không ghi nhận sự kiện cảm biến nào trong khoảng thời gian này. Cảm biến EC/pH/nhiệt độ/mực nước vẫn hoạt động bình thường nếu không có cảnh báo. Để xem bản đọc info chi tiết, chuyển sang chế độ "Toàn bộ kỹ thuật" hoặc kiểm tra hiệu chuẩn và kết nối cảm biến.'
                    : 'Chưa ghi nhận khoảnh khắc nào khớp bộ lọc hiện tại.'
              }
              action={
                search ? (
                  <button
                    type="button"
                    onClick={() => setSearch('')}
                    className="text-xs font-semibold text-primary hover:text-primary-deep"
                  >
                    Xoá tìm kiếm
                  </button>
                ) : filter === 'sensor' ? (
                  <div className="flex items-center justify-center gap-3">
                    <button
                      type="button"
                      onClick={() => setMode('all_technical')}
                      className="text-xs font-semibold text-primary hover:text-primary-deep"
                    >
                      Xem bản đọc chi tiết
                    </button>
                    <button
                      type="button"
                      onClick={() => setFilter('all')}
                      className="text-xs font-semibold text-primary hover:text-primary-deep"
                    >
                      Xem tất cả danh mục
                    </button>
                  </div>
                ) : filter !== 'all' ? (
                  <button
                    type="button"
                    onClick={() => setFilter('all')}
                    className="text-xs font-semibold text-primary hover:text-primary-deep"
                  >
                    Xem tất cả danh mục
                  </button>
                ) : undefined
              }
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
                        return <EventLogCard key={row.event.id} ev={row.event} idx={globalIdx} search={search} onOpenDetail={setSelectedEvent} onAcknowledge={handleAcknowledge} />;
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
