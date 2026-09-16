import { Wifi, AlertTriangle, ChevronRight, Activity, Clock } from 'lucide-react';
import type { OperationalState } from '../../types/models';

export interface FleetStationCardDevice {
  device_id: string;
  label?: string | null;
  is_online?: boolean;
  last_seen?: string;
  firmware_version?: string;
  operational_state?: OperationalState;
}

export interface FleetStationCardSummary {
  crop?: string | null;
  ec_latest?: number | null;
  ph_latest?: number | null;
  warning_count: number;
}

export interface FleetStationCardProps {
  device: FleetStationCardDevice;
  summary?: FleetStationCardSummary;
  onSelect: (deviceId: string) => void;
}

/**
 * Format relative time in Vietnamese (e.g., "vừa xong", "5 phút trước", "2 giờ trước")
 */
export function formatRelativeTime(dateString?: string): string {
  if (!dateString) return 'Chưa có dữ liệu';
  const date = new Date(dateString);
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  if (isNaN(diffMs) || diffMs < 0) return 'Vừa xong';

  const diffSec = Math.floor(diffMs / 1000);
  const diffMin = Math.floor(diffSec / 60);
  const diffHour = Math.floor(diffMin / 60);
  const diffDay = Math.floor(diffHour / 24);

  if (diffSec < 60) return 'Vừa xong';
  if (diffMin < 60) return `${diffMin} phút trước`;
  if (diffHour < 24) return `${diffHour} giờ trước`;
  if (diffDay < 7) return `${diffDay} ngày trước`;
  return date.toLocaleDateString('vi-VN', { day: '2-digit', month: '2-digit' });
}

/**
 * Assess EC range status:
 * 1.0 - 2.5: optimal (green)
 * 0.7 - 3.0: drift/warning (amber)
 * < 0.7 or > 3.0: critical (red)
 */
function getEcStatusClass(ec: number | null | undefined): { bg: string; text: string; label: string } {
  if (ec === null || ec === undefined) {
    return { bg: 'bg-surface-muted', text: 'text-text-muted', label: 'Chưa có' };
  }
  if (ec >= 1.0 && ec <= 2.5) {
    return { bg: 'bg-pill', text: 'text-status', label: 'Tối ưu' };
  }
  if (ec >= 0.7 && ec <= 3.0) {
    return { bg: 'bg-amber-50 border border-amber-200', text: 'text-amber-800', label: 'Lệch' };
  }
  return { bg: 'bg-red-50 border border-red-200', text: 'text-red-700', label: 'Nguy hiểm' };
}

/**
 * Assess pH range status:
 * 5.5 - 6.5: optimal (green)
 * 5.0 - 7.0: drift/warning (amber)
 * < 5.0 or > 7.0: critical (red)
 */
function getPhStatusClass(ph: number | null | undefined): { bg: string; text: string; label: string } {
  if (ph === null || ph === undefined) {
    return { bg: 'bg-surface-muted', text: 'text-text-muted', label: 'Chưa có' };
  }
  if (ph >= 5.5 && ph <= 6.5) {
    return { bg: 'bg-pill', text: 'text-status', label: 'Tối ưu' };
  }
  if (ph >= 5.0 && ph <= 7.0) {
    return { bg: 'bg-amber-50 border border-amber-200', text: 'text-amber-800', label: 'Lệch' };
  }
  return { bg: 'bg-red-50 border border-red-200', text: 'text-red-700', label: 'Nguy hiểm' };
}

export function FleetStationCard({ device, summary, onSelect }: FleetStationCardProps) {
  const isOnline = device.operational_state?.contact === 'CONTACTED' && device.operational_state?.freshness === 'FRESH';
  const isOffline = device.operational_state?.contact === 'NOT_CONTACTED';
  const isUnknown = !isOnline && !isOffline;
  const warningCount = summary?.warning_count ?? 0;
  const label = device.label || device.device_id;
  const ec = summary?.ec_latest;
  const ph = summary?.ph_latest;
  const crop = summary?.crop;

  const ecStyle = getEcStatusClass(ec);
  const phStyle = getPhStatusClass(ph);

  const ariaLabel = `Trạm ${label}: ${isOnline ? 'Đang hoạt động' : isOffline ? 'Ngoại tuyến' : 'Chưa rõ trạng thái'}${
    warningCount > 0 ? `, ${warningCount} cảnh báo` : ''
  }${ec !== null && ec !== undefined ? `, EC ${ec.toFixed(1)}` : ''}${
    ph !== null && ph !== undefined ? `, pH ${ph.toFixed(1)}` : ''
  }`;

  return (
    <button
      type="button"
      onClick={() => onSelect(device.device_id)}
      aria-label={ariaLabel}
      className={`w-full min-h-[48px] text-left p-4 rounded-2xl border transition-all duration-150 flex flex-col justify-between group focus:outline-none focus:ring-2 focus:ring-primary focus:ring-offset-2 bg-white ${
        warningCount > 0
          ? 'border-amber-300 hover:border-amber-400 hover:shadow-md'
          : 'border-line hover:border-primary/40 hover:shadow-sm'
      }`}
    >
      {/* Top row: Status indicator, Name, Crop & Warnings */}
      <div>
        <div className="flex items-start justify-between gap-2 mb-1.5">
          <div className="flex items-center gap-2 min-w-0">
            <span
              className={`relative flex h-3 w-3 flex-shrink-0`}
              title={isOnline ? 'Trực tuyến' : isOffline ? 'Ngoại tuyến' : 'Chưa rõ trạng thái'}
            >
              {isOnline && (
                <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-status opacity-75" />
              )}
              <span
                className={`relative inline-flex rounded-full h-3 w-3 ${
                  isOnline ? 'bg-status' : isUnknown ? 'bg-text-muted' : 'bg-gray-300'
                }`}
              />
            </span>
            <h3 className="font-bold text-base text-primary-deep truncate group-hover:text-primary transition-colors">
              {label}
            </h3>
          </div>

          <div className="flex items-center gap-1.5 flex-shrink-0">
            {warningCount > 0 && (
              <span className="flex items-center gap-1 text-xs px-2 py-0.5 bg-red-50 text-red-700 border border-red-200 rounded-full font-bold">
                <AlertTriangle size={12} />
                {warningCount}
              </span>
            )}
            <ChevronRight
              size={18}
              className="text-line group-hover:text-primary transition-transform group-hover:translate-x-0.5"
            />
          </div>
        </div>

        {/* Subtitle: hardware ID & crop pill */}
        <div className="flex items-center gap-2 mb-3">
          <span className="text-xs text-faint font-mono">{device.device_id}</span>
          {crop && (
            <span className="text-xs px-2 py-0.5 bg-pill text-status font-medium rounded-full truncate max-w-[140px]">
              🌱 {crop}
            </span>
          )}
        </div>
      </div>

      {/* Center: EC and pH telemetry chips */}
      <div className="grid grid-cols-2 gap-2 my-2 pt-2 border-t border-line/60">
        <div className={`px-2.5 py-1.5 rounded-xl flex items-center justify-between ${ecStyle.bg}`}>
          <div className="flex items-center gap-1.5">
            <Activity size={13} className="text-text-muted opacity-80" />
            <span className="text-xs font-semibold text-text-muted">EC</span>
          </div>
          <span className={`text-xs font-bold ${ecStyle.text}`}>
            {ec !== null && ec !== undefined ? ec.toFixed(1) : '—'}
          </span>
        </div>

        <div className={`px-2.5 py-1.5 rounded-xl flex items-center justify-between ${phStyle.bg}`}>
          <div className="flex items-center gap-1.5">
            <span className="text-xs font-semibold text-text-muted">pH</span>
          </div>
          <span className={`text-xs font-bold ${phStyle.text}`}>
            {ph !== null && ph !== undefined ? ph.toFixed(1) : '—'}
          </span>
        </div>
      </div>

      {/* Footer: relative last seen & firmware */}
      <div className="flex items-center justify-between text-[11px] text-faint pt-2 mt-1">
        <div className="flex items-center gap-1">
          {isOnline ? (
            <span className="flex items-center gap-1 text-status font-medium">
              <Wifi size={12} />
              Trực tuyến
            </span>
          ) : isOffline ? (
            <span className="flex items-center gap-1">
              <Clock size={12} />
              {formatRelativeTime(device.last_seen)}
            </span>
          ) : (
            <span className="flex items-center gap-1 text-text-muted">
              <Clock size={12} />
              Chưa rõ trạng thái
            </span>
          )}
        </div>

        {device.firmware_version && (
          <span className="text-faint font-mono">FW: {device.firmware_version}</span>
        )}
      </div>
    </button>
  );
}
