import { useEffect, useState } from 'react';
import { RefreshCw, Wifi, WifiOff, ChevronRight, Cpu, ArrowLeft, PlusCircle } from 'lucide-react';
import { useFleetStatus } from '../hooks/useFleetStatus';
import { useDeviceStore } from '../store/useDeviceStore';
import { useNavigate } from 'react-router-dom';
import { apiGet } from '../lib/apiClient';

export function FleetView() {
  const { devices, loading, error, refresh } = useFleetStatus();
  const setDeviceId = useDeviceStore((s) => s.setDeviceId);
  const navigate = useNavigate();

  interface FleetSummaryEntry {
    device_id: string;
    crop: string | null;
    ec_latest: number | null;
    ph_latest: number | null;
    warning_count: number;
  }
  const [summaries, setSummaries] = useState<Record<string, FleetSummaryEntry>>({});
  const [filter, setFilter] = useState<'all' | 'warning'>('all');

  useEffect(() => {
    if (devices.length === 0) return;
    let cancelled = false;
    apiGet<{ data: FleetSummaryEntry[] }>('/fleet/summary')
      .then((res) => {
        if (cancelled) return;
        const map: Record<string, FleetSummaryEntry> = {};
        res.data.forEach((entry) => { map[entry.device_id] = entry; });
        setSummaries(map);
      })
      .catch(() => { if (!cancelled) setSummaries({}); });
    return () => { cancelled = true; };
  }, [devices]);

  function selectDevice(deviceId: string) {
    setDeviceId(deviceId);
    navigate('/');
  }

  const filteredDevices = filter === 'warning'
    ? devices.filter((d) => (summaries[d.device_id]?.warning_count ?? 0) > 0)
    : devices;

  return (
    <div className="max-w-3xl mx-auto p-6">
      <div className="flex items-center justify-between mb-2">
        <div className="flex items-center gap-2">
          <button
            onClick={() => navigate(-1)}
            aria-label="Quay lại"
            className="flex items-center gap-1 px-2.5 py-1.5 border border-line rounded-lg text-sm text-primary-deep hover:bg-soft"
          >
            <ArrowLeft size={14} />
          </button>
          <h1 className="text-2xl font-bold text-primary-deep">Tổng Quan Thiết Bị</h1>
        </div>
        <button
          onClick={refresh}
          disabled={loading}
          className="flex items-center gap-2 px-3 py-1.5 border border-line rounded-lg text-sm text-primary-deep hover:bg-soft"
        >
          <RefreshCw size={14} className={loading ? 'animate-spin' : ''} />
          Làm mới
        </button>
      </div>
      <p className="text-sm text-text-muted mb-4">
        Trạng thái và thông số trực quan của tất cả trạm đã liên kết với tài khoản của bạn.
      </p>

      <div className="flex flex-wrap items-center justify-between gap-3 mb-4">
        <div className="flex gap-1.5">
          {(['all', 'warning'] as const).map((f) => {
            const active = filter === f;
            return (
              <button
                key={f}
                onClick={() => setFilter(f)}
                className={`px-4 py-1.5 rounded-xl text-xs font-bold transition-all border ${
                  active
                    ? 'bg-primary-deep text-white border-transparent shadow-md'
                    : 'bg-white text-text-muted border-line hover:bg-pill'
                }`}
              >
                {f === 'all' ? 'Tất cả' : 'Có cảnh báo trước'}
              </button>
            );
          })}
        </div>
        <button
          onClick={() => navigate('/pairing')}
          className="flex items-center gap-1.5 px-3 py-1.5 rounded-xl text-xs font-bold bg-pill text-primary-deep border border-line hover:bg-soft transition-colors"
        >
          <PlusCircle size={14} />
          Liên kết thiết bị mới
        </button>
      </div>

      {error && (
        <div className="mb-4 p-3 bg-red-50 text-red-700 rounded-lg text-sm">{error}</div>
      )}

      {loading && devices.length === 0 ? (
        <div className="space-y-3">
          {[1, 2, 3].map((i) => (
            <div key={i} className="h-20 bg-line rounded-xl animate-pulse" />
          ))}
        </div>
      ) : (
        <div className="space-y-3">
          {filteredDevices.length === 0 ? (
            <div className="text-center py-12 text-faint">
              <Cpu className="mx-auto mb-3" size={40} />
              <p>{filter === 'warning' ? 'Không có thiết bị nào đang cảnh báo.' : 'Chưa có thiết bị nào được liên kết.'}</p>
              {filter !== 'warning' && (
                <button
                  onClick={() => navigate('/pairing')}
                  className="mt-2 text-sm font-semibold text-primary hover:text-primary-deep"
                >
                  Liên kết thiết bị mới
                </button>
              )}
            </div>
          ) : (
            filteredDevices.map((d) => (
              <button
                key={d.device_id}
                onClick={() => selectDevice(d.device_id)}
                className="w-full flex items-center gap-4 p-4 border border-line rounded-xl hover:bg-surface-muted transition text-left"
              >
                {/* Online indicator */}
                <div className={`w-10 h-10 rounded-full flex items-center justify-center flex-shrink-0 ${
                  d.is_online ? 'bg-pill' : 'bg-surface-muted'
                }`}>
                  {d.is_online
                    ? <Wifi size={18} className="text-status" />
                    : <WifiOff size={18} className="text-faint" />
                  }
                </div>

                {/* Device info */}
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2">
                    <p className="font-medium truncate">{d.label ?? d.device_id}</p>
                    {summaries[d.device_id]?.crop && (
                      <span className="text-xs px-2 py-0.5 bg-pill text-status rounded-full">
                        🌱 {summaries[d.device_id].crop}
                      </span>
                    )}
                    {(summaries[d.device_id]?.warning_count ?? 0) > 0 && (
                      <span className="text-xs px-2 py-0.5 bg-red-50 text-red-700 border border-red-200 rounded-full">
                        ⚠ {summaries[d.device_id].warning_count}
                      </span>
                    )}
                  </div>
                  <p className="text-xs text-faint">{d.device_id}</p>

                  <div className="flex items-center gap-2 mt-1">
                    {(() => {
                      const s = summaries[d.device_id];
                      const ec = s?.ec_latest;
                      const ph = s?.ph_latest;
                      return (
                        <>
                          {ec !== undefined && ec !== null && (
                            <span className="text-[11px] px-1.5 py-0.5 rounded bg-surface-muted text-text-muted font-medium">
                              EC {ec.toFixed(1)}
                            </span>
                          )}
                          {ph !== undefined && ph !== null && (
                            <span className="text-[11px] px-1.5 py-0.5 rounded bg-surface-muted text-text-muted font-medium">
                              pH {ph.toFixed(1)}
                            </span>
                          )}
                          {((ec === undefined || ec === null) && (ph === undefined || ph === null)) && (
                            <span className="text-[11px] text-faint">EC — · pH —</span>
                          )}
                        </>
                      );
                    })()}
                  </div>

                  {d.firmware_version && (
                    <p className="text-xs text-faint mt-0.5">FW: {d.firmware_version}</p>
                  )}
                </div>

                {/* Status badge + arrow */}
                <div className="flex items-center gap-2 flex-shrink-0">
                  <span className={`text-xs px-2 py-0.5 rounded-full font-medium ${
                    d.is_online
                      ? 'bg-pill text-status'
                      : 'bg-surface-muted text-faint'
                  }`}>
                    {d.is_online ? 'Online' : 'Offline'}
                  </span>
                  {d.last_seen && !d.is_online && (
                    <span className="text-xs text-faint">
                      {new Date(d.last_seen).toLocaleString('vi-VN', { dateStyle: 'short', timeStyle: 'short' })}
                    </span>
                  )}
                  <ChevronRight size={16} className="text-line" />
                </div>
              </button>
            ))
          )}
        </div>
      )}
    </div>
  );
}
