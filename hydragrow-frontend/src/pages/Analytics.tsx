// src/pages/Analytics.tsx
import { useState, useEffect } from 'react';
import {
  LineChart as ChartIcon,
  ExternalLink,
  Activity,
  Server,
  Cpu,
  Wifi,
  Clock,
  Mail,
  SlidersHorizontal,
  RefreshCw,
} from 'lucide-react';
import { useQuery } from '@tanstack/react-query';
import toast from 'react-hot-toast';
import { Link } from 'react-router-dom';
import { PageHeader } from '../components/ui/PageHeader';
import { SubCard } from '../components/ui/SubCard';
import { useDeviceStore } from '../store/useDeviceStore';
import { loadAppSettings } from '../platform/settings';
import { apiGet, apiPut } from '../lib/apiClient';
import { useWhoami } from '../hooks/useWhoami';

export interface DeviceHealthMetrics {
  free_heap_bytes: number | null;
  wifi_rssi_dbm: number | null;
  uptime_seconds: number | null;
  backend_process_cpu_percent: number | null;
  last_updated_at: string | null;
}

export function formatUptime(seconds: number | null | undefined): string {
  if (seconds === null || seconds === undefined || seconds <= 0) return '—';
  const days = Math.floor(seconds / 86400);
  const hours = Math.floor((seconds % 86400) / 3600);
  const mins = Math.floor((seconds % 3600) / 60);
  if (days > 0) return `${days}d ${hours}h`;
  if (hours > 0) return `${hours}h ${mins}m`;
  return `${mins}m`;
}

export function formatHeap(bytes: number | null | undefined): string {
  if (bytes === null || bytes === undefined) return '—';
  return `${(bytes / 1024).toFixed(1)} KB`;
}

export function formatRssi(rssi: number | null | undefined): string {
  if (rssi === null || rssi === undefined) return '—';
  return `${rssi.toFixed(0)} dBm`;
}

const Analytics = ({ variant = 'standalone' }: { variant?: 'standalone' | 'embedded' }) => {
  const deviceId = useDeviceStore((s) => s.deviceId);
  const [grafanaUrl, setGrafanaUrl] = useState<string>('');
  const [weeklyReport, setWeeklyReport] = useState<boolean>(false);
  const [isUpdatingPref, setIsUpdatingPref] = useState(false);

  const { data: whoami, refetch: refetchWhoami } = useWhoami();

  useEffect(() => {
    loadAppSettings().then((settings) => {
      if (settings?.grafana_url) {
        setGrafanaUrl(String(settings.grafana_url).trim());
      }
    });
  }, []);

  useEffect(() => {
    if (whoami?.preferences && typeof whoami.preferences === 'object') {
      setWeeklyReport(Boolean((whoami.preferences as Record<string, unknown>).weekly_report));
    }
  }, [whoami]);

  const {
    data: health,
    isLoading: isHealthLoading,
    refetch: refetchHealth,
  } = useQuery<DeviceHealthMetrics>({
    queryKey: ['device-health', deviceId],
    queryFn: async () => {
      if (!deviceId) {
        return {
          free_heap_bytes: null,
          wifi_rssi_dbm: null,
          uptime_seconds: null,
          backend_process_cpu_percent: null,
          last_updated_at: null,
        };
      }
      return apiGet<DeviceHealthMetrics>(`/devices/${deviceId}/analytics/health`);
    },
    enabled: Boolean(deviceId),
    refetchInterval: 15000,
  });

  const handleToggleWeeklyReport = async (enabled: boolean) => {
    setWeeklyReport(enabled);
    setIsUpdatingPref(true);
    try {
      const nextPrefs = {
        ...(whoami?.preferences || {}),
        weekly_report: enabled,
      };
      await apiPut('/admin/me/preferences', { preferences: nextPrefs });
      toast.success(
        enabled
          ? 'Đã kích hoạt gửi báo cáo tuần về email của bạn'
          : 'Đã tắt tính năng nhận báo cáo tuần'
      );
      refetchWhoami();
    } catch {
      setWeeklyReport(!enabled);
      toast.error('Không thể cập nhật tuỳ chọn báo cáo tuần');
    } finally {
      setIsUpdatingPref(false);
    }
  };

  return (
    <div className={variant === 'embedded' ? 'space-y-6' : 'app-page space-y-6'}>
      {variant !== 'embedded' && (
        <PageHeader
          icon={ChartIcon}
          title="Phân tích & Giám sát Hệ thống"
          subtitle="Tình trạng phần cứng vi điều khiển, báo cáo vận hành tuần và bảng điều khiển Grafana"
        />
      )}

      {/* HARDWARE HEALTH CARDS (TASK C8) */}
      <div className="grid grid-cols-2 lg:grid-cols-4 gap-4" data-testid="health-cards-grid">
        <div className="ui-card p-4 space-y-1">
          <div className="flex items-center justify-between text-text-muted">
            <span className="text-xs font-medium">RAM Vi điều khiển</span>
            <Cpu size={16} className="text-purple-600" />
          </div>
          <div className="text-xl font-bold text-primary-deep" data-testid="health-free-heap">
            {isHealthLoading ? '...' : formatHeap(health?.free_heap_bytes)}
          </div>
          <p className="text-[11px] text-text-muted">Bộ nhớ heap khả dụng ESP32</p>
        </div>

        <div className="ui-card p-4 space-y-1">
          <div className="flex items-center justify-between text-text-muted">
            <span className="text-xs font-medium">Tín hiệu WiFi</span>
            <Wifi size={16} className="text-emerald-600" />
          </div>
          <div className="text-xl font-bold text-primary-deep" data-testid="health-wifi-rssi">
            {isHealthLoading ? '...' : formatRssi(health?.wifi_rssi_dbm)}
          </div>
          <p className="text-[11px] text-text-muted">Cường độ sóng kết nối trạm</p>
        </div>

        <div className="ui-card p-4 space-y-1">
          <div className="flex items-center justify-between text-text-muted">
            <span className="text-xs font-medium">Thời gian chạy (Uptime)</span>
            <Clock size={16} className="text-sky-600" />
          </div>
          <div className="text-xl font-bold text-primary-deep" data-testid="health-uptime">
            {isHealthLoading ? '...' : formatUptime(health?.uptime_seconds)}
          </div>
          <p className="text-[11px] text-text-muted">Thời gian trạm hoạt động liên tục</p>
        </div>

        <div className="ui-card p-4 space-y-1">
          <div className="flex items-center justify-between text-text-muted">
            <span className="text-xs font-medium">Tải Backend CPU</span>
            <Server size={16} className="text-amber-600" />
          </div>
          <div className="text-xl font-bold text-primary-deep" data-testid="health-cpu">
            {isHealthLoading
              ? '...'
              : health?.backend_process_cpu_percent !== null &&
                health?.backend_process_cpu_percent !== undefined
              ? `${health.backend_process_cpu_percent.toFixed(1)}%`
              : '—'}
          </div>
          <p className="text-[11px] text-text-muted">Mức sử dụng CPU tiến trình backend</p>
        </div>
      </div>

      {/* WEEKLY REPORT PREFERENCE TOGGLE (TASK D3) */}
      <div className="ui-card p-4 flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4">
        <div className="flex items-start gap-3">
          <div className="p-2 rounded-xl bg-pill text-status mt-0.5">
            <Mail size={20} />
          </div>
          <div>
            <h3 className="text-sm font-bold text-primary-deep">
              Báo cáo vận hành tuần qua email
            </h3>
            <p className="text-xs text-text-muted mt-0.5">
              Bản tin tóm tắt biến động EC, pH, lượng nước tiêu thụ và lịch châm phân sẽ được gửi tới email của bạn vào 7:00 sáng thứ Hai.
            </p>
          </div>
        </div>
        <div className="flex items-center gap-3 self-end sm:self-center">
          <label className="relative inline-flex items-center cursor-pointer">
            <input
              type="checkbox"
              aria-label="Báo cáo tuần qua email"
              checked={weeklyReport}
              disabled={isUpdatingPref}
              onChange={(e) => handleToggleWeeklyReport(e.target.checked)}
              className="sr-only peer"
            />
            <div className="w-11 h-6 bg-surface-muted peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-line after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-primary"></div>
          </label>
          <span className="text-xs font-semibold text-primary-deep min-w-[56px]">
            {weeklyReport ? 'Đã bật' : 'Tắt'}
          </span>
        </div>
      </div>

      {/* GRAFANA EMBEDDED DASHBOARD GATE (TASK D3) */}
      {grafanaUrl ? (
        <div className="ui-card space-y-3" data-testid="grafana-frame-container">
          <div className="flex items-center justify-between border-b border-line pb-3">
            <div className="flex items-center gap-2">
              <Activity size={18} className="text-primary" />
              <h2 className="farm-section-title">Bảng điều khiển chuỗi thời gian Grafana</h2>
            </div>
            <div className="flex items-center gap-2">
              <button
                type="button"
                onClick={() => refetchHealth()}
                className="p-1.5 rounded-lg border border-line text-text-muted hover:bg-soft"
                title="Làm mới số liệu"
              >
                <RefreshCw size={15} />
              </button>
              <a
                href={grafanaUrl}
                target="_blank"
                rel="noopener noreferrer"
                className="inline-flex items-center gap-1.5 px-3 py-1.5 text-xs font-semibold rounded-lg bg-soft text-primary-deep hover:bg-pill hover:text-status transition-colors"
              >
                <ExternalLink size={14} /> Mở tab riêng
              </a>
            </div>
          </div>

          <div className="rounded-2xl overflow-hidden border border-line bg-gray-950 shadow-inner">
            <iframe
              src={grafanaUrl}
              title="Grafana Dashboard"
              className="w-full h-[640px] border-none"
              data-testid="grafana-iframe"
            />
          </div>
        </div>
      ) : (
        <div
          className="ui-card p-8 text-center space-y-4 border-dashed"
          data-testid="grafana-placeholder"
        >
          <div className="w-12 h-12 rounded-2xl bg-surface-muted flex items-center justify-center mx-auto text-primary">
            <SlidersHorizontal size={24} />
          </div>
          <div className="max-w-md mx-auto space-y-1">
            <h3 className="text-base font-bold text-primary-deep">
              Chưa thiết lập URL Grafana Dashboard
            </h3>
            <p className="text-xs text-text-muted leading-relaxed">
              Bạn có thể nhúng trực tiếp bảng điều khiển Grafana (biểu đồ chuỗi thời gian, ma trận MIMO và Kalman) vào trang này bằng cách cấu hình trong mục Cài đặt.
            </p>
          </div>
          <div>
            <Link
              to="/settings"
              className="ui-btn-primary inline-flex items-center gap-2 text-xs"
            >
              <SlidersHorizontal size={15} /> Đi tới Cài đặt &gt; Tích hợp để thêm URL
            </Link>
          </div>
        </div>
      )}

      {/* METRIC GROUPS EXPLANATION */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
        <SubCard title="Trung tâm Giám sát Grafana">
          <div className="space-y-4 text-sm text-primary-deep">
            <p className="leading-relaxed">
              Các biểu đồ chi tiết về biến động EC, pH, nhiệt độ, mực nước, hệ số tự học EMA Gain,
              ma trận tương tác MIMO và độ tin cậy Kalman hiện được giám sát trực tiếp trên hạ tầng
              Grafana/Prometheus chuyên dụng.
            </p>
            <a
              href={grafanaUrl || 'http://localhost:3000'}
              target="_blank"
              rel="noopener noreferrer"
              className="inline-flex items-center gap-2 px-5 py-2.5 bg-primary hover:bg-primary-deep text-white rounded-xl font-medium transition-all shadow-sm text-xs"
            >
              <ExternalLink size={14} />
              <span>Mở Grafana Dashboard</span>
            </a>
          </div>
        </SubCard>

        <SubCard title="Các nhóm Metrics chính trên Grafana">
          <div className="space-y-3 text-xs text-primary-deep">
            <div className="flex items-center gap-2.5 p-2.5 bg-surface-muted rounded-lg border border-line">
              <Activity size={16} className="text-primary shrink-0" />
              <span>
                <b>Adaptive Learning:</b> Gain, Step Ratio, Tuner State &amp; Tolerance.
              </span>
            </div>
            <div className="flex items-center gap-2.5 p-2.5 bg-surface-muted rounded-lg border border-line">
              <Server size={16} className="text-sky-700 shrink-0" />
              <span>
                <b>MIMO Matrix &amp; Kalman:</b> Độ tin cậy cơ cấu chấp hành &amp; trạng thái ma trận.
              </span>
            </div>
            <div className="flex items-center gap-2.5 p-2.5 bg-surface-muted rounded-lg border border-line">
              <Cpu size={16} className="text-purple-700 shrink-0" />
              <span>
                <b>ESP32 Telemetry:</b> Free Heap, WiFi RSSI, Uptime &amp; Log Drop Count.
              </span>
            </div>
          </div>
        </SubCard>
      </div>
    </div>
  );
};

export default Analytics;
