import { useEffect, useState, useMemo } from 'react';
import { RefreshCw, Cpu, ArrowLeft, PlusCircle, AlertTriangle, Layers, Sprout, Map, Grid2X2, Download, Printer, GitCompare } from 'lucide-react';
import { useFleetStatus, FleetDevice } from '../hooks/useFleetStatus';
import { useStationContext } from '../contexts/StationContext';
import { useNavigate } from 'react-router-dom';
import { apiGet } from '../lib/apiClient';
import { FleetStationCard, FleetStationCardSummary } from '../components/fleet';
import { routePath } from '../routes';
import { configApi } from '../api/config';

interface FleetSummaryEntry {
  device_id: string;
  label?: string | null;
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
  const [viewMode, setViewMode] = useState<'grid' | 'map'>('grid');
  const [selectedForComparison, setSelectedForComparison] = useState<string[]>([]);
  const [comparison, setComparison] = useState<{
    data: FleetSummaryEntry[];
    same_crop: boolean;
    normalized: boolean;
  } | null>(null);

  const toggleComparison = (deviceId: string) => {
    setSelectedForComparison((current) =>
      current.includes(deviceId)
        ? current.filter((id) => id !== deviceId)
        : current.length < 4 ? [...current, deviceId] : current,
    );
  };

  async function compareSelected() {
    if (selectedForComparison.length < 2) return;
    const ids = encodeURIComponent(selectedForComparison.join(','));
    const result = await apiGet<{
      data: FleetSummaryEntry[];
      comparison: { same_crop: boolean; normalized: boolean };
    }>(`/fleet/compare?device_ids=${ids}`);
    setComparison({ data: result.data, ...result.comparison });
  }

  async function proposeAckThreshold(deviceId: string) {
    const raw = window.prompt('Nhập ngưỡng EC ACK mới. Giá trị phải > 0.');
    if (raw === null) return;
    const value = Number(raw);
    if (!Number.isFinite(value) || value <= 0) {
      window.alert('Ngưỡng không hợp lệ.');
      return;
    }
    const config = await configApi.getRaw(deviceId);
    await configApi.update(deviceId, {
      ...config,
      safety_config: { ...config.safety_config, ec_ack_threshold: value },
    });
  }

  function exportCsv() {
    const rows = devices.map((device) => {
      const summary = summaries[device.device_id];
      return [device.device_id, device.label ?? '', summary?.crop ?? '', summary?.ec_latest ?? '', summary?.ph_latest ?? '', summary?.warning_count ?? ''];
    });
    const csv = [['device_id', 'label', 'crop', 'ec_latest', 'ph_latest', 'warning_count'], ...rows]
      .map((row) => row.map((value) => `"${String(value).replace(/"/g, '""')}"`).join(','))
      .join('\n');
    const url = URL.createObjectURL(new Blob([csv], { type: 'text/csv;charset=utf-8' }));
    const anchor = document.createElement('a');
    anchor.href = url;
    anchor.download = `hydragrow-fleet-${new Date().toISOString().slice(0, 10)}.csv`;
    anchor.click();
    URL.revokeObjectURL(url);
  }

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
          <button type="button" onClick={() => setViewMode('grid')} aria-pressed={viewMode === 'grid'} className="px-3 py-2 border border-line rounded-xl text-xs font-semibold"><Grid2X2 size={14} /></button>
          <button type="button" onClick={() => setViewMode('map')} aria-pressed={viewMode === 'map'} className="px-3 py-2 border border-line rounded-xl text-xs font-semibold"><Map size={14} /></button>
          <button type="button" onClick={exportCsv} disabled={sortedAndFilteredDevices.length === 0} className="flex items-center gap-1.5 px-3 py-2 border border-line rounded-xl text-xs font-semibold"><Download size={14} /> CSV</button>
          <button type="button" onClick={() => window.print()} disabled={sortedAndFilteredDevices.length === 0} className="flex items-center gap-1.5 px-3 py-2 border border-line rounded-xl text-xs font-semibold"><Printer size={14} /> PDF</button>
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

      <div className="mb-5 p-3 rounded-2xl border border-line bg-white flex flex-wrap items-center gap-3">
        <span className="text-xs font-semibold text-text-muted">So sánh 2–4 trạm</span>
        <span className="text-xs text-text-muted">{selectedForComparison.length}/4 đã chọn</span>
        <button
          type="button"
          onClick={() => void compareSelected()}
          disabled={selectedForComparison.length < 2}
          className="ui-btn-primary flex items-center gap-1.5 text-xs disabled:opacity-50"
        >
          <GitCompare size={14} /> So sánh
        </button>
        {comparison && (
          <span className={`text-xs font-semibold ${comparison.normalized ? 'text-status' : 'text-text-muted'}`}>
            {comparison.same_crop ? 'Cùng crop/stage: có thể chuẩn hóa' : 'Khác crop/stage: không chuẩn hóa'}
          </span>
        )}
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
      ) : viewMode === 'map' ? (
        <div className="grid grid-cols-2 md:grid-cols-4 gap-3 min-h-72 p-4 rounded-2xl border border-line bg-surface-muted">
          {sortedAndFilteredDevices.map((device, index) => (
            <button key={device.device_id} type="button" onClick={() => selectDevice(device.device_id)} className="rounded-2xl border border-line bg-white p-4 text-left shadow-sm">
              <div className="text-[10px] uppercase text-text-muted">Vị trí {index + 1}</div>
              <div className="font-bold text-primary-deep mt-1">{device.label ?? device.device_id}</div>
              <div className="text-xs text-text-muted mt-2">{summaries[device.device_id]?.crop ?? 'Chưa có crop'}</div>
              <div className="mt-3 text-xs">EC {summaries[device.device_id]?.ec_latest ?? '--'} · pH {summaries[device.device_id]?.ph_latest ?? '--'}</div>
            </button>
          ))}
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
                  <div key={d.device_id} className="relative">
                    <label className="absolute right-3 top-3 z-10 flex items-center gap-1 rounded-lg bg-white/95 px-2 py-1 text-[10px] shadow-sm">
                      <input type="checkbox" checked={selectedForComparison.includes(d.device_id)} onChange={() => toggleComparison(d.device_id)} />
                      So sánh
                    </label>
                    <FleetStationCard
                      device={d}
                      summary={summaries[d.device_id] as FleetStationCardSummary | undefined}
                      onSelect={selectDevice}
                    />
                  </div>
                ))}
              </div>
            </section>
          ))}
        </div>
      )}

      {comparison && (
        <section className="mt-6 ui-card print:shadow-none">
          <div className="flex items-center justify-between mb-3">
            <h2 className="farm-section-title">So sánh trạm</h2>
            <button type="button" onClick={() => setComparison(null)} className="text-xs text-text-muted">Đóng</button>
          </div>
          <div className="overflow-x-auto">
            <table className="w-full text-xs">
              <thead><tr className="border-b border-line"><th className="text-left p-2">Trạm</th><th className="text-left p-2">Crop/stage</th><th className="text-left p-2">EC</th><th className="text-left p-2">pH</th><th className="text-left p-2">Ngưỡng</th></tr></thead>
              <tbody>
                {comparison.data.map((entry) => (
                  <tr key={entry.device_id} className="border-b border-line">
                    <td className="p-2 font-semibold">{entry.label ?? entry.device_id}</td>
                    <td className="p-2">{entry.crop ?? 'Thiếu dữ liệu'}</td>
                    <td className="p-2">{entry.ec_latest ?? 'Thiếu dữ liệu'}</td>
                    <td className="p-2">{entry.ph_latest ?? 'Thiếu dữ liệu'}</td>
                    <td className="p-2"><button type="button" onClick={() => void proposeAckThreshold(entry.device_id)} className="text-status font-semibold">Đề xuất ngưỡng EC</button></td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </section>
      )}
    </div>
  );
}
