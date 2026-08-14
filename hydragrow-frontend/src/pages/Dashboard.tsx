import { useEffect, useMemo, useState } from 'react';
import {
  Droplets, Thermometer, Activity, Waves, Settings, Zap, Cpu,
  Wifi, AlertTriangle, LineChart
} from 'lucide-react';

// --- ZUSTAND & GLEAM ---
import { useDeviceStore } from '../store/useDeviceStore';
import {
  eval_sensor_status_safe,
  calc_budget_percent_safe,
  calc_hourly_dose_str
} from '../../gleam_core/build/dev/javascript/gleam_core/dashboard.mjs';
import { extract_fault_code_str, friendly_state, compute_health_safe } from '../../gleam_core/build/dev/javascript/gleam_core/fsm.mjs';
import { get_fault_guide } from '../../gleam_core/build/dev/javascript/gleam_core/faults.mjs';

// --- UI COMPONENTS & UTILS ---
import { SensorBentoCard } from '../components/ui/SensorBentoCard';
import { LoadingState } from '../components/ui/LoadingState';
import { httpFetch } from '../platform/http';
import { loadAppSettings } from '../platform/settings';
import { useFCM } from '../hooks/useFCM';
import { useQuery } from '@tanstack/react-query';

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

const sensorStatus = (hasError: boolean | undefined, value: any, min?: any, max?: any) => {
  const res = eval_sensor_status_safe(
    Boolean(hasError),
    String(value ?? ''),
    String(min ?? ''),
    String(max ?? '')
  );
  return { label: res.label, tone: res.tone as 'good' | 'warn' | 'danger' | 'info' };
};

const getDosingDose = (event: any) => {
  const meta = event?.metadata || event?.payload || {};
  const dose = meta.dose ?? meta.dosing_report?.dose ?? meta.dosing_data?.dose ?? meta.dosing_report ?? meta.dosing_data ?? meta;
  return {
    ec_ml: Number(dose.pump_a_ml || 0) + Number(dose.pump_b_ml || 0),
    ph_ml: Number(dose.ph_up_ml || 0) + Number(dose.ph_down_ml || 0),
  };
};

const deriveHourlyBudgets = (events: any[]) => {
  const now = Date.now();
  const doseItemsString = events
    .map((event) => {
      const ts = Number(event?.timestamp || event?.timestamp_ms || 0);
      const eventMs = ts > 1e12 ? ts : ts * 1000;
      const dose = getDosingDose(event);
      return `${eventMs},${dose.ec_ml},${dose.ph_ml}`;
    })
    .join(';');

  const res = calc_hourly_dose_str(doseItemsString, now);
  return { ec_ml: res.ec_ml, ph_ml: res.ph_ml };
};

const mergeUniqueEvents = (...groups: any[][]) => {
  const seen = new Set<string>();
  const merged: any[] = [];
  groups.flat().forEach((event) => {
    if (!event) return;
    const key = String(event.id ?? `${event.timestamp || event.timestamp_ms}-${event.category || ''}-${event.title || ''}-${event.message || ''}`);
    if (seen.has(key)) return;
    seen.add(key);
    merged.push(event);
  });
  return merged;
};

const Dashboard = () => {
const deviceId = useDeviceStore((s) => s.deviceId);
  const sensorData = useDeviceStore((s) => s.sensorData);
  const isOnline = useDeviceStore((s) => s.deviceStatus.is_online);
  const rawBudgets = useDeviceStore((s) => (s.deviceStatus as any)?.budgets);
  const controllerHealth = useDeviceStore((s) => s.controllerHealth);
  const fsmState = useDeviceStore((s) => s.fsmState);
  const isLoading = useDeviceStore((s) => s.isLoading);
  const isSensorOnline = useDeviceStore((s) => s.isSensorOnline);
  const settings = useDeviceStore((s) => s.settings);
  const systemEvents = useDeviceStore((s) => s.systemEvents);

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

  const { permission } = useFCM();

  // 🔴 THAY THẾ useEffect CŨ BẰNG useQuery (Tự động catch AbortError & Retry)
  const { data: recentEvents = [] } = useQuery({
    queryKey: ['recent-events', deviceId],
    queryFn: async ({ signal }) => {
      if (!deviceId || !settings?.backend_url) return [];
      const res = await httpFetch(`${settings.backend_url}/api/devices/${deviceId}/events?limit=200`, {
        headers: { 'X-API-Key': settings.api_key || '' },
        signal // Tự động chuyển AbortSignal của TanStack Query vào fetch
      });
      if (!res.ok) return [];
      const json = await res.json();
      return Array.isArray(json.data) ? json.data : [];
    },
    enabled: Boolean(deviceId && settings?.backend_url),
  });

  const eventBudgets = useMemo(
    () => deriveHourlyBudgets(mergeUniqueEvents(recentEvents, systemEvents || [])),
    [recentEvents, systemEvents]
  );

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

  const faultCode = extract_fault_code_str(fsmState || '');
  const faultGuideOpt = faultCode ? get_fault_guide(faultCode) : null;
  const faultGuide = faultGuideOpt && (faultGuideOpt as any)[0] ? (faultGuideOpt as any)[0] : null;

  const pumps: any = sensorData?.pump_status || {};
  const budgets = {
    ec_ml: Number(rawBudgets?.ec_ml || 0) > 0 ? rawBudgets.ec_ml : eventBudgets.ec_ml,
    ph_ml: Number(rawBudgets?.ph_ml || 0) > 0 ? rawBudgets.ph_ml : eventBudgets.ph_ml,
  };

  const modeLabel = settings?.control_mode === 'auto' ? 'Tự động' : 'Thủ công';

  const ecStatus = sensorStatus(sensorData?.err_ec, sensorData?.ec, (settings as any)?.min_ec_limit, (settings as any)?.max_ec_limit);
  const phStatus = sensorStatus(sensorData?.err_ph, sensorData?.ph, (settings as any)?.min_ph_limit, (settings as any)?.max_ph_limit);
  const tempStatus = sensorStatus(sensorData?.err_temp, sensorData?.temp, (settings as any)?.min_temp_limit, (settings as any)?.max_temp_limit);
  const waterStatus = sensorStatus(sensorData?.err_water, sensorData?.water_level, (settings as any)?.water_level_min, (settings as any)?.water_level_max);

  const nextAction = !isOnline
    ? 'Kiểm tra nguồn Wi-Fi trạm điều khiển.'
    : !isSensorOnline
      ? 'Đang mất tín hiệu cảm biến. Kiểm tra nguồn node cảm biến.'
      : faultGuide?.action || (permission !== 'granted' ? 'Bật thông báo để nhận cảnh báo tức thì.' : 'Không cần thao tác. Tiếp tục theo dõi.');

  return (
    <div className="app-page">
      {/* Header Bento Box */}
      <div className="ui-card relative overflow-hidden p-6 md:p-8">
        <div className="flex flex-col lg:flex-row lg:items-center justify-between gap-6">
          <div className="space-y-4 max-w-2xl">
            <div className="flex flex-wrap items-center gap-2">
              <span className={`farm-status-pill ${isOnline ? 'bg-emerald-50 text-emerald-700 border-emerald-200' : 'bg-red-50 text-red-700 border-red-200'}`}>
                <Wifi size={13} />
                {isOnline ? 'Trạm Online' : 'Trạm Offline'}
              </span>
              <span className="farm-status-pill bg-sky-50 text-sky-700 border-sky-200">
                <Cpu size={13} />
                {modeLabel}
              </span>
              <span className="farm-status-pill bg-white text-emerald-800 border-emerald-200">
                ID: {deviceId}
              </span>
            </div>
            <div>
              <h1 className="text-2xl md:text-3xl font-bold tracking-tight text-emerald-950">
                {friendlyState.label}
              </h1>
              <p className="text-sm md:text-base text-emerald-800/80 leading-relaxed mt-2">
                {friendlyState.description}
              </p>
            </div>
            <div className={`rounded-2xl border p-4 flex gap-3 items-start ${faultCode || !isOnline || !isSensorOnline ? 'bg-amber-50 border-amber-200' : 'bg-emerald-50 border-emerald-200'}`}>
              <AlertTriangle className={`${faultCode || !isOnline || !isSensorOnline ? 'text-amber-700' : 'text-emerald-700'} shrink-0 mt-0.5`} size={18} />
              <div>
                <h2 className="text-sm font-bold text-emerald-950">Hành động tiếp theo</h2>
                <p className="text-xs md:text-sm text-emerald-800/80 leading-relaxed mt-1">{nextAction}</p>
              </div>
            </div>
          </div>

          <div className="grid grid-cols-2 gap-3 w-full lg:w-72">
            <div className="rounded-2xl border border-emerald-100 bg-emerald-50 p-4 text-center">
              <span className="text-[10px] text-emerald-800/70 font-bold uppercase tracking-wider">Sức khỏe trạm</span>
              <div className={`text-4xl font-black font-mono tracking-tight mt-1 ${computedHealth.score >= 90 ? 'text-emerald-700' : computedHealth.score >= 60 ? 'text-amber-700' : 'text-red-700'}`}>
                {computedHealth.score}%
              </div>
              <p className="text-xs font-semibold text-emerald-900 mt-1">{computedHealth.label}</p>
            </div>
            <div className="rounded-2xl border border-sky-100 bg-sky-50 p-4 text-center">
              <span className="text-[10px] text-sky-800/70 font-bold uppercase tracking-wider">Cảm biến</span>
              <div className={`text-2xl font-black mt-3 ${isSensorOnline ? 'text-emerald-700' : 'text-red-700'}`}>
                {isSensorOnline ? 'Tốt' : 'Mất'}
              </div>
              <p className="text-xs font-semibold text-sky-900 mt-2">{isSensorOnline ? 'Đang đo' : 'Cần kiểm tra'}</p>
            </div>
          </div>
        </div>
      </div>

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
            unit={sensorData?.err_ec === true ? "" : "mS/cm"}
            icon={Activity}
            theme={sensorData?.err_ec === true ? "rose" : "blue"}
            statusLabel={ecStatus.label}
            statusTone={ecStatus.tone}
            rangeLabel={`Mục tiêu ${formatNumber((settings as any)?.ec_target, 2)} ± ${formatNumber((settings as any)?.ec_tolerance, 2)}`}
            description={sensorData?.err_ec === true ? 'Lỗi cảm biến EC.' : 'Nồng độ dinh dưỡng bồn chứa.'}
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
              return <ActiveDeviceTag key={key} label={pumpLabels[key] || key} color={pumpColors[key] || 'bg-emerald-50 text-emerald-700 border-emerald-200'} />;
            })
          ) : (
            <span className="farm-status-pill bg-emerald-50 text-emerald-700 border-emerald-200">
              Không có bơm hoặc van nào đang chạy
            </span>
          )}
        </div>
      </div>
    </div>
  );
};

export default Dashboard;
