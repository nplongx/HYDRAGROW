import { useMemo } from 'react';
import {
  Droplets, Thermometer, Activity, Waves, Settings, Zap, FlaskConical, AlertTriangle
} from 'lucide-react';
import { useDeviceStore } from '../store/useDeviceStore';
import { eval_sensor_status_safe } from '../../gleam_core/build/dev/javascript/gleam_core/dashboard.mjs';
import { friendly_state, compute_health_safe } from '../../gleam_core/build/dev/javascript/gleam_core/fsm.mjs';
import { SensorBentoCard } from '../components/ui/SensorBentoCard';
import { FsmStatusBadge } from '../components/ui/FsmStatusBadge';
import { LoadingState } from '../components/ui/LoadingState';

const HealthRing = ({ score }: { score: number }) => {
  const radius = 45;
  const circumference = 2 * Math.PI * radius;
  const offset = circumference - (score / 100) * circumference;
  const color = score >= 80 ? '#16a34a' : score >= 50 ? '#d97706' : '#dc2626';
  return (
    <svg width="110" height="110" viewBox="0 0 110 110">
      <circle cx="55" cy="55" r={radius} fill="none" stroke="#d1fae5" strokeWidth="10" />
      <circle
        cx="55" cy="55" r={radius} fill="none"
        stroke={color} strokeWidth="10" strokeLinecap="round"
        strokeDasharray={circumference}
        strokeDashoffset={offset}
        transform="rotate(-90 55 55)"
        className="health-ring-arc"
        style={{ '--ring-offset': offset } as any}
      />
      <text x="55" y="51" textAnchor="middle" className="fill-emerald-950 font-black" style={{ fontSize: '18px', fontWeight: 800 }}>
        {score}
      </text>
      <text x="55" y="65" textAnchor="middle" className="fill-emerald-700/60" style={{ fontSize: '10px', fontWeight: 600 }}>
        Sức khoẻ
      </text>
    </svg>
  );
};

const ActiveDeviceTag = ({ label, color }: { label: string; color: string }) => (
  <span className={`flex items-center gap-1.5 px-3 py-1 rounded-full text-xs font-semibold tracking-wide border ${color}`}>
    <Zap size={12} className="fill-current" />
    {label}
  </span>
);

const pumpLabels: Record<string, string> = {
  pump_a: 'Dinh dưỡng A', pump_b: 'Dinh dưỡng B',
  ph_up: 'pH Up', ph_down: 'pH Down',
  osaka_pump: 'Trộn tuần hoàn', mist_valve: 'Phun sương',
  water_pump_in: 'Cấp nước', water_pump_out: 'Xả nước'
};

const pumpColors: Record<string, string> = {
  pump_a: 'bg-orange-50 text-orange-700 border-orange-200',
  pump_b: 'bg-orange-50 text-orange-700 border-orange-200',
  ph_up: 'bg-fuchsia-50 text-fuchsia-700 border-fuchsia-200',
  ph_down: 'bg-rose-50 text-rose-700 border-rose-200',
  osaka_pump: 'bg-indigo-50 text-indigo-700 border-indigo-200',
  mist_valve: 'bg-sky-50 text-sky-700 border-sky-200',
  water_pump_in: 'bg-blue-50 text-blue-700 border-blue-200',
  water_pump_out: 'bg-cyan-50 text-cyan-700 border-cyan-200'
};

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
  const settings = useDeviceStore((s) => s.settings);

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
        <div className="p-6 bg-white rounded-3xl border border-emerald-100 shadow-xl shadow-emerald-950/10">
          <Settings size={40} className="text-emerald-700" />
        </div>
        <div className="space-y-2 max-w-xs">
          <h2 className="text-xl font-bold text-emerald-950">Chưa chọn thiết bị</h2>
          <p className="text-sm text-emerald-800/75 leading-relaxed">
            Hệ thống cần Device ID. Vui lòng chuyển tới cài đặt.
          </p>
        </div>
      </div>
    );
  }

  const pumps: any = sensorData?.pump_status || {};
  const modeLabel = settings?.control_mode === 'auto' ? 'Tự động' : 'Thủ công';

  const ecStatus = sensorStatus(sensorData?.err_ec, sensorData?.ec, getTdsSetting(settings, 'min_ec_limit', 'min_ec_limit'), getTdsSetting(settings, 'max_ec_limit', 'max_ec_limit'));
  const phStatus = sensorStatus(sensorData?.err_ph, sensorData?.ph, settings?.min_ph_limit, settings?.max_ph_limit);
  const tempStatus = sensorStatus(sensorData?.err_temp, sensorData?.temp, settings?.min_temp_limit, settings?.max_temp_limit);
  const waterStatus = sensorStatus(sensorData?.err_water, sensorData?.water_level, settings?.water_level_min, settings?.water_level_max);

  const tankAlert = useDeviceStore((s) => s.tankAlert);
  const hasTankAlert = Boolean(
    tankAlert && (tankAlert.tank_a_low || tankAlert.tank_b_low || tankAlert.tank_ph_down_low || tankAlert.tank_ph_up_low)
  );

  return (
    <div className="app-page space-y-4">
      {/* Status Hero */}
      <div className="ui-card flex flex-col md:flex-row items-center md:items-start gap-5 p-5">
        <div className="shrink-0">
          <HealthRing score={computedHealth.score >= 0 ? computedHealth.score : 0} />
        </div>
        <div className="flex-1 min-w-0 text-center md:text-left space-y-2">
          <div className="flex flex-wrap justify-center md:justify-start items-center gap-2">
            <FsmStatusBadge state={friendlyState.type} label={friendlyState.label} />
            <span className={`farm-status-pill ${isOnline ? 'bg-emerald-50 text-emerald-700 border-emerald-200' : 'bg-red-50 text-red-700 border-red-200'}`}>
              <Activity size={10} />{isOnline ? 'Trực tuyến' : 'Ngoại tuyến'}
            </span>
            <span className="farm-status-pill bg-emerald-50 text-emerald-700 border-emerald-200">
              {modeLabel}
            </span>
          </div>
          <p className="text-sm font-medium text-emerald-800/70">{friendlyState.description}</p>
          <p className="text-xs text-emerald-700/50 font-mono">{deviceId}</p>
        </div>
      </div>

      {/* Cảnh báo cạn bình dung dịch */}
      {hasTankAlert && (
        <div className="bg-amber-50 border border-amber-300 rounded-2xl p-4 flex items-start gap-3 text-amber-900 shadow-sm animate-in fade-in">
          <AlertTriangle className="text-amber-600 shrink-0 mt-0.5" size={20} />
          <div className="space-y-1">
            <h4 className="font-bold text-sm">Cảnh báo: Bình dung dịch sắp cạn</h4>
            <div className="flex flex-wrap gap-2 pt-1">
              {tankAlert?.tank_a_low && <span className="px-2.5 py-0.5 rounded-full text-xs font-bold bg-amber-200/70 border border-amber-300 text-amber-950">Cạn Dinh Dưỡng A</span>}
              {tankAlert?.tank_b_low && <span className="px-2.5 py-0.5 rounded-full text-xs font-bold bg-amber-200/70 border border-amber-300 text-amber-950">Cạn Dinh Dưỡng B</span>}
              {tankAlert?.tank_ph_up_low && <span className="px-2.5 py-0.5 rounded-full text-xs font-bold bg-purple-100 border border-purple-300 text-purple-900">Cạn pH Up</span>}
              {tankAlert?.tank_ph_down_low && <span className="px-2.5 py-0.5 rounded-full text-xs font-bold bg-rose-100 border border-rose-300 text-rose-900">Cạn pH Down</span>}
            </div>
          </div>
        </div>
      )}

      {/* Sensor Cards Grid */}
      <div className="grid grid-cols-2 gap-3">
        <SensorBentoCard
          title="EC dinh dưỡng"
          value={formatNumber(sensorData.ec, 1)}
          unit="mS/cm"
          icon={Droplets}
          theme="orange"
          statusLabel={ecStatus.label}
          statusTone={ecStatus.tone}
          rangeLabel={settings?.min_ec_limit != null ? `Mục tiêu: ${settings.min_ec_limit}–${getTdsSetting(settings, 'max_ec_limit', 'max_ec_limit')} mS/cm` : undefined}
          description={sensorData.err_ec ? 'Cảm biến EC lỗi. Cần hiệu chỉnh.' : undefined}
        />
        <SensorBentoCard
          title="Độ pH"
          value={formatNumber(sensorData.ph, 2)}
          unit="pH"
          icon={FlaskConical}
          theme="fuchsia"
          statusLabel={phStatus.label}
          statusTone={phStatus.tone}
          rangeLabel={settings?.min_ph_limit != null ? `Mục tiêu: ${settings.min_ph_limit}–${settings.max_ph_limit}` : undefined}
          description={sensorData.err_ph ? 'Cảm biến pH lỗi. Cần hiệu chỉnh.' : undefined}
        />
        <SensorBentoCard
          title="Nhiệt độ nước"
          value={formatNumber(sensorData.temp, 1)}
          unit="°C"
          icon={Thermometer}
          theme="rose"
          statusLabel={tempStatus.label}
          statusTone={tempStatus.tone}
          rangeLabel={settings?.min_temp_limit != null ? `Mục tiêu: ${settings.min_temp_limit}–${settings.max_temp_limit}°C` : undefined}
          description={sensorData.err_temp ? 'Cảm biến nhiệt độ lỗi.' : undefined}
        />
        <SensorBentoCard
          title="Mực nước bể"
          value={formatNumber(sensorData.water_level, 0)}
          unit="%"
          icon={Waves}
          theme="cyan"
          statusLabel={waterStatus.label}
          statusTone={waterStatus.tone}
          rangeLabel={settings?.water_level_min != null ? `Giới hạn: ${settings.water_level_min}–${settings.water_level_max}%` : undefined}
          description={sensorData.err_water ? 'Cảm biến mực nước lỗi.' : undefined}
        />
      </div>

      {/* Active Equipment */}
      {Object.keys(pumps).length > 0 && (
        <div className="ui-card space-y-3">
          <p className="farm-section-title">Thiết bị đang hoạt động</p>
          <div className="flex flex-wrap gap-2">
            {Object.entries(pumps).filter(([, v]: any) => Boolean(v)).map(([key]) => (
              <ActiveDeviceTag key={key} label={pumpLabels[key] || key} color={pumpColors[key] || 'bg-emerald-50 text-emerald-700 border-emerald-200'} />
            ))}
            {Object.values(pumps).every(v => !v) && (
              <p className="text-xs text-emerald-700/50">Không có thiết bị đang chạy</p>
            )}
          </div>
        </div>
      )}
    </div>
  );
};

export default Dashboard;
