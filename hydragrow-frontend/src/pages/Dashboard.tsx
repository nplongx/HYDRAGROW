import { useMemo } from 'react';
import {
  Droplets, Thermometer, Activity, Waves, Settings, Zap, Cpu,
  LineChart, ArrowRight, AlertTriangle, Clock3
} from 'lucide-react';
import { eval_sensor_status_safe } from '../../gleam_core/build/dev/javascript/gleam_core/dashboard.mjs';
import { extract_fault_code_str, friendly_state, compute_health_safe } from '../../gleam_core/build/dev/javascript/gleam_core/fsm.mjs';
import { get_fault_guide } from '../../gleam_core/build/dev/javascript/gleam_core/faults.mjs';
import { useNavigate, Link } from 'react-router-dom';
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
import type { TelemetryQuality } from '../types/models';
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

const Dashboard = () => {
  const { status: stationStatus, selectedDeviceId: deviceId } = useStationContext();
  const {
    data: authoritativeTelemetry,
    isLoading: isTelemetryLoading,
    error: telemetryError,
  } = useDeviceTelemetry(deviceId);
  const { data: settings } = useDeviceConfig(deviceId);
  const availability = authoritativeTelemetry?.availability ?? 'UNKNOWN';
  const operationalState = authoritativeTelemetry?.operational_state;
  const isOnline = operationalState?.contact === 'CONTACTED' && operationalState?.freshness === 'FRESH';
  const isOffline = operationalState?.contact === 'NOT_CONTACTED';
  const controllerHealth = authoritativeTelemetry?.controller_health ?? null;
  const fsmState = authoritativeTelemetry?.fsm?.state ?? 'Unknown';
  const isLoading = isTelemetryLoading;
  const isSensorOnline = operationalState?.freshness === 'FRESH';
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
  const { forceOn } = useDeviceControl(deviceId ?? '');
  const { permission, enableNotifications } = useFCM();
  const { data: healthSummary } = useSystemHealthSummary(deviceId ?? '');
  const { shouldShowOnboarding } = useOnboardingState();
  const dosingTotalCount = (healthSummary?.ec_dosing_count ?? 0) + (healthSummary?.ph_dosing_count ?? 0);

  const displayName = user?.displayName?.trim() || user?.email?.split('@')[0] || undefined;
  const greetingName = displayName ? (displayName[0].toUpperCase() + displayName.slice(1)) : '';

  const friendlyState = useMemo(() => {
    if (!operationalState || operationalState.contact === 'UNKNOWN' || operationalState.freshness === 'UNKNOWN') {
      return { label: 'Chưa rõ trạng thái', description: 'Chưa có bằng chứng đủ mới về trạng thái trạm.', type: 'warning' as const };
    }
    if (operationalState.freshness === 'STALE') {
      return { label: 'Dữ liệu đã cũ', description: 'Chưa nhận được quan sát vận hành đủ mới.', type: 'warning' as const };
    }
    const res = friendly_state(fsmState || 'Monitoring', isOnline);
    return { label: res.label, description: res.description, type: res.tone as any };
  }, [operationalState, fsmState, isOnline]);

  const computedHealth = useMemo(() => {
    if (availability === 'UNKNOWN') {
      return { score: null, label: 'Chưa rõ', color: 'text-faint', description: 'Chưa có dữ liệu health hiện tại.' };
    }
    const rawScore = controllerHealth?.health_score_percent ?? controllerHealth?.diagnostics?.health_score_percent;
    const scoreInt = typeof rawScore === 'number' ? Math.round(rawScore) : -1;
    const res = compute_health_safe(isOnline, scoreInt);
    return { score: res.score, label: res.label, color: res.color, description: res.description };
  }, [availability, controllerHealth, isOnline]);

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
  const modeLabel = settings?.control_mode === 'auto' ? 'Tự động' : 'Thủ công';

  const ecStatus = sensorStatus(ecAxis?.quality, ecAxis?.value, getTdsSetting(settings, 'min_ec_limit', 'min_ec_limit'), getTdsSetting(settings, 'max_ec_limit', 'max_ec_limit'));
  const phStatus = sensorStatus(phAxis?.quality, phAxis?.value, settings?.min_ph_limit, settings?.max_ph_limit);
  const tempStatus = sensorStatus(tempAxis?.quality, tempAxis?.value, settings?.min_temp_limit, settings?.max_temp_limit);
  const waterStatus = sensorStatus(waterAxis?.quality, waterAxis?.value, settings?.water_level_min, settings?.water_level_max);

  const nextAction = availability === 'UNKNOWN'
    ? 'Chưa đủ dữ liệu để xác định trạng thái trạm.'
    : isOffline
    ? 'Kiểm tra nguồn Wi-Fi trạm điều khiển.'
    : !isSensorOnline
      ? 'Đang mất tín hiệu cảm biến. Kiểm tra nguồn node cảm biến.'
      : faultGuide?.action || (permission !== 'granted' ? 'Bật thông báo để nhận cảnh báo tức thì.' : 'Không cần thao tác. Tiếp tục theo dõi.');

  const hasTankAlert = Boolean(
    tankAlert && (tankAlert.tank_a_low || tankAlert.tank_b_low || tankAlert.tank_ph_down_low || tankAlert.tank_ph_up_low)
  );

  const hasActionableIssue = Boolean(faultCode) || isOffline || isSensorUnknown || !isSensorOnline;
  const isCritical = isOffline;

  return (
    <div className="app-page">
      <PageHeader
        title="Tổng quan"
        subtitle={friendlyState.description}
        icon={Activity}
        action={
          <DeviceStatePill
            state={isOnline ? 'online' : isOffline ? 'offline' : 'warning'}
            label={isOnline ? 'Trạm Online' : isOffline ? 'Trạm Offline' : 'Trạng thái chưa rõ'}
          />
        }
      />

      <section aria-labelledby="dashboard-station-status" className="ui-card relative overflow-hidden p-6 md:p-8">
        <h2 id="dashboard-station-status" className="sr-only">Trạng thái trạm</h2>
        <div className="flex flex-col lg:flex-row lg:items-center justify-between gap-6">
          <div className="space-y-4 max-w-2xl">
            <div className="flex flex-wrap items-center gap-2">
              <DeviceStatePill
                state={isOnline ? 'online' : isOffline ? 'offline' : 'unknown'}
                label={isOnline ? 'Trạm Online' : isOffline ? 'Trạm Offline' : 'Trạng thái chưa rõ'}
              />
              <span className="farm-status-pill bg-soft text-text-muted border-line">
                <Cpu size={13} />
                {modeLabel}
              </span>
              <Link
                to={routePath('fleet')}
                title="Quản lý các thiết bị đã liên kết"
                className="farm-status-pill bg-surface text-text-muted border-line hover:bg-soft transition-colors"
              >
                ID: {deviceId}
              </Link>
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
                isSensorUnknown ? 'text-faint' : isSensorOnline ? 'text-status' : 'text-error'
              }`}>
                {isSensorUnknown ? 'Chưa rõ' : isSensorOnline ? 'Tốt' : 'Mất'}
              </div>
              <p className="text-xs font-semibold text-primary-deep mt-2">
                {isSensorUnknown ? 'Chưa đủ dữ liệu' : isSensorOnline ? 'Đang đo' : 'Cần kiểm tra'}
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
      />

      {shouldShowOnboarding && <OnboardingWizard className="mb-6" />}

      {/* Sensor Bento Grid */}
      <div className="space-y-3">
        <div className="flex items-center justify-between">
          <h2 className="farm-section-title">
            <LineChart size={14} />
            <span>Thông số thời gian thực</span>
          </h2>
        </div>
        <div className={`grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-4 gap-4 transition-all duration-500 ${!isSensorOnline ? 'opacity-60 grayscale' : ''}`}>
          <SensorBentoCard
            title="Dinh dưỡng EC"
            value={ecAxis?.quality === 'ERROR' ? "Lỗi" : ecAxis?.quality === 'INVALID' ? "Không hợp lệ" : ecAxis?.quality === 'STALE' ? "Cũ" : ecAxis?.quality === 'VALID' ? formatNumber(ecAxis.value, 2) : "Chưa có dữ liệu"}
            unit={ecAxis?.quality === 'VALID' ? "ppm" : ""}
            icon={Activity}
            theme={ecAxis?.quality === 'ERROR' || ecAxis?.quality === 'INVALID' ? "rose" : "blue"}
            statusLabel={ecStatus.label}
            statusTone={ecStatus.tone}
            rangeLabel={`Mục tiêu ${formatNumber(getTdsSetting(settings, 'ec_target', 'ec_target'), 2)} ± ${formatNumber(getTdsSetting(settings, 'ec_tolerance', 'ec_tolerance'), 2)}`}
            description={ecAxis?.quality === 'ERROR' ? 'Lỗi cảm biến EC.' : 'Nồng độ dinh dưỡng bồn chứa.'}
            sparkline={ecAxis?.quality === 'VALID' ? Math.max(0, Math.min(100, (Number(ecAxis.value) / (Number(getTdsSetting(settings, 'ec_max_limit', 'ec_max_limit')) || 1)) * 100)) : undefined}
          />
          <SensorBentoCard
            title="Độ pH"
            value={phAxis?.quality === 'ERROR' ? "Lỗi" : phAxis?.quality === 'INVALID' ? "Không hợp lệ" : phAxis?.quality === 'STALE' ? "Cũ" : phAxis?.quality === 'VALID' ? formatNumber(phAxis.value, 2) : "Chưa có dữ liệu"}
            unit=""
            icon={Droplets}
            theme={phAxis?.quality === 'ERROR' || phAxis?.quality === 'INVALID' ? "rose" : "fuchsia"}
            statusLabel={phStatus.label}
            statusTone={phStatus.tone}
            rangeLabel={`Mục tiêu ${formatNumber((settings as any)?.ph_target, 2)} ± ${formatNumber((settings as any)?.ph_tolerance, 2)}`}
            description={phAxis?.quality === 'ERROR' ? 'Cần hiệu chuẩn pH.' : 'Độ cân bằng axit/kiềm.'}
            sparkline={phAxis?.quality === 'VALID' ? Math.max(0, Math.min(100, (Number(phAxis.value) / 14) * 100)) : undefined}
          />
          <SensorBentoCard
            title="Nhiệt độ"
            value={tempAxis?.quality === 'ERROR' ? "Lỗi" : tempAxis?.quality === 'INVALID' ? "Không hợp lệ" : tempAxis?.quality === 'STALE' ? "Cũ" : tempAxis?.quality === 'VALID' ? formatNumber(tempAxis.value, 1) : "Chưa có dữ liệu"}
            unit={tempAxis?.quality === 'VALID' ? "°C" : ""}
            icon={Thermometer}
            theme={tempAxis?.quality === 'ERROR' || tempAxis?.quality === 'INVALID' ? "rose" : "orange"}
            statusLabel={tempStatus.label}
            statusTone={tempStatus.tone}
            rangeLabel={`An toàn ${formatNumber((settings as any)?.min_temp_limit, 0)}-${formatNumber((settings as any)?.max_temp_limit, 0)}°C`}
            description={tempAxis?.quality === 'ERROR' ? 'Lỗi cảm biến nhiệt độ.' : 'Nhiệt độ dung dịch bồn chứa.'}
            sparkline={tempAxis?.quality === 'VALID' ? Math.max(0, Math.min(100, (Number(tempAxis.value) / 50) * 100)) : undefined}
          />
          <SensorBentoCard
            title="Mực nước"
            value={waterAxis?.quality === 'ERROR' ? "Lỗi phao" : waterAxis?.quality === 'INVALID' ? "Không hợp lệ" : waterAxis?.quality === 'STALE' ? "Cũ" : waterAxis?.quality === 'VALID' ? formatNumber(waterAxis.value, 0) : "Chưa có dữ liệu"}
            unit={waterAxis?.quality === 'VALID' ? "%" : ""}
            icon={Waves}
            theme={waterAxis?.quality === 'ERROR' || waterAxis?.quality === 'INVALID' ? "rose" : "cyan"}
            statusLabel={waterStatus.label}
            statusTone={waterStatus.tone}
            rangeLabel={`Giữ quanh ${formatNumber((settings as any)?.water_level_target, 0)}%`}
            description={waterAxis?.quality === 'ERROR' ? 'Kiểm tra phao siêu âm.' : 'Đảm bảo bơm không chạy khô.'}
            sparkline={waterAxis?.quality === 'VALID' ? Math.max(0, Math.min(100, Number(waterAxis.value))) : undefined}
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

      <DosingSummaryCard
        totalCount={dosingTotalCount}
        lastDosedAt={healthSummary?.latest_ph_dosing_at ?? null}
      />

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

export default Dashboard;
