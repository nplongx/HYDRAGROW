import { useMemo } from 'react';
import {
  Droplets, Thermometer, Activity, Waves, Settings, Zap, Cpu,
  LineChart, ArrowRight
} from 'lucide-react';
import { useDeviceStore } from '../store/useDeviceStore';
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
import { useFCM } from '../hooks/useFCM';
import { useSystemHealthSummary } from '../hooks/useSystemHealthSummary';
import { useDeviceControl } from '../hooks/useDeviceControl';
import { useAuth } from '../contexts/AuthContext';
import { pumpLabels, pumpColors } from '../lib/pumpLabels';
import { EmergencyStopButton } from '../components/safety/EmergencyStopButton';

const ActiveDeviceTag = ({ label, color }: { label: string; color: string }) => (
  <span className={`flex items-center gap-1.5 px-3 py-1 rounded-full text-xs font-semibold tracking-wide border ${color}`}>
    <Zap size={12} className="fill-current" />
    {label}
  </span>
);

const formatNumber = (value: any, digits = 1) => {
  const num = Number(value);
  if (!Number.isFinite(num)) return '--';
  return num.toFixed(digits);
};

const getTdsSetting = (settings: any, ecKey: string, legacyEcKey: string) => settings?.[ecKey] ?? settings?.[legacyEcKey];

const sensorStatus = (hasError: boolean | undefined, value: any, min?: any, max?: any) => {
  const res = eval_sensor_status_safe(
    Boolean(hasError),
    String(value ?? ''),
    String(min ?? ''),
    String(max ?? '')
  );
  return { label: res.label, tone: res.tone as 'good' | 'warn' | 'danger' | 'info' };
};

const Dashboard = () => {
  const deviceId = useDeviceStore((s) => s.deviceId);
  const sensorData = useDeviceStore((s) => s.sensorData);
  const isOnline = useDeviceStore((s) => s.deviceStatus.is_online);
  const controllerHealth = useDeviceStore((s) => s.controllerHealth);
  const fsmState = useDeviceStore((s) => s.fsmState);
  const isLoading = useDeviceStore((s) => s.isLoading);
  const isSensorOnline = useDeviceStore((s) => s.isSensorOnline);
  const settings = useDeviceStore((s) => s.settings);
  const tankAlert = useDeviceStore((s) => s.tankAlert);

  const navigate = useNavigate();
  const { user } = useAuth();
  const { forceOn } = useDeviceControl(deviceId ?? '');
  const { permission, enableNotifications } = useFCM();
  const { data: healthSummary } = useSystemHealthSummary(deviceId ?? '');
  const dosingTotalCount = (healthSummary?.ec_dosing_count ?? 0) + (healthSummary?.ph_dosing_count ?? 0);

  const displayName = user?.displayName?.trim() || user?.email?.split('@')[0] || undefined;
  const greetingName = displayName ? (displayName[0].toUpperCase() + displayName.slice(1)) : '';

  const friendlyState = useMemo(() => {
    const res = friendly_state(fsmState || 'Monitoring', isOnline);
    return { label: res.label, description: res.description, type: res.tone as any };
  }, [fsmState, isOnline]);

  const computedHealth = useMemo(() => {
    const rawScore = controllerHealth?.health_score_percent ?? controllerHealth?.diagnostics?.health_score_percent;
    const scoreInt = typeof rawScore === 'number' ? Math.round(rawScore) : -1;
    const res = compute_health_safe(isOnline, scoreInt);
    return { score: res.score, label: res.label, color: res.color, description: res.description };
  }, [controllerHealth, isOnline]);

  if (isLoading) {
    return <LoadingState message="Đang tải dữ liệu trạm thông minh..." />;
  }

  if (!sensorData) {
    return <LoadingState message="Không có tín hiệu cảm biến!" />;
  }

  if (!deviceId) {
    return (
      <div className="flex flex-col items-center justify-center h-full min-h-[80vh] space-y-5 p-6 text-center">
        <div className="p-6 bg-white rounded-3xl border border-line shadow-xl shadow-primary/10">
          <Settings size={40} className="text-primary" />
        </div>
        <div className="space-y-2 max-w-xs">
          <h2 className="text-xl font-bold text-primary-deep">Chưa chọn thiết bị</h2>
          <p className="text-sm text-text-muted leading-relaxed">
            Hệ thống cần Device ID. Vui lòng chuyển tới cài đặt.
          </p>
        </div>
      </div>
    );
  }

  const faultCode = extract_fault_code_str(fsmState || '');
  const faultGuideOpt = faultCode ? get_fault_guide(faultCode) : null;
  const faultGuide = faultGuideOpt && (faultGuideOpt as any)[0] ? (faultGuideOpt as any)[0] : null;

  const pumps: any = sensorData?.pump_status || {};
  const modeLabel = settings?.control_mode === 'auto' ? 'Tự động' : 'Thủ công';

  const ecStatus = sensorStatus(sensorData?.err_ec, sensorData?.ec, getTdsSetting(settings, 'min_ec_limit', 'min_ec_limit'), getTdsSetting(settings, 'max_ec_limit', 'max_ec_limit'));
  const phStatus = sensorStatus(sensorData?.err_ph, sensorData?.ph, settings?.min_ph_limit, settings?.max_ph_limit);
  const tempStatus = sensorStatus(sensorData?.err_temp, sensorData?.temp, settings?.min_temp_limit, settings?.max_temp_limit);
  const waterStatus = sensorStatus(sensorData?.err_water, sensorData?.water_level, settings?.water_level_min, settings?.water_level_max);

  const nextAction = !isOnline
    ? 'Kiểm tra nguồn Wi-Fi trạm điều khiển.'
    : !isSensorOnline
      ? 'Đang mất tín hiệu cảm biến. Kiểm tra nguồn node cảm biến.'
      : faultGuide?.action || (permission !== 'granted' ? 'Bật thông báo để nhận cảnh báo tức thì.' : 'Không cần thao tác. Tiếp tục theo dõi.');

  const hasTankAlert = Boolean(
    tankAlert && (tankAlert.tank_a_low || tankAlert.tank_b_low || tankAlert.tank_ph_down_low || tankAlert.tank_ph_up_low)
  );

  const hasActionableIssue = Boolean(faultCode) || !isOnline || !isSensorOnline;
  const isCritical = !isOnline || !isSensorOnline;

  return (
    <div className="app-page">
      {/* Header Bento Box */}
      <div className="ui-card relative overflow-hidden p-6 md:p-8">
        <div className="flex flex-col lg:flex-row lg:items-center justify-between gap-6">
          <div className="space-y-4 max-w-2xl">
            <div className="flex flex-wrap items-center gap-2">
              <DeviceStatePill
                state={isOnline ? 'online' : 'offline'}
                label={isOnline ? 'Trạm Online' : 'Trạm Offline'}
              />
              <span className="farm-status-pill bg-soft text-text-muted border-line">
                <Cpu size={13} />
                {modeLabel}
              </span>
              <Link
                to="/fleet"
                title="Quản lý các thiết bị đã liên kết"
                className="farm-status-pill bg-white text-text-muted border-line hover:bg-soft transition-colors"
              >
                ID: {deviceId}
              </Link>
            </div>
            <div>
              <h1 className="text-2xl md:text-3xl font-bold tracking-tight text-primary-deep">
                {greetingName ? `Xin chào, ${greetingName} 👋` : friendlyState.label}
              </h1>
              <p className="text-sm md:text-base text-text-muted leading-relaxed mt-2">
                {greetingName ? friendlyState.description : friendlyState.description}
              </p>
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
                      <Button size="sm" onClick={() => navigate('/operations')}>
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
              <span className="text-[10px] text-faint font-bold uppercase tracking-wider">Sức khỏe trạm</span>
              <div className="mt-2 flex justify-center">
                <HealthScore score={computedHealth.score} label={computedHealth.label} />
              </div>
            </div>
            <div className="rounded-2xl border border-line bg-surface-muted p-4 text-center">
              <span className="text-[10px] text-faint font-bold uppercase tracking-wider">Cảm biến</span>
              <div className={`text-2xl font-black mt-3 ${isSensorOnline ? 'text-status' : 'text-error'}`}>
                {isSensorOnline ? 'Tốt' : 'Mất'}
              </div>
              <p className="text-xs font-semibold text-primary-deep mt-2">{isSensorOnline ? 'Đang đo' : 'Cần kiểm tra'}</p>
            </div>
          </div>
        </div>
      </div>

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
        onDose={() => navigate('/operations')}
        onPausePumps={() => navigate('/operations')}
        onViewAlerts={() => navigate('/journal')}
      />

      {/* Sensor Bento Grid */}
      <div className="space-y-3">
        <div className="flex items-center justify-between">
          <h3 className="farm-section-title">
            <LineChart size={14} />
            <span>Thông số thời gian thực</span>
          </h3>
        </div>
        <div className={`grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-4 gap-4 transition-all duration-500 ${!isSensorOnline ? 'opacity-60 grayscale' : ''}`}>
          <SensorBentoCard
            title="Dinh dưỡng EC"
            value={sensorData?.err_ec === true ? "Bảo trì" : formatNumber(sensorData?.ec, 2)}
            unit={sensorData?.err_ec === true ? "" : "ppm"}
            icon={Activity}
            theme={sensorData?.err_ec === true ? "rose" : "blue"}
            statusLabel={ecStatus.label}
            statusTone={ecStatus.tone}
            rangeLabel={`Mục tiêu ${formatNumber(getTdsSetting(settings, 'ec_target', 'ec_target'), 2)} ± ${formatNumber(getTdsSetting(settings, 'ec_tolerance', 'ec_tolerance'), 2)}`}
            description={sensorData?.err_ec === true ? 'Lỗi cảm biến EC.' : 'Nồng độ dinh dưỡng bồn chứa.'}
            sparkline={sensorData?.err_ec === true ? 0 : Math.max(0, Math.min(100, (Number(sensorData?.ec ?? 0) / (Number(getTdsSetting(settings, 'ec_max_limit', 'ec_max_limit')) || 1)) * 100))}
          />
          <SensorBentoCard
            title="Độ pH"
            value={sensorData?.err_ph === true ? "Lỗi" : formatNumber(sensorData?.ph, 2)}
            unit=""
            icon={Droplets}
            theme={sensorData?.err_ph === true ? "rose" : "fuchsia"}
            statusLabel={phStatus.label}
            statusTone={phStatus.tone}
            rangeLabel={`Mục tiêu ${formatNumber((settings as any)?.ph_target, 2)} ± ${formatNumber((settings as any)?.ph_tolerance, 2)}`}
            description={sensorData?.err_ph === true ? 'Cần hiệu chuẩn pH.' : 'Độ cân bằng axit/kiềm.'}
            sparkline={sensorData?.err_ph === true ? 0 : Math.max(0, Math.min(100, (Number(sensorData?.ph ?? 14) / 14) * 100))}
          />
          <SensorBentoCard
            title="Nhiệt độ"
            value={sensorData?.err_temp === true ? "Lỗi" : formatNumber(sensorData?.temp, 1)}
            unit={sensorData?.err_temp === true ? "" : "°C"}
            icon={Thermometer}
            theme={sensorData?.err_temp === true ? "rose" : "orange"}
            statusLabel={tempStatus.label}
            statusTone={tempStatus.tone}
            rangeLabel={`An toàn ${formatNumber((settings as any)?.min_temp_limit, 0)}-${formatNumber((settings as any)?.max_temp_limit, 0)}°C`}
            description="Nhiệt độ dung dịch bồn chứa."
            sparkline={sensorData?.err_temp === true ? 0 : Math.max(0, Math.min(100, (Number(sensorData?.temp ?? 0) / 50) * 100))}
          />
          <SensorBentoCard
            title="Mực nước"
            value={sensorData?.err_water === true ? "Lỗi phao" : formatNumber(sensorData?.water_level, 0)}
            unit={sensorData?.err_water === true ? "" : "%"}
            icon={Waves}
            theme={sensorData?.err_water === true ? "rose" : "cyan"}
            statusLabel={waterStatus.label}
            statusTone={waterStatus.tone}
            rangeLabel={`Giữ quanh ${formatNumber((settings as any)?.water_level_target, 0)}%`}
            description={sensorData?.err_water === true ? 'Kiểm tra phao siêu âm.' : 'Đảm bảo bơm không chạy khô.'}
            sparkline={sensorData?.err_water === true ? 0 : Math.max(0, Math.min(100, Number(sensorData?.water_level ?? 0)))}
          />
        </div>
      </div>

      {/* Active Device Pumps */}
      <div className="ui-card space-y-3">
        <h3 className="farm-section-title"><Zap size={14} /> Thiết bị đang chạy</h3>
        <div className="flex flex-wrap gap-2">
          {Object.values(pumps).some(v => v === true) ? (
            Object.entries(pumps).map(([key, isRunning]) => {
              if (!isRunning) return null;
              return <ActiveDeviceTag key={key} label={pumpLabels[key] || key} color={pumpColors[key] || 'bg-pill text-status border-pill'} />;
            })
          ) : (
            <span className="farm-status-pill bg-pill text-status border-pill">
              Không có bơm hoặc van nào đang chạy
            </span>
          )}
        </div>
      </div>

      <DosingSummaryCard
        totalCount={dosingTotalCount}
        lastDosedAt={healthSummary?.latest_ph_dosing_at ?? null}
      />

      <EmergencyStopButton deviceId={deviceId} variant="floating" />
    </div>
  );
};

export default Dashboard;
