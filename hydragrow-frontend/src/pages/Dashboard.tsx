import { useMemo, useState } from 'react';
import {
  Droplets, Thermometer, Activity, Waves, Settings, Zap, Cpu,
  LineChart, ArrowRight, AlertTriangle, Clock3
} from 'lucide-react';
import { eval_sensor_status_safe } from '../../gleam_core/build/dev/javascript/gleam_core/dashboard.mjs';
import { extract_fault_code_str, friendly_state, compute_health_safe } from '../../gleam_core/build/dev/javascript/gleam_core/fsm.mjs';
import { get_fault_guide } from '../../gleam_core/build/dev/javascript/gleam_core/faults.mjs';
import { useNavigate, Link, useLocation } from 'react-router-dom';
import { SensorBentoCard } from '../components/ui/SensorBentoCard';
import { QuickActionBar } from '../components/ui/QuickActionBar';
import { DosingSummaryCard } from '../components/ui/DosingSummaryCard';
import { LoadingState } from '../components/ui/LoadingState';
import { Banner } from '../components/ui/Banner';
import { Badge } from '../components/ui/Badge';
import { Button } from '../components/ui/Button';
import { DeviceStatePill } from '../components/ui/DeviceStatePill';
import { HealthScore } from '../components/ui/HealthScore';
import { PageHeader } from '../components/ui/PageHeader';
import { StateView } from '../components/ui/StateView';
import { useFCM } from '../hooks/useFCM';
import { useSystemHealthSummary } from '../hooks/useSystemHealthSummary';
import { useDeviceControl } from '../hooks/useDeviceControl';
import { useAuth } from '../contexts/AuthContext';
import { pumpLabels, pumpColors } from '../lib/pumpLabels';
import { EmergencyStopButton } from '../components/safety/EmergencyStopButton';
import { OnboardingWizard } from '../components/onboarding';
import { useOnboardingState } from '../hooks/useOnboardingState';
import { useStationContext } from '../contexts/StationContext';
import { useDeviceTelemetry } from '../hooks/useDeviceTelemetry';
import { useDeviceConfig } from '../hooks/useDeviceConfig';
import { useSystemEvents } from '../hooks/useSystemEvents';
import { useDashboardFleet, type DashboardFleetStation } from '../hooks/useDashboardFleet';
import type { TelemetryQuality } from '../types/models';
import { normalizeStationCardState } from '../contracts/stationCard';
import { routePath } from '../routes';

const ActiveDeviceTag = ({ label, color }: { label: string; color: string }) => (
  <span className={`flex items-center gap-1.5 px-3 py-1 rounded-full text-xs font-semibold tracking-wide border ${color}`}>
    <Zap size={12} className="fill-current" />
    {label}
  </span>
);

const formatEventTime = (timestampMs: number) =>
  new Date(timestampMs).toLocaleString('vi-VN', {
    hour: '2-digit',
    minute: '2-digit',
    day: '2-digit',
    month: '2-digit',
  });

const formatNumber = (value: any, digits = 1) => {
  const num = Number(value);
  if (!Number.isFinite(num)) return '--';
  return num.toFixed(digits);
};

const getTdsSetting = (settings: any, ecKey: string, legacyEcKey: string) => settings?.[ecKey] ?? settings?.[legacyEcKey];

const sensorStatus = (quality: TelemetryQuality | undefined, value: unknown, min?: any, max?: any) => {
  if (quality === 'ERROR') return { label: 'Lỗi', tone: 'danger' as const };
  if (quality === 'INVALID') return { label: 'Không hợp lệ', tone: 'danger' as const };
  if (quality === 'STALE') return { label: 'Cũ', tone: 'warn' as const };
  if (quality !== 'VALID') return { label: 'Chưa có dữ liệu', tone: 'info' as const };
  const res = eval_sensor_status_safe(
    false,
    String(value ?? ''),
    String(min ?? ''),
    String(max ?? '')
  );
  return { label: res.label, tone: res.tone as 'good' | 'warn' | 'danger' | 'info' };
};

const DashboardStationCard = ({
  station,
  onSelect,
}: {
  station: DashboardFleetStation;
  onSelect: (deviceId: string) => void;
}) => {
  const label = station.label?.trim() || station.device_id;
  const state = normalizeStationCardState(station);
  const online = state.connection === 'ONLINE';
  const offline = state.connection === 'OFFLINE';
  const warningUnknown = !state.warning.known;
  const warning = warningUnknown || station.warning_count > 0;

  return (
    <button
      type="button"
      onClick={() => onSelect(station.device_id)}
      aria-label={'Mở trạm ' + label}
      className={
        'w-full text-left rounded-2xl border bg-white p-4 transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-2 ' +
        (warning ? 'border-warning/60 hover:border-warning' : 'border-line hover:border-primary/40')
      }
    >
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0">
          <h2 className="text-base font-bold text-primary-deep truncate">{label}</h2>
          <p className="mt-1 text-xs font-mono text-faint truncate">{station.device_id}</p>
        </div>
        <DeviceStatePill
          state={online ? 'online' : offline ? 'offline' : 'warning'}
          label={online ? 'Online' : offline ? 'Offline' : 'Chưa rõ'}
        />
      </div>
      <div className="mt-4 grid grid-cols-2 gap-2 text-sm">
        <div className="rounded-xl bg-surface-muted p-3">
          <span className="block text-[11px] font-semibold text-text-muted">EC</span>
          <strong className="mt-1 block text-primary-deep">
            {station.ec_quality === 'ERROR' ? 'Lỗi' : station.ec_quality === 'INVALID' ? 'Không hợp lệ' : station.ec_quality === 'STALE' ? 'Cũ' : station.ec_quality === 'VALID' ? station.ec_latest ?? '—' : '—'}
          </strong>
        </div>
        <div className="rounded-xl bg-surface-muted p-3">
          <span className="block text-[11px] font-semibold text-text-muted">pH</span>
          <strong className="mt-1 block text-primary-deep">
            {station.ph_quality === 'ERROR' ? 'Lỗi' : station.ph_quality === 'INVALID' ? 'Không hợp lệ' : station.ph_quality === 'STALE' ? 'Cũ' : station.ph_quality === 'VALID' ? station.ph_latest ?? '—' : '—'}
          </strong>
        </div>
      </div>
      <div className="mt-3 flex flex-wrap items-center gap-2 text-xs">
        {station.crop && (
          <span className="rounded-full bg-pill px-2.5 py-1 font-semibold text-status">{station.crop}</span>
        )}
        {warningUnknown ? (
          <span className="rounded-full bg-warning-bg px-2.5 py-1 font-semibold text-warn-deep">
            Cảnh báo: chưa xác định
          </span>
        ) : warning ? (
          <span className="rounded-full bg-warning-bg px-2.5 py-1 font-semibold text-warn-deep">
            {station.warning_count} cảnh báo
          </span>
        ) : (
          <span className="text-text-muted">Không có cảnh báo trong dữ liệu tổng hợp</span>
        )}
      </div>
      {station.telemetry_freshness !== 'FRESH' && (station.telemetry_observed_at || station.last_seen) && (
        <p className="mt-3 text-xs text-text-muted">
          Telemetry {station.telemetry_freshness === 'STALE' ? 'cũ' : 'chưa xác định'} · Lần cuối quan sát: {new Date(station.telemetry_observed_at ?? station.last_seen!).toLocaleString('vi-VN')}
        </p>
      )}
    </button>
  );
};

const AllStationsOverview = () => {
  const navigate = useNavigate();
  const { selectDevice } = useStationContext();
  const { stations, isLoading, isFetching, error, refresh } = useDashboardFleet();
  const [filter, setFilter] = useState<'all' | 'warning'>('all');

  const visibleStations = useMemo(() => {
    const filtered =
        filter === 'warning'
        ? stations.filter((station) => station.warning_count_known === false || station.warning_count > 0)
        : stations;
    return [...filtered].sort((a, b) => {
      const attentionRank = (station: DashboardFleetStation) => {
        const state = normalizeStationCardState(station);
        if (state.operational.actuator === 'UNKNOWN' || state.operational.actuator === 'CONTRADICTORY') return 0;
        if (state.connection === 'UNKNOWN' || state.connection === 'STALE') return 1;
        if (state.operational.readiness === 'UNKNOWN' || state.operational.readiness === 'NOT_READY') return 1;
        if (!state.warning.known) return 1;
        if (state.warning.count > 0) return 3;
        return 5;
      };
      const rankDiff = attentionRank(a) - attentionRank(b);
      if (rankDiff !== 0) return rankDiff;
      if (b.warning_count !== a.warning_count) return b.warning_count - a.warning_count;
      return (a.label || a.device_id).localeCompare(b.label || b.device_id, 'vi');
    });
  }, [filter, stations]);

  const onlineCount = stations.filter((station) => normalizeStationCardState(station).connection === 'ONLINE').length;
  const knownWarningCount = stations.reduce((sum, station) => sum + station.warning_count, 0);
  const unknownWarningStations = stations.filter((station) => station.warning_count_known === false).length;

  const selectStation = (deviceId: string) => {
    selectDevice(deviceId);
    navigate(routePath('dashboard'), { state: { dashboardView: 'detail' } });
  };

  return (
    <div className="app-page">
      <PageHeader
        title="Tổng quan"
        subtitle="Theo dõi các trạm có thể truy cập, xác định trạm cần chú ý rồi mở chi tiết trạm."
        icon={Activity}
        action={
          <div className="flex flex-wrap items-center gap-2">
            <span className="farm-status-pill bg-soft text-text-muted border-line">{stations.length} trạm</span>
            <Button size="sm" variant="secondary" onClick={() => void refresh()} disabled={isFetching}>
              {isFetching ? 'Đang làm mới…' : 'Làm mới'}
            </Button>
            <Button size="sm" onClick={() => navigate(routePath('pairing'))}>Thêm trạm</Button>
          </div>
        }
      />

      <section aria-label="Tóm tắt đội trạm" className="grid grid-cols-1 sm:grid-cols-3 gap-3">
        <div className="ui-card"><span className="ui-overline">Tổng số trạm</span><strong className="mt-2 block text-2xl text-primary-deep">{stations.length}</strong></div>
        <div className="ui-card"><span className="ui-overline">Đang online</span><strong className="mt-2 block text-2xl text-status">{onlineCount}</strong></div>
        <div className="ui-card"><span className="ui-overline">Cảnh báo</span><strong className="mt-2 block text-2xl text-warn-deep">{knownWarningCount}{unknownWarningStations > 0 ? ' + ?' : ''}</strong></div>
      </section>

      <section aria-labelledby="dashboard-monitoring-controls" className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h2 id="dashboard-monitoring-controls" className="farm-section-title">Giám sát trạm</h2>
          <p className="mt-1 text-xs text-text-muted">Ưu tiên trạm có cảnh báo để kiểm tra trước.</p>
        </div>
        <div className="flex rounded-xl border border-line bg-surface-muted p-1" role="group" aria-label="Bộ lọc trạm">
          <button type="button" aria-pressed={filter === 'all'} onClick={() => setFilter('all')} className={'rounded-lg px-3 py-1.5 text-xs font-semibold ' + (filter === 'all' ? 'bg-white text-primary-deep shadow-sm' : 'text-text-muted')}>Tất cả ({stations.length})</button>
          <button type="button" aria-pressed={filter === 'warning'} onClick={() => setFilter('warning')} className={'rounded-lg px-3 py-1.5 text-xs font-semibold ' + (filter === 'warning' ? 'bg-white text-warn-deep shadow-sm' : 'text-text-muted')}>Cần chú ý ({stations.filter((station) => station.warning_count_known === false || station.warning_count > 0).length})</button>
        </div>
      </section>

      {isLoading ? (
        <div className="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-3 gap-4" aria-label="Đang tải danh sách trạm">
          {[1, 2, 3, 4, 5, 6].map((item) => <div key={item} className="h-48 rounded-2xl bg-line/50 animate-pulse" />)}
        </div>
      ) : error ? (
        <StateView icon={AlertTriangle} tone="danger" title="Không thể tải danh sách trạm" description="Không thể lấy dữ liệu tổng hợp của các trạm. Không thay thế bằng dữ liệu giả." action={<Button size="sm" variant="secondary" onClick={() => void refresh()}>Thử lại</Button>} />
      ) : stations.length === 0 ? (
        <StateView icon={Settings} title="Chưa có trạm có thể truy cập" description="Liên kết một trạm để bắt đầu theo dõi." action={<Button size="sm" onClick={() => navigate(routePath('pairing'))}>Thêm trạm</Button>} />
      ) : visibleStations.length === 0 ? (
        <StateView icon={AlertTriangle} tone="info" title="Không có trạm cần chú ý" description="Bộ lọc hiện tại không có trạm phù hợp." action={<Button size="sm" variant="secondary" onClick={() => setFilter('all')}>Xem tất cả</Button>} />
      ) : (
        <div className="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-3 gap-4" data-testid="dashboard-station-grid">
          {visibleStations.map((station) => (
            <DashboardStationCard key={station.device_id} station={station} onSelect={selectStation} />
          ))}
        </div>
      )}
    </div>
  );
};

const SelectedStationDetail = () => {
  const { status: stationStatus, selectedDeviceId: deviceId, selectedDevice } = useStationContext();
  const {
    data: authoritativeTelemetry,
    isLoading: isTelemetryLoading,
    error: telemetryError,
  } = useDeviceTelemetry(deviceId);
  const { data: settings } = useDeviceConfig(deviceId);
  const availability = authoritativeTelemetry?.availability ?? 'UNKNOWN';
  const operationalState = authoritativeTelemetry?.operational_state;
  const isConnected = operationalState?.contact === 'CONTACTED';
  const isTelemetryFresh = operationalState?.freshness === 'FRESH';
  const isTelemetryStale = operationalState?.freshness === 'STALE';
  const isOffline = operationalState?.contact === 'NOT_CONTACTED';
  const controllerHealth = authoritativeTelemetry?.controller_health ?? null;
  const fsmState = authoritativeTelemetry?.fsm?.state ?? 'Unknown';
  const isLoading = isTelemetryLoading;
  const isSensorLost = operationalState?.contact === 'NOT_CONTACTED';
  const isSensorUnknown = operationalState?.freshness === 'UNKNOWN' || !operationalState;
  const { data: systemEvents = [] } = useSystemEvents(deviceId);
  const tankAlert = useMemo(() => {
    const alert = systemEvents.find(
      (event) => event.category === 'alert' && (
        event.metadata?.reason === 'tank_level_alert' || event.metadata?.tank_a_low !== undefined
      ),
    );
    const details = alert?.metadata;
    if (!details) return null;
    return {
      tank_a_low: Boolean(details.tank_a_low),
      tank_b_low: Boolean(details.tank_b_low),
      tank_ph_down_low: Boolean(details.tank_ph_down_low),
      tank_ph_up_low: Boolean(details.tank_ph_up_low),
    };
  }, [systemEvents]);

  const navigate = useNavigate();
  const { user } = useAuth();
  const { forceOn, commandStatus } = useDeviceControl(deviceId ?? '');
  const { permission, enableNotifications } = useFCM();
  const { data: healthSummary, isLoading: isHealthSummaryLoading, isError: isHealthSummaryError } = useSystemHealthSummary(deviceId ?? '');
  const { shouldShowOnboarding } = useOnboardingState();
  const dosingTotalCount = healthSummary
    ? healthSummary.ec_dosing_count + healthSummary.ph_dosing_count
    : null;

  const displayName = user?.displayName?.trim() || user?.email?.split('@')[0] || undefined;
  const greetingName = displayName ? (displayName[0].toUpperCase() + displayName.slice(1)) : '';

  const friendlyState = useMemo(() => {
    if (!operationalState || operationalState.contact === 'UNKNOWN' || operationalState.freshness === 'UNKNOWN') {
      return { label: 'Chưa rõ trạng thái', description: 'Chưa có bằng chứng đủ mới về trạng thái trạm.', type: 'warning' as const };
    }
    if (operationalState.freshness === 'STALE') {
      return { label: 'Dữ liệu đã cũ', description: 'Chưa nhận được quan sát vận hành đủ mới.', type: 'warning' as const };
    }
    const res = friendly_state(fsmState || 'Monitoring', isConnected);
    return { label: res.label, description: res.description, type: res.tone as any };
  }, [operationalState, fsmState, isConnected]);

  const computedHealth = useMemo(() => {
    if (availability === 'UNKNOWN') {
      return { score: null, label: 'Chưa rõ', color: 'text-faint', description: 'Chưa có dữ liệu health hiện tại.' };
    }
    const rawScore = controllerHealth?.health_score_percent ?? controllerHealth?.diagnostics?.health_score_percent;
    const scoreInt = typeof rawScore === 'number' ? Math.round(rawScore) : -1;
    if (isTelemetryStale || !isTelemetryFresh) {
      return {
        score: null,
        label: 'Chưa đủ dữ liệu',
        color: 'text-faint',
        description: isTelemetryStale
          ? 'Telemetry đã cũ; không dùng dữ liệu cũ để kết luận sức khỏe hiện tại.'
          : 'Chưa có telemetry đủ mới để kết luận sức khỏe hiện tại.',
      };
    }
    const res = compute_health_safe(isConnected, scoreInt);
    return { score: res.score, label: res.label, color: res.color, description: res.description };
  }, [availability, controllerHealth, isConnected, isTelemetryFresh, isTelemetryStale]);

  if (stationStatus === 'LoadingSelection' || isLoading || isTelemetryLoading) {
    return <LoadingState message="Đang tải dữ liệu trạm thông minh..." />;
  }

  if (stationStatus !== 'Selected' || !deviceId) {
    return (
      <StateView
        icon={Settings}
        tone={stationStatus === 'PermissionDenied' || stationStatus === 'Unavailable' ? 'danger' : 'warning'}
        title={
          stationStatus === 'InvalidSelection'
            ? 'Thiết bị không còn khả dụng'
            : stationStatus === 'PermissionDenied'
              ? 'Không có quyền truy cập'
              : stationStatus === 'Unavailable'
                ? 'Không thể tải danh sách trạm'
                : 'Chưa chọn thiết bị'
        }
        description={
          stationStatus === 'InvalidSelection'
            ? 'Thiết bị đã chọn không còn nằm trong danh sách trạm có thể truy cập.'
            : stationStatus === 'PermissionDenied'
              ? 'Bạn không có quyền truy cập danh sách trạm.'
              : stationStatus === 'Unavailable'
                ? 'Không thể tải danh sách trạm. Vui lòng thử lại.'
                : 'Vui lòng chọn một trạm từ danh sách thiết bị.'
        }
        className="min-h-[60vh]"
      />
    );
  }

  if (telemetryError || !authoritativeTelemetry) {
    return (
      <StateView
        icon={AlertTriangle}
        tone="danger"
        title="Không có dữ liệu telemetry"
        description="Không thể đọc trạng thái hiện tại của trạm. Dữ liệu không được thay bằng giá trị mặc định."
        className="min-h-[60vh]"
      />
    );
  }

  const faultCode = extract_fault_code_str(fsmState || '');
  const faultGuideOpt = faultCode ? get_fault_guide(faultCode) : null;
  const faultGuide = faultGuideOpt && (faultGuideOpt as any)[0] ? (faultGuideOpt as any)[0] : null;

  const pumps: any = authoritativeTelemetry?.actuator?.pump_status || {};
  const telemetryAxis = (name: 'ec' | 'ph' | 'temp' | 'water_level') =>
    authoritativeTelemetry?.axes.find((axis) => axis.name === name);
  const ecAxis = telemetryAxis('ec');
  const phAxis = telemetryAxis('ph');
  const tempAxis = telemetryAxis('temp');
  const waterAxis = telemetryAxis('water_level');
  const modeLabel = settings?.control_mode === 'auto'
    ? 'Tự động'
    : settings?.control_mode === 'manual'
      ? 'Thủ công'
      : 'Chưa rõ';

  const ecStatus = sensorStatus(ecAxis?.quality, ecAxis?.value, getTdsSetting(settings, 'min_ec_limit', 'min_ec_limit'), getTdsSetting(settings, 'max_ec_limit', 'max_ec_limit'));
  const phStatus = sensorStatus(phAxis?.quality, phAxis?.value, settings?.min_ph_limit, settings?.max_ph_limit);
  const tempStatus = sensorStatus(tempAxis?.quality, tempAxis?.value, settings?.min_temp_limit, settings?.max_temp_limit);
  const waterStatus = sensorStatus(waterAxis?.quality, waterAxis?.value, settings?.water_level_min, settings?.water_level_max);

  const nextAction = availability === 'UNKNOWN'
    ? 'Chưa đủ dữ liệu để xác định trạng thái trạm.'
    : isOffline
    ? 'Kiểm tra nguồn Wi-Fi trạm điều khiển.'
    : isTelemetryStale
      ? 'Trạm vẫn kết nối nhưng telemetry đã cũ. Kiểm tra đường truyền hoặc node cảm biến nếu dữ liệu không cập nhật.'
      : isSensorUnknown
        ? 'Chưa đủ dữ liệu mới để xác định trạng thái cảm biến.'
      : faultGuide?.action || (permission !== 'granted' ? 'Bật thông báo để nhận cảnh báo tức thì.' : 'Không cần thao tác. Tiếp tục theo dõi.');

  const hasTankAlert = Boolean(
    tankAlert && (tankAlert.tank_a_low || tankAlert.tank_b_low || tankAlert.tank_ph_down_low || tankAlert.tank_ph_up_low)
  );

  const hasActionableIssue = Boolean(faultCode) || isOffline || isSensorUnknown || isTelemetryStale;
  const isCritical = isOffline;
  const waterCommandStatus = commandStatus.WATER_PUMP_IN;

  const withFreshness = (status: ReturnType<typeof sensorStatus>, quality?: TelemetryQuality) =>
    isTelemetryStale && quality === 'VALID'
      ? { ...status, label: 'Hợp lệ · Dữ liệu cũ', tone: 'warn' as const }
      : status;
  const displayedPhStatus = withFreshness(phStatus, phAxis?.quality);
  const displayedTempStatus = withFreshness(tempStatus, tempAxis?.quality);
  const displayedWaterStatus = withFreshness(waterStatus, waterAxis?.quality);
  const displayedEcStatusFinal = withFreshness(ecStatus, ecAxis?.quality);
  const physicalStateLabel = !isTelemetryFresh
    ? isTelemetryStale ? 'Chưa xác nhận · telemetry cũ' : 'Chưa xác nhận'
    : operationalState?.actuator !== 'KNOWN' || authoritativeTelemetry.actuator_contradictory
      ? 'Chưa xác nhận'
      : Object.values(pumps).some((value) => value === true) ? 'Đang chạy' : 'Đã dừng';

  return (
    <div className="app-page">
      <PageHeader
        title={selectedDevice?.label?.trim() || deviceId}
        subtitle={'Dashboard / Trạm đang chọn · ID: ' + deviceId + ' · ' + friendlyState.description}
        icon={Activity}
        action={
          <div className="flex flex-wrap items-center gap-2">
            <button
              type="button"
              onClick={() => navigate(routePath('dashboard'), { state: { dashboardView: 'overview' } })}
              className="ui-btn-outline"
            >
              ← Tất cả trạm
            </button>
            <DeviceStatePill
              state={isConnected ? 'online' : isOffline ? 'offline' : 'warning'}
              label={isConnected ? 'Online' : isOffline ? 'Offline' : 'Chưa rõ'}
            />
          </div>
        }
      />

      <section aria-label="An toàn và kết nối" className="flex flex-wrap items-center justify-between gap-3 rounded-2xl border border-line bg-surface-muted px-4 py-3">
        <div className="flex flex-wrap items-center gap-2 text-xs font-semibold">
          <span className="farm-status-pill bg-soft text-text-muted border-line">Kết nối: {isConnected ? 'Online' : isOffline ? 'Offline' : 'Chưa rõ'}</span>
          <span className="farm-status-pill bg-soft text-text-muted border-line">Trạng thái vật lý: {physicalStateLabel}</span>
          <span className={`farm-status-pill border-line ${isTelemetryStale ? 'bg-warning-bg text-warn-deep' : 'bg-soft text-text-muted'}`}>
            Telemetry: {isTelemetryFresh ? 'Mới' : isTelemetryStale ? 'Cũ' : 'Chưa rõ'}
          </span>
        </div>
        <span className="text-xs text-text-muted">Không suy diễn trạng thái vật lý từ kết nối.</span>
        <EmergencyStopButton deviceId={deviceId} variant="status" />
      </section>

      <section aria-labelledby="dashboard-station-status" className="ui-card relative overflow-hidden p-6 md:p-8">
        <h2 id="dashboard-station-status" className="sr-only">Trạng thái trạm</h2>
        <div className="flex flex-col lg:flex-row lg:items-center justify-between gap-6">
          <div className="space-y-4 max-w-2xl">
            <div className="flex flex-wrap items-center gap-2">
              <DeviceStatePill
                state={isConnected ? 'online' : isOffline ? 'offline' : 'unknown'}
                label={isConnected ? 'Trạm Online' : isOffline ? 'Trạm Offline' : 'Trạng thái chưa rõ'}
              />
              <span className="farm-status-pill bg-soft text-text-muted border-line">
                <Cpu size={13} />
                {modeLabel}
              </span>
              <span className="farm-status-pill bg-surface text-text-muted border-line">
                ID: {deviceId}
              </span>
            </div>
            <div>
              <h2 className="text-2xl md:text-3xl font-bold tracking-tight text-primary-deep">
                {greetingName ? `Xin chào, ${greetingName}` : friendlyState.label}
              </h2>
            </div>
            <Banner
              tone={isCritical ? 'danger' : hasActionableIssue ? 'warning' : 'info'}
              title={
                <span className="inline-flex items-center gap-2">
                  {isCritical ? 'KHẨN CẤP' : 'Hành động tiếp theo'}
                  <Badge tone={isCritical ? 'danger' : hasActionableIssue ? 'warning' : 'success'}>
                    {isCritical ? 'Sự cố hoạt động' : hasActionableIssue ? 'Ưu tiên' : 'Ổn định'}
                  </Badge>
                </span>
              }
              action={
                hasActionableIssue || permission !== 'granted' ? (
                  <div className="flex flex-col items-stretch gap-2">
                    {hasActionableIssue && (
                      <Button size="sm" onClick={() => navigate(routePath('operations'))}>
                        Mở Vận hành <ArrowRight size={12} />
                      </Button>
                    )}
                    {permission !== 'granted' && (
                      <Button size="sm" variant="secondary" onClick={enableNotifications}>
                        Bật quyền thông báo
                      </Button>
                    )}
                  </div>
                ) : undefined
              }
            >
              {nextAction}
            </Banner>
          </div>

          <div className="grid grid-cols-2 gap-3 w-full lg:w-72">
            <div className="rounded-2xl border border-line bg-surface-muted p-4">
              <span className="ui-overline">Sức khỏe trạm</span>
              <div className="mt-2 flex justify-center">
                <HealthScore score={computedHealth.score} label={computedHealth.label} />
              </div>
            </div>
            <div className="rounded-2xl border border-line bg-surface-muted p-4 text-center">
              <span className="ui-overline">Cảm biến</span>
              <div className={`text-2xl font-black mt-3 ${
                isSensorUnknown || isTelemetryStale ? 'text-faint' : isSensorLost ? 'text-error' : 'text-status'
              }`}>
                {isSensorUnknown ? 'Chưa rõ' : isSensorLost ? 'Mất kết nối' : isTelemetryStale ? 'Dữ liệu cũ' : 'Tốt'}
              </div>
              <p className="text-xs font-semibold text-primary-deep mt-2">
                {isSensorUnknown ? 'Chưa đủ dữ liệu' : isSensorLost ? 'Kiểm tra kết nối' : isTelemetryStale ? 'Không dùng để kết luận hiện tại' : 'Đang đo'}
              </p>
            </div>
          </div>
        </div>
      </section>

      {/* Cảnh báo cạn bình dung dịch */}
      {hasTankAlert && (
        <Banner tone="warning" title="Cảnh báo: Bình dung dịch sắp cạn" className="animate-in fade-in">
          <div className="flex flex-wrap gap-2 pt-0.5">
            {tankAlert?.tank_a_low && <Badge tone="warning">Cạn Dinh Dưỡng A</Badge>}
            {tankAlert?.tank_b_low && <Badge tone="warning">Cạn Dinh Dưỡng B</Badge>}
            {tankAlert?.tank_ph_up_low && <Badge tone="info">Cạn pH Up</Badge>}
            {tankAlert?.tank_ph_down_low && <Badge tone="danger">Cạn pH Down</Badge>}
          </div>
        </Banner>
      )}

      <QuickActionBar
        onWaterNow={() => forceOn('WATER_PUMP_IN', 30)}
        onDose={() => navigate(routePath('operations'))}
        onPausePumps={() => navigate(routePath('operations'))}
        onViewAlerts={() => navigate(routePath('journal'))}
        commandStatus={waterCommandStatus}
      />
      {(isTelemetryStale || isSensorUnknown) && (
        <p className="-mt-2 text-xs text-text-muted" role="note">
          {isTelemetryStale
            ? 'Có thể gửi lệnh, nhưng trạng thái vật lý chưa được xác nhận từ telemetry mới. Lệnh chỉ được coi là hoàn tất khi có xác nhận.'
            : 'Trạng thái trạm chưa đủ rõ để xác nhận cơ cấu chấp hành. Kiểm tra trước khi thực hiện thao tác ảnh hưởng thiết bị.'}
        </p>
      )}

      {shouldShowOnboarding && <OnboardingWizard className="mb-6" />}

      {/* Sensor Bento Grid */}
      <div className="space-y-3">
        <div className="flex items-center justify-between">
          <h2 className="farm-section-title">
            <LineChart size={14} />
            <span>Thông số thời gian thực</span>
          </h2>
        </div>
        <div className={`grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-4 gap-4 transition-all duration-500 ${isSensorLost || isSensorUnknown ? 'opacity-60 grayscale' : ''}`}>
          <SensorBentoCard
            title="Dinh dưỡng EC"
            value={ecAxis?.quality === 'ERROR' ? "Lỗi" : ecAxis?.quality === 'INVALID' ? "Không hợp lệ" : ecAxis?.quality === 'STALE' ? "Cũ" : ecAxis?.quality === 'VALID' ? formatNumber(ecAxis.value, 2) : "Chưa có dữ liệu"}
            unit=""
            icon={Activity}
            theme={ecAxis?.quality === 'ERROR' || ecAxis?.quality === 'INVALID' ? "rose" : "blue"}
            statusLabel={displayedEcStatusFinal.label}
            statusTone={displayedEcStatusFinal.tone}
            rangeLabel={`Mục tiêu ${formatNumber(getTdsSetting(settings, 'ec_target', 'ec_target'), 2)} ± ${formatNumber(getTdsSetting(settings, 'ec_tolerance', 'ec_tolerance'), 2)}`}
            description={ecAxis?.quality === 'ERROR' ? 'Lỗi cảm biến EC.' : 'Nồng độ dinh dưỡng bồn chứa.'}
            quality={ecAxis?.quality}
            observedAt={ecAxis?.observed_at ?? null}
          />
          <SensorBentoCard
            title="Độ pH"
            value={phAxis?.quality === 'ERROR' ? "Lỗi" : phAxis?.quality === 'INVALID' ? "Không hợp lệ" : phAxis?.quality === 'STALE' ? "Cũ" : phAxis?.quality === 'VALID' ? formatNumber(phAxis.value, 2) : "Chưa có dữ liệu"}
            unit=""
            icon={Droplets}
            theme={phAxis?.quality === 'ERROR' || phAxis?.quality === 'INVALID' ? "rose" : "fuchsia"}
            statusLabel={displayedPhStatus.label}
            statusTone={displayedPhStatus.tone}
            rangeLabel={`Mục tiêu ${formatNumber((settings as any)?.ph_target, 2)} ± ${formatNumber((settings as any)?.ph_tolerance, 2)}`}
            description={phAxis?.quality === 'ERROR' ? 'Cần hiệu chuẩn pH.' : 'Độ cân bằng axit/kiềm.'}
            quality={phAxis?.quality}
            observedAt={phAxis?.observed_at ?? null}
          />
          <SensorBentoCard
            title="Nhiệt độ"
            value={tempAxis?.quality === 'ERROR' ? "Lỗi" : tempAxis?.quality === 'INVALID' ? "Không hợp lệ" : tempAxis?.quality === 'STALE' ? "Cũ" : tempAxis?.quality === 'VALID' ? formatNumber(tempAxis.value, 1) : "Chưa có dữ liệu"}
            unit={tempAxis?.quality === 'VALID' ? "°C" : ""}
            icon={Thermometer}
            theme={tempAxis?.quality === 'ERROR' || tempAxis?.quality === 'INVALID' ? "rose" : "orange"}
            statusLabel={displayedTempStatus.label}
            statusTone={displayedTempStatus.tone}
            rangeLabel={`An toàn ${formatNumber((settings as any)?.min_temp_limit, 0)}-${formatNumber((settings as any)?.max_temp_limit, 0)}°C`}
            description={tempAxis?.quality === 'ERROR' ? 'Lỗi cảm biến nhiệt độ.' : 'Nhiệt độ dung dịch bồn chứa.'}
            quality={tempAxis?.quality}
            observedAt={tempAxis?.observed_at ?? null}
          />
          <SensorBentoCard
            title="Mực nước"
            value={waterAxis?.quality === 'ERROR' ? "Lỗi phao" : waterAxis?.quality === 'INVALID' ? "Không hợp lệ" : waterAxis?.quality === 'STALE' ? "Cũ" : waterAxis?.quality === 'VALID' ? formatNumber(waterAxis.value, 0) : "Chưa có dữ liệu"}
            unit={waterAxis?.quality === 'VALID' ? "%" : ""}
            icon={Waves}
            theme={waterAxis?.quality === 'ERROR' || waterAxis?.quality === 'INVALID' ? "rose" : "cyan"}
            statusLabel={displayedWaterStatus.label}
            statusTone={displayedWaterStatus.tone}
            rangeLabel={`Giữ quanh ${formatNumber((settings as any)?.water_level_target, 0)}%`}
            description={waterAxis?.quality === 'ERROR' ? 'Kiểm tra phao siêu âm.' : 'Đảm bảo bơm không chạy khô.'}
            quality={waterAxis?.quality}
            observedAt={waterAxis?.observed_at ?? null}
          />
        </div>
      </div>

      <section aria-labelledby="dashboard-active-operation" className="ui-card space-y-3">
        <h2 id="dashboard-active-operation" className="farm-section-title"><Zap size={14} /> Vận hành hiện tại</h2>
        <div className="flex flex-wrap items-center gap-2">
          <DeviceStatePill
            state={isCritical ? 'offline' : hasActionableIssue ? 'warning' : 'online'}
            label={friendlyState.label}
          />
          <span className="farm-status-pill bg-soft text-text-muted border-line">{modeLabel}</span>
        </div>
        <div className="flex flex-wrap gap-2">
          {authoritativeTelemetry?.actuator && Object.values(pumps).some(v => v === true) ? (
            Object.entries(pumps).map(([key, isRunning]) => {
              if (!isRunning) return null;
              return <ActiveDeviceTag key={key} label={pumpLabels[key] || key} color={pumpColors[key] || 'bg-pill text-status border-pill'} />;
            })
          ) : authoritativeTelemetry?.actuator ? (
            <span className="farm-status-pill bg-pill text-status border-pill">
              Không có bơm hoặc van nào đang chạy
            </span>
          ) : (
            <span className="farm-status-pill bg-soft text-text-muted border-line">
              Chưa có dữ liệu trạng thái bơm/van
            </span>
          )}
        </div>
      </section>

      {isHealthSummaryLoading ? (
        <StateView icon={Clock3} title="Đang tải dữ liệu châm dinh dưỡng" description="Chưa có dữ liệu health summary để hiển thị." />
      ) : isHealthSummaryError || !healthSummary || dosingTotalCount === null ? (
        <StateView icon={AlertTriangle} tone="danger" title="Không thể tải dữ liệu châm dinh dưỡng" description="Dữ liệu health summary không khả dụng. Không hiển thị số liệu mặc định." />
      ) : (
        <DosingSummaryCard
          totalCount={dosingTotalCount}
          lastDosedAt={healthSummary.latest_ph_dosing_at}
        />
      )}

      <section aria-labelledby="dashboard-recent-events" className="ui-card space-y-3">
        <div className="flex items-center justify-between gap-3">
          <h2 id="dashboard-recent-events" className="farm-section-title">
            <Clock3 size={14} />
            <span>Sự kiện gần đây</span>
          </h2>
          <Link
            to={routePath('journal')}
            className="text-xs font-semibold text-primary hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/40 rounded"
          >
            Mở Nhật ký
          </Link>
        </div>
        {systemEvents.length === 0 ? (
          <StateView
            icon={Clock3}
            title="Chưa có sự kiện gần đây"
            description="Chưa ghi nhận sự kiện mới cho trạm này."
          />
        ) : (
          <ul className="divide-y divide-line">
            {systemEvents.slice(0, 5).map((event) => (
              <li key={String(event.id ?? `${event.timestamp_ms}-${event.message}`)} className="flex items-start justify-between gap-4 py-3 first:pt-0 last:pb-0">
                <div className="min-w-0">
                  <p className="text-sm font-semibold text-primary-deep truncate">{event.message}</p>
                  <p className="mt-1 text-xs text-text-muted">{event.category} · {event.level}</p>
                </div>
                <time className="shrink-0 text-xs text-faint" dateTime={new Date(event.timestamp_ms).toISOString()}>
                  {formatEventTime(event.timestamp_ms)}
                </time>
              </li>
            ))}
          </ul>
        )}
      </section>

      <EmergencyStopButton deviceId={deviceId} variant="floating" />
    </div>
  );
};

const Dashboard = () => {
  const { selectedDeviceId, status } = useStationContext();
  const location = useLocation();
  const isOverview = location.state?.dashboardView === 'overview';
  return selectedDeviceId && status === 'Selected' && !isOverview
    ? <SelectedStationDetail />
    : <AllStationsOverview />;
};

export default Dashboard;
