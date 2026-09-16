import { useEffect, useState, useMemo } from 'react';
import { RefreshCw, Cpu, ArrowLeft, PlusCircle, AlertTriangle, Layers, Sprout } from 'lucide-react';
import { useFleetStatus, FleetDevice } from '../hooks/useFleetStatus';
import { useStationContext } from '../contexts/StationContext';
import { useNavigate } from 'react-router-dom';
import { apiGet } from '../lib/apiClient';
import { FleetStationCard, FleetStationCardSummary } from '../components/fleet';
import { routePath } from '../routes';

interface FleetSummaryEntry {
  device_id: string;
  crop: string | null;
  ec_latest: number | null;
  ph_latest: number | null;
  warning_count: number;
}

export function FleetView() {
  const { devices, loading, error, refresh } = useFleetStatus();
  const { selectDevice: setSelectedDevice } = useStationContext();
  const navigate = useNavigate();

  const [summaries, setSummaries] = useState<Record<string, FleetSummaryEntry>>({});
  const [filter, setFilter] = useState<'all' | 'warning'>('all');

  useEffect(() => {
    if (devices.length === 0) return;
    let cancelled = false;
    apiGet<{ data: FleetSummaryEntry[] }>('/fleet/summary')
      .then((res) => {
        if (cancelled) return;
        const map: Record<string, FleetSummaryEntry> = {};
        res.data.forEach((entry) => {
          map[entry.device_id] = entry;
        });
        setSummaries(map);
      })
      .catch(() => {
        if (!cancelled) setSummaries({});
      });
    return () => {
      cancelled = true;
    };
  }, [devices]);

  function selectDevice(deviceId: string) {
    setSelectedDevice(deviceId);
    navigate(routePath('dashboard'));
  }

  function goBack() {
    const historyIndex = window.history.state?.idx;
    if (typeof historyIndex === 'number' && historyIndex > 0) {
      navigate(-1);
      return;
    }
    navigate(routePath('dashboard'), { replace: true });
  }

  // Warning-first sort: stations with warning_count > 0 always surface to top
  const sortedAndFilteredDevices = useMemo(() => {
    const list = filter === 'warning'
      ? devices.filter((d) => (summaries[d.device_id]?.warning_count ?? 0) > 0)
      : [...devices];

    return list.sort((a, b) => {
      const warnB = summaries[b.device_id]?.warning_count ?? 0;
      const warnA = summaries[a.device_id]?.warning_count ?? 0;
      if (warnB !== warnA) {
        return warnB - warnA; // Descending warnings
      }
      // Tie-breaker: online stations first
      const onlineA = a.is_online ? 1 : 0;
      const onlineB = b.is_online ? 1 : 0;
      return onlineB - onlineA;
    });
  }, [devices, summaries, filter]);

  // Crop grouping when there are 4 or more devices
  const shouldGroup = sortedAndFilteredDevices.length >= 4;

  const groupedDevices = useMemo(() => {
    if (!shouldGroup) {
      return [{ groupName: null, items: sortedAndFilteredDevices }];
    }

    const groups: Record<string, FleetDevice[]> = {};
    for (const d of sortedAndFilteredDevices) {
      const crop = summaries[d.device_id]?.crop?.trim();
      const groupKey = crop ? crop : 'Chưa thiết lập cây trồng';
      if (!groups[groupKey]) {
        groups[groupKey] = [];
      }
      groups[groupKey].push(d);
    }

    return Object.entries(groups).map(([groupName, items]) => ({
      groupName,
      items,
    }));
  }, [sortedAndFilteredDevices, summaries, shouldGroup]);

  // Total warning count badge for filter tab
  const totalWarningCount = useMemo(() => {
    return devices.reduce((acc, d) => acc + (summaries[d.device_id]?.warning_count ?? 0), 0);
  }, [devices, summaries]);

  return (
    <div className="app-page">
      {/* Top Header */}
      <div className="page-header">
        <div className="page-header-main">
          <button
            onClick={goBack}
            aria-label="Quay lại"
            className="page-header-icon"
          >
            <ArrowLeft size={20} />
          </button>
          <div>
            <h1 className="page-header-title">
              Tổng Quan Thiết Bị
            </h1>
            <p className="page-header-subtitle">
              Theo dõi tình trạng kết nối, dinh dưỡng EC/pH và cảnh báo theo thời gian thực.
            </p>
          </div>
        </div>

        <div className="flex items-center gap-2">
          <button
            onClick={refresh}
            disabled={loading}
            className="flex items-center gap-1.5 px-3 py-2 border border-line rounded-xl text-xs font-semibold text-primary-deep bg-white hover:bg-soft transition-colors shadow-sm"
          >
            <RefreshCw size={14} className={loading ? 'animate-spin' : ''} />
            <span>Làm mới</span>
          </button>

          <button
            onClick={() => navigate(routePath('pairing'))}
            className="flex items-center gap-1.5 px-3.5 py-2 rounded-xl text-xs font-bold bg-primary text-white hover:bg-primary-deep transition-all shadow-sm"
          >
            <PlusCircle size={14} />
            <span>Thêm trạm mới</span>
          </button>
        </div>
      </div>

      {/* Filter and overview metrics bar */}
      <div className="flex flex-wrap items-center justify-between gap-3 mb-5 p-1 bg-surface-muted rounded-2xl border border-line">
        <div className="flex items-center gap-1">
          <button
            onClick={() => setFilter('all')}
            className={`flex items-center gap-2 px-3.5 py-1.5 rounded-xl text-xs font-bold transition-all ${
              filter === 'all'
                ? 'bg-primary-deep text-white shadow-sm'
                : 'text-text-muted hover:text-primary-deep'
            }`}
          >
            <Layers size={13} />
            <span>Tất cả ({devices.length})</span>
          </button>

          <button
            onClick={() => setFilter('warning')}
            className={`flex items-center gap-2 px-3.5 py-1.5 rounded-xl text-xs font-bold transition-all ${
              filter === 'warning'
                ? 'bg-red-700 text-white shadow-sm'
                : 'text-text-muted hover:text-red-700'
            }`}
          >
            <AlertTriangle size={13} />
            <span>Cần chú ý</span>
            {totalWarningCount > 0 && (
              <span
                className={`text-[10px] px-1.5 py-0.2 rounded-full font-extrabold ${
                  filter === 'warning'
                    ? 'bg-white text-red-700'
                    : 'bg-red-100 text-red-700'
                }`}
              >
                {totalWarningCount}
              </span>
            )}
          </button>
        </div>

        <div className="text-xs text-text-muted px-3 py-1 flex items-center gap-3">
          <span>
            Trực tuyến: <strong className="text-status">{devices.filter((d) => d.is_online).length}</strong>/{devices.length}
          </span>
          {shouldGroup && (
            <span className="hidden sm:inline text-faint">
              (Đang phân nhóm theo cây trồng)
            </span>
          )}
        </div>
      </div>

      {error && (
        <div className="mb-6 p-4 bg-red-50 text-red-700 border border-red-200 rounded-xl text-sm flex items-center gap-2">
          <AlertTriangle size={16} className="flex-shrink-0" />
          <span>{error}</span>
        </div>
      )}

      {/* Loading Skeletons */}
      {loading && devices.length === 0 ? (
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3.5">
          {[1, 2, 3, 4, 5, 6].map((i) => (
            <div key={i} className="h-44 bg-line/60 rounded-2xl animate-pulse" />
          ))}
        </div>
      ) : sortedAndFilteredDevices.length === 0 ? (
        /* Empty States */
        <div className="text-center py-16 px-4 bg-white rounded-2xl border border-line shadow-sm max-w-lg mx-auto mt-6">
          <div className="w-16 h-16 rounded-full bg-pill flex items-center justify-center mx-auto mb-4 text-status">
            {filter === 'warning' ? <Sprout size={32} /> : <Cpu size={32} />}
          </div>

          <h3 className="text-lg font-bold text-primary-deep mb-1">
            {filter === 'warning'
              ? 'Tất cả trạm hoạt động tối ưu'
              : 'Chưa có trạm nào'}
          </h3>

          <p className="text-sm text-text-muted mb-6 max-w-sm mx-auto">
            {filter === 'warning'
              ? 'Không có trạm nào ghi nhận cảnh báo EC, pH hay mất kết nối trong hệ thống.'
              : 'Tài khoản của bạn chưa liên kết trạm thủy canh nào. Bắt đầu ngay bằng cách ghép nối trạm đầu tiên.'}
          </p>

          {filter === 'warning' ? (
            <button
              onClick={() => setFilter('all')}
              className="px-4 py-2 rounded-xl text-xs font-bold bg-pill text-status hover:bg-pill/80 transition-colors"
            >
              Xem tất cả trạm
            </button>
          ) : (
            <button
              onClick={() => navigate(routePath('pairing'))}
              className="px-4 py-2.5 rounded-xl text-sm font-bold bg-primary text-white hover:bg-primary-deep transition-all shadow-sm"
            >
              Liên kết thiết bị mới
            </button>
          )}
        </div>
      ) : (
        /* Grouped or Flat Responsive Cards */
        <div className="space-y-6">
          {groupedDevices.map((group, groupIdx) => (
            <section key={group.groupName ?? `ungrouped-${groupIdx}`}>
              {group.groupName && (
                <div className="flex items-center gap-2 mb-3 px-1">
                  <span className="text-base">🌱</span>
                  <h2 className="text-sm md:text-base font-bold text-primary-deep">
                    {group.groupName}
                  </h2>
                  <span className="text-xs px-2 py-0.5 rounded-full bg-pill text-status font-semibold">
                    {group.items.length} trạm
                  </span>
                </div>
              )}

              <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3.5">
                {group.items.map((d) => (
                  <FleetStationCard
                    key={d.device_id}
                    device={d}
                    summary={summaries[d.device_id] as FleetStationCardSummary | undefined}
                    onSelect={selectDevice}
                  />
                ))}
              </div>
            </section>
          ))}
        </div>
      )}
    </div>
  );
}
