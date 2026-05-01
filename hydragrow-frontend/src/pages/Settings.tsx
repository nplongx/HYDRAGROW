import React, { useState, useEffect, useMemo } from 'react';
import {
  Save, Target, ShieldAlert, Waves,
  FlaskConical, Activity, Settings2, Power, Network, Zap, LockKeyhole,
  CalendarClock
} from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { fetch } from '@tauri-apps/plugin-http';
import toast from 'react-hot-toast';

import { Switch } from '../components/ui/Switch';
import { InputGroup } from '../components/ui/InputGroup';
import { SubCard } from '../components/ui/SubCard';
import { AccordionSection } from '../components/ui/AccordionSection';
import { useDeviceContext } from '../context/DeviceContext';
import { LoadingState } from '../components/ui/LoadingState';

// ... (Giữ nguyên toàn bộ Type và Logic validate của bạn ở đây)
type InputEvent = React.ChangeEvent<HTMLInputElement | HTMLSelectElement>;
type DosingFieldKey =
  | 'dosing_pwm_percent' | 'dosing_min_pwm_percent' | 'pump_a_capacity_ml_per_sec'
  | 'pump_b_capacity_ml_per_sec' | 'pump_ph_up_capacity_ml_per_sec' | 'pump_ph_down_capacity_ml_per_sec'
  | 'scheduled_dose_a_ml' | 'scheduled_dose_b_ml';

type DosingValidationErrors = Partial<Record<DosingFieldKey, string>>;

const toFiniteNumber = (value: any): number => {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : NaN;
};

const backendLikeError = (field: string, detail: string) => `Giá trị không hợp lệ cho ${field}: ${detail}.`;

const validateDosingConfig = (inputConfig: any): DosingValidationErrors => {
  const errors: DosingValidationErrors = {};
  const scheduledDosingEnabled = Boolean(inputConfig.scheduled_dosing_enabled);

  const dosingPwm = toFiniteNumber(inputConfig.dosing_pwm_percent);
  const dosingMinPwm = toFiniteNumber(inputConfig.dosing_min_pwm_percent);
  const doseA = toFiniteNumber(inputConfig.scheduled_dose_a_ml);
  const doseB = toFiniteNumber(inputConfig.scheduled_dose_b_ml);
  const pumpA = toFiniteNumber(inputConfig.pump_a_capacity_ml_per_sec);
  const pumpB = toFiniteNumber(inputConfig.pump_b_capacity_ml_per_sec);
  const pumpPhUp = toFiniteNumber(inputConfig.pump_ph_up_capacity_ml_per_sec);
  const pumpPhDown = toFiniteNumber(inputConfig.pump_ph_down_capacity_ml_per_sec);

  if (!Number.isFinite(dosingPwm) || dosingPwm < 1 || dosingPwm > 100) {
    errors.dosing_pwm_percent = backendLikeError('dosing_pwm_percent', 'phải nằm trong khoảng 1-100');
  }
  if (!Number.isFinite(dosingMinPwm) || dosingMinPwm < 0 || dosingMinPwm > 100) {
    errors.dosing_min_pwm_percent = backendLikeError('dosing_min_pwm_percent', 'phải nằm trong khoảng 0-100');
  }

  const validateCapacity = (field: DosingFieldKey, value: number) => {
    if (!Number.isFinite(value) || value <= 0) {
      errors[field] = backendLikeError(field, 'phải lớn hơn 0');
    }
  };

  validateCapacity('pump_a_capacity_ml_per_sec', pumpA);
  validateCapacity('pump_b_capacity_ml_per_sec', pumpB);
  validateCapacity('pump_ph_up_capacity_ml_per_sec', pumpPhUp);
  validateCapacity('pump_ph_down_capacity_ml_per_sec', pumpPhDown);

  if (scheduledDosingEnabled && (!Number.isFinite(doseA) || doseA < 0)) {
    errors.scheduled_dose_a_ml = backendLikeError('scheduled_dose_a_ml', 'phải lớn hơn hoặc bằng 0');
  }
  if (scheduledDosingEnabled && (!Number.isFinite(doseB) || doseB < 0)) {
    errors.scheduled_dose_b_ml = backendLikeError('scheduled_dose_b_ml', 'phải lớn hơn hoặc bằng 0');
  }

  return errors;
};

// --- COMPONENT TRỰC QUAN HOÁ CRON (Tối giản lại) ---
const VisualCronPicker = ({ value, onChange, label, desc }: {
  value: string; onChange: (val: string) => void; label: string; desc?: string;
}) => {
  const parts = (value || "0 0 8 * * *").trim().split(/\s+/);
  const minute = parts[1] !== '*' && parts[1] !== undefined ? parts[1].padStart(2, '0') : '00';
  const hour = parts[2] !== '*' && parts[2] !== undefined ? parts[2].padStart(2, '0') : '08';
  const timeStr = `${hour}:${minute}`;

  const dow = parts[5] || '*';
  const isEveryDay = dow === '*';
  const selectedDays = isEveryDay ? [] : dow.split(',');

  const handleTimeChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const val = e.target.value;
    if (!val) return;
    const [h, m] = val.split(':');
    onChange(`${parts[0] || '0'} ${parseInt(m)} ${parseInt(h)} ${parts[3] || '*'} ${parts[4] || '*'} ${dow}`);
  };

  const toggleDay = (dayVal: string) => {
    let newDays = [...selectedDays];
    if (newDays.includes(dayVal)) newDays = newDays.filter(d => d !== dayVal);
    else newDays.push(dayVal);
    const newDow = newDays.length === 0 ? '*' : newDays.join(',');
    onChange(`${parts[0] || '0'} ${parseInt(minute)} ${parseInt(hour)} ${parts[3] || '*'} ${parts[4] || '*'} ${newDow}`);
  };

  const setEveryDay = () => onChange(`${parts[0] || '0'} ${parseInt(minute)} ${parseInt(hour)} ${parts[3] || '*'} ${parts[4] || '*'} *`);

  const daysOfWeek = [
    { val: 'MON', label: 'T2' }, { val: 'TUE', label: 'T3' }, { val: 'WED', label: 'T4' },
    { val: 'THU', label: 'T5' }, { val: 'FRI', label: 'T6' }, { val: 'SAT', label: 'T7' }, { val: 'SUN', label: 'CN' },
  ];

  return (
    <div className="space-y-4 bg-slate-900/50 border border-slate-800 p-5 rounded-xl">
      <div>
        <label className="text-sm font-medium text-slate-200 flex items-center gap-2">
          <CalendarClock size={16} className="text-slate-400" /> {label}
        </label>
        {desc && <p className="text-xs text-slate-500 mt-1">{desc}</p>}
      </div>

      <div className="flex flex-col sm:flex-row sm:items-center gap-6">
        <div className="bg-slate-950 px-4 py-2 rounded-lg border border-slate-800">
          <input
            type="time"
            value={timeStr}
            onChange={handleTimeChange}
            className="bg-transparent text-slate-100 text-xl font-medium outline-none text-center cursor-pointer [color-scheme:dark]"
          />
        </div>

        <div className="flex-1 space-y-3">
          <div className="flex items-center gap-3">
            <button
              onClick={setEveryDay}
              className={`px-3 py-1.5 rounded-md text-xs font-medium transition-colors ${isEveryDay ? 'bg-blue-600 text-white' : 'bg-slate-800 text-slate-400 hover:bg-slate-700'}`}
            >
              Hàng ngày
            </button>
            <span className="text-xs text-slate-500">hoặc chọn ngày:</span>
          </div>

          <div className="flex flex-wrap gap-2">
            {daysOfWeek.map(day => {
              const isSelected = !isEveryDay && selectedDays.includes(day.val);
              return (
                <button
                  key={day.val}
                  onClick={() => toggleDay(day.val)}
                  className={`w-9 h-9 rounded-full text-xs font-medium transition-colors flex items-center justify-center border ${isSelected ? 'bg-blue-500/20 border-blue-500 text-blue-400' : 'bg-slate-800/50 border-slate-700 text-slate-400 hover:border-slate-600 hover:text-slate-200'
                    }`}
                >
                  {day.label}
                </button>
              );
            })}
          </div>
        </div>
      </div>

      <div className="pt-3 border-t border-slate-800">
        <span className="text-xs text-slate-500 font-mono">Cron: {value || "0 0 8 * * *"}</span>
      </div>
    </div>
  );
};

// --- COMPONENT SETTINGS CHÍNH ---
const Settings = () => {
  const { sensorData, isSensorOnline, settings: runtimeSettings, deviceId: ctxDeviceId, systemEvents } = useDeviceContext();
  const [isLoading, setIsLoading] = useState(true);
  const [isSaving, setIsSaving] = useState(false);
  const [openSection, setOpenSection] = useState<string | null>('general');
  const [isAdvancedMode, setIsAdvancedMode] = useState(false);

  const handleToggleSection = (id: string) => setOpenSection(openSection === id ? null : id);

  // ... (Toàn bộ logic khởi tạo config, API call, handleSave được GIỮ NGUYÊN 100%)
  const [config, setConfig] = useState<any>({
    control_mode: 'auto', is_enabled: true,
    ec_target: 1.5, ec_tolerance: 0.05, ph_target: 6.0, ph_tolerance: 0.5, temp_target: 24.0, temp_tolerance: 2.0,
    misting_on_duration_ms: 10000, misting_off_duration_ms: 180000,
    misting_temp_threshold: 30.0, high_temp_misting_on_duration_ms: 15000, high_temp_misting_off_duration_ms: 60000,
    tank_height: 50, water_level_min: 20.0, water_level_target: 80.0, water_level_max: 90.0, water_level_drain: 5.0,
    water_level_tolerance: 5.0, auto_refill_enabled: true, auto_drain_overflow: true, auto_dilute_enabled: false, dilute_drain_amount_cm: 5.0,
    scheduled_water_change_enabled: false, water_change_cron: '0 0 7 * * SUN', scheduled_drain_amount_cm: 10.0,
    ec_gain_per_ml: 0.1, ph_shift_up_per_ml: 0.2, ph_shift_down_per_ml: 0.2,
    ec_step_ratio: 0.4, ph_step_ratio: 0.1, delay_between_a_and_b_sec: 10,
    pump_a_capacity_ml_per_sec: 1.2, pump_b_capacity_ml_per_sec: 1.2, pump_ph_up_capacity_ml_per_sec: 1.2, pump_ph_down_capacity_ml_per_sec: 1.2,
    active_mixing_sec: 5, sensor_stabilize_sec: 5, scheduled_mixing_interval_sec: 3600, scheduled_mixing_duration_sec: 300,
    dosing_pwm_percent: 50, osaka_mixing_pwm_percent: 60, osaka_misting_pwm_percent: 100, soft_start_duration: 3000,
    scheduled_dosing_enabled: false, scheduled_dosing_cron: '0 0 8 * * *', scheduled_dose_a_ml: 10.0, scheduled_dose_b_ml: 10.0,
    dosing_min_pwm_percent: 20, pump_a_min_pwm_percent: 20, pump_b_min_pwm_percent: 20, pump_ph_up_min_pwm_percent: 20, pump_ph_down_min_pwm_percent: 20,
    dosing_pulse_on_ms: 500, dosing_pulse_off_ms: 500, dosing_min_dose_ml: 1.0, dosing_max_pulse_count_per_cycle: 20,
    min_ec_limit: 0.5, max_ec_limit: 3.0, min_ph_limit: 4.0, max_ph_limit: 8.0,
    min_temp_limit: 15.0, max_temp_limit: 35.0, max_ec_delta: 0.5, max_ph_delta: 0.3,
    max_dose_per_cycle: 50.0, max_dose_per_hour: 200.0, cooldown_sec: 60, water_level_critical_min: 10.0,
    max_refill_cycles_per_hour: 3, max_drain_cycles_per_hour: 3, max_refill_duration_sec: 120, max_drain_duration_sec: 120,
    emergency_shutdown: false, ec_ack_threshold: 0.05, ph_ack_threshold: 0.1, water_ack_threshold: 0.5,
    ph_v7: 2.5, ph_v4: 1.428, ph_v10: null, ph_calibration_mode: '2-point',
    ec_factor: 880.0, ec_offset: 0.0, temp_offset: 0.0, temp_compensation_beta: 0.02,
    publish_interval: 5000, moving_average_window: 15,
    enable_ph_sensor: true, enable_ec_sensor: true, enable_temp_sensor: true, enable_water_level_sensor: true,
  });

  const [appSettings, setAppSettings] = useState({ api_key: '', backend_url: 'http://localhost:8000', device_id: '' });
  const [calibrationPointsCount, setCalibrationPointsCount] = useState<2 | 3>(2);
  const [wizardStep, setWizardStep] = useState(0);
  const [isCapturingPoint, setIsCapturingPoint] = useState(false);
  const [countdown, setCountdown] = useState(0);
  const [stabilityStatus, setStabilityStatus] = useState<'idle' | 'waiting' | 'stable'>('idle');
  const [capturedPoints, setCapturedPoints] = useState<Record<number, { voltage: number; confidence: number; capturedAt: string }>>({});
  const [adaptivePhases, setAdaptivePhases] = useState({ observe: true, recommend: true, auto_apply: false, confidence_threshold: 85 });

  const calibrationPoints = calibrationPointsCount === 3 ? [7, 4, 10] : [7, 4];
  const activePoint = calibrationPoints[wizardStep];
  const isPhError = sensorData?.err_ph === true;
  const isCalibrationBlocked = !isSensorOnline || isPhError;

  const callApi = async (path: string, method: string = 'GET', body: any = null, currentSettings: any = appSettings, customTimeoutMs?: number) => {
    const url = `${currentSettings.backend_url}${path}`;
    const options: any = { method, headers: { 'Content-Type': 'application/json', 'X-API-Key': currentSettings.api_key } };
    if (customTimeoutMs) { options.connectTimeout = customTimeoutMs; options.timeout = customTimeoutMs; }
    if (body) options.body = JSON.stringify(body);
    const res = await fetch(url, options);
    if (!res.ok) {
      let errDetail = `HTTP ${res.status}`;
      try { errDetail = `${res.status}: ${await res.text()}`; } catch (_) { }
      throw new Error(errDetail);
    }
    return await res.json();
  };

  const normalizeVoltage = (payload: any): number | null => {
    if (!payload) return null;
    const mvVal = payload?.data?.mean_voltage_mv ?? payload?.mean_voltage_mv;
    if (mvVal !== undefined && mvVal !== null) { const num = Number(mvVal); if (Number.isFinite(num)) return num / 1000.0; }
    const vCandidates = [payload.voltage, payload.ph_voltage, payload.raw_voltage, payload?.data?.voltage, payload?.data?.ph_voltage, payload?.result?.voltage, payload?.result?.ph_voltage];
    for (const value of vCandidates) { if (value === undefined || value === null) continue; const numberValue = Number(value); if (Number.isFinite(numberValue)) return numberValue; }
    return null;
  };

  const normalizeConfidence = (payload: any): number => {
    const candidates = [payload?.confidence, payload?.data?.confidence, payload?.result?.confidence];
    for (const value of candidates) { const numberValue = Number(value); if (Number.isFinite(numberValue)) return Math.max(0, Math.min(100, numberValue)); }
    return 0;
  };

  const handleCapturePoint = async () => {
    if (!activePoint || isCalibrationBlocked || isCapturingPoint) return;
    const currentDeviceId = appSettings.device_id || ctxDeviceId;
    const currentSettings = runtimeSettings || appSettings;
    if (!currentDeviceId || !currentSettings?.backend_url) { toast.error('Thiếu Device ID hoặc URL máy chủ.'); return; }
    setIsCapturingPoint(true);
    if (wizardStep === 0) {
      try { await callApi(`/api/devices/${currentDeviceId}/calibration/ph/start`, 'POST', { mode: calibrationPointsCount === 3 ? '3-point' : '2-point' }, currentSettings); }
      catch (error: any) { toast.error(`Lỗi: ${error.message}`); setIsCapturingPoint(false); return; }
    }
    const targetSamples = 5;
    const intervalSec = Number(config.publish_interval || 5000) / 1000;
    const dynamicWindowSec = Math.ceil((targetSamples + 2) * intervalSec) + 5;
    const requestTimeoutMs = (dynamicWindowSec + 5) * 1000;
    setCountdown(dynamicWindowSec);
    setStabilityStatus('waiting');
    const timer = setInterval(() => {
      setCountdown((prev) => { if (prev <= 1) { clearInterval(timer); setStabilityStatus('stable'); return 0; } return prev - 1; });
    }, 1000);
    try {
      const captureRes = await callApi(`/api/devices/${currentDeviceId}/calibration/ph/capture`, 'POST', { point: activePoint, sample_target: targetSamples, window_seconds: dynamicWindowSec }, currentSettings, requestTimeoutMs);
      const voltage = normalizeVoltage(captureRes);
      if (voltage === null) throw new Error('Không nhận được giá trị.');
      setCapturedPoints((prev) => ({ ...prev, [activePoint]: { voltage, confidence: normalizeConfidence(captureRes), capturedAt: new Date().toISOString() } }));
      toast.success(`Đã ghi nhận điểm pH ${activePoint}.`);
    } catch (error) { toast.error(`Không thể đo điểm pH ${activePoint}.`); }
    finally { clearInterval(timer); setIsCapturingPoint(false); setCountdown(0); setStabilityStatus('idle'); }
  };

  const goToNextPoint = () => { if (wizardStep < calibrationPoints.length - 1) { setWizardStep((prev) => prev + 1); return; } setWizardStep(calibrationPoints.length); };

  const calibrationSummary = (() => {
    const p7 = capturedPoints[7]?.voltage; const p4 = capturedPoints[4]?.voltage; const p10 = capturedPoints[10]?.voltage;
    const confList = Object.values(capturedPoints).map((p) => p.confidence);
    const avgConf = confList.length ? Math.round(confList.reduce((s, v) => s + v, 0) / confList.length) : 0;
    const spread = Number.isFinite(p7) && Number.isFinite(p4) ? Math.abs((p7 as number) - (p4 as number)) : 0;
    const spreadBonus = spread >= 0.2 ? 15 : spread >= 0.1 ? 8 : 0;
    return {
      ph_v7: Number.isFinite(p7) ? Number((p7 as number).toFixed(3)) : null,
      ph_v4: Number.isFinite(p4) ? Number((p4 as number).toFixed(3)) : null,
      ph_v10: Number.isFinite(p10) ? Number((p10 as number).toFixed(3)) : null,
      reliability: Math.max(0, Math.min(100, avgConf + spreadBonus))
    };
  })();

  const phaseConfigStorageKey = 'adaptive-calibration-phase-by-device';
  const effectiveDeviceId = appSettings.device_id || ctxDeviceId || '';

  useEffect(() => {
    if (!effectiveDeviceId) return;
    try {
      const raw = localStorage.getItem(phaseConfigStorageKey);
      const perDevice = raw ? JSON.parse(raw)?.[effectiveDeviceId] : null;
      if (perDevice && typeof perDevice === 'object') setAdaptivePhases((prev) => ({ ...prev, ...perDevice }));
    } catch (error) { }
  }, [effectiveDeviceId]);

  const saveAdaptivePhases = (nextValue: any) => {
    setAdaptivePhases(nextValue);
    if (!effectiveDeviceId) return;
    try {
      const all = JSON.parse(localStorage.getItem(phaseConfigStorageKey) || '{}');
      all[effectiveDeviceId] = nextValue; localStorage.setItem(phaseConfigStorageKey, JSON.stringify(all));
    } catch (error) { }
  };

  const applyCalibrationToConfig = (): any | null => {
    if (calibrationSummary.ph_v7 === null || calibrationSummary.ph_v4 === null || (calibrationPointsCount === 3 && calibrationSummary.ph_v10 === null)) {
      toast.error(`Chưa đủ dữ liệu.`); return null;
    }
    const nextConfig = { ...config, ph_v7: calibrationSummary.ph_v7, ph_v4: calibrationSummary.ph_v4, ph_v10: calibrationSummary.ph_v10, ph_calibration_mode: calibrationPointsCount === 3 ? '3-point' : '2-point' };
    setConfig(nextConfig); toast.success('Đã áp dụng kết quả.'); return nextConfig;
  };

  useEffect(() => {
    const loadConfig = async () => {
      try {
        setIsLoading(true);
        let settings: any = null;
        try { settings = await invoke('load_settings'); if (settings) setAppSettings(settings); } catch (e) { }
        const currentDeviceId = settings?.device_id || appSettings.device_id;
        if (!currentDeviceId) return;
        const unifiedData = await callApi(`/api/devices/${currentDeviceId}/config/unified`, 'GET', null, settings).catch(() => null);
        if (unifiedData) {
          setConfig((prev: any) => ({ ...prev, ...unifiedData.device_config, ...unifiedData.water_config, ...unifiedData.safety_config, ...unifiedData.sensor_calibration, ...unifiedData.dosing_calibration }));
        }
      } catch (error) { } finally { setIsLoading(false); }
    };
    loadConfig();
  }, []);

  const dosingValidationErrors = useMemo(() => validateDosingConfig(config), [config]);
  const hasDosingValidationError = Object.keys(dosingValidationErrors).length > 0;

  const handleSave = async (configOverride?: any) => {
    if (!appSettings.device_id || !appSettings.backend_url) { toast.error('Thiếu thông tin kết nối.'); return; }
    setIsSaving(true); const toastId = toast.loading("Đang lưu...");
    try {
      const savingConfig = configOverride || config;
      if (Object.keys(validateDosingConfig(savingConfig)).length > 0) { toast.error('Dữ liệu không hợp lệ.'); return; }
      const devId = appSettings.device_id;
      const toNumberOr = (value: any, fallback: number) => { const parsed = Number(value); return Number.isFinite(parsed) ? parsed : fallback; };
      try { await invoke('save_settings', { apiKey: appSettings.api_key, backendUrl: appSettings.backend_url, deviceId: devId }); } catch (e) { }
      const ts = new Date().toISOString();

      const unifiedPayload = {
        device_config: {
          device_id: devId, control_mode: savingConfig.control_mode || 'manual', is_enabled: savingConfig.is_enabled ?? true,
          ec_target: toNumberOr(savingConfig.ec_target, 1.5), ec_tolerance: toNumberOr(savingConfig.ec_tolerance, 0.05),
          ph_target: toNumberOr(savingConfig.ph_target, 6.0), ph_tolerance: toNumberOr(savingConfig.ph_tolerance, 0.5),
          temp_target: toNumberOr(savingConfig.temp_target, 24.0), temp_tolerance: toNumberOr(savingConfig.temp_tolerance, 2.0),
          last_updated: ts, delay_between_a_and_b_sec: toNumberOr(savingConfig.delay_between_a_and_b_sec, 10),
        },
        water_config: {
          device_id: devId, tank_height: toNumberOr(savingConfig.tank_height, 50),
          water_level_min: toNumberOr(savingConfig.water_level_min, 20.0), water_level_target: toNumberOr(savingConfig.water_level_target, 80.0),
          water_level_max: toNumberOr(savingConfig.water_level_max, 90.0), water_level_drain: toNumberOr(savingConfig.water_level_drain, 5.0),
          water_level_tolerance: toNumberOr(savingConfig.water_level_tolerance, 5.0), auto_refill_enabled: savingConfig.auto_refill_enabled ?? true,
          auto_drain_overflow: savingConfig.auto_drain_overflow ?? true, auto_dilute_enabled: savingConfig.auto_dilute_enabled ?? false,
          dilute_drain_amount_cm: toNumberOr(savingConfig.dilute_drain_amount_cm, 5.0), scheduled_water_change_enabled: savingConfig.scheduled_water_change_enabled ?? false,
          water_change_cron: String(savingConfig.water_change_cron || '0 0 7 * * SUN'), scheduled_drain_amount_cm: toNumberOr(savingConfig.scheduled_drain_amount_cm, 10.0),
          misting_on_duration_ms: toNumberOr(savingConfig.misting_on_duration_ms, 10000), misting_off_duration_ms: toNumberOr(savingConfig.misting_off_duration_ms, 180000),
          misting_temp_threshold: toNumberOr(savingConfig.misting_temp_threshold, 30.0), high_temp_misting_on_duration_ms: toNumberOr(savingConfig.high_temp_misting_on_duration_ms, 15000),
          high_temp_misting_off_duration_ms: toNumberOr(savingConfig.high_temp_misting_off_duration_ms, 60000), last_updated: ts,
        },
        safety_config: {
          device_id: devId, emergency_shutdown: savingConfig.emergency_shutdown ?? false,
          max_ec_limit: toNumberOr(savingConfig.max_ec_limit, 3.0), min_ec_limit: toNumberOr(savingConfig.min_ec_limit, 0.5),
          min_ph_limit: toNumberOr(savingConfig.min_ph_limit, 4.0), max_ph_limit: toNumberOr(savingConfig.max_ph_limit, 8.0),
          max_ec_delta: toNumberOr(savingConfig.max_ec_delta, 0.5), max_ph_delta: toNumberOr(savingConfig.max_ph_delta, 0.3),
          max_dose_per_cycle: toNumberOr(savingConfig.max_dose_per_cycle, 50.0), cooldown_sec: toNumberOr(savingConfig.cooldown_sec, 60),
          max_dose_per_hour: toNumberOr(savingConfig.max_dose_per_hour, 200.0), water_level_critical_min: toNumberOr(savingConfig.water_level_critical_min, 10.0),
          max_refill_cycles_per_hour: toNumberOr(savingConfig.max_refill_cycles_per_hour, 3), max_drain_cycles_per_hour: toNumberOr(savingConfig.max_drain_cycles_per_hour, 3),
          max_refill_duration_sec: toNumberOr(savingConfig.max_refill_duration_sec, 120), max_drain_duration_sec: toNumberOr(savingConfig.max_drain_duration_sec, 120),
          min_temp_limit: toNumberOr(savingConfig.min_temp_limit, 15.0), max_temp_limit: toNumberOr(savingConfig.max_temp_limit, 35.0),
          ec_ack_threshold: toNumberOr(savingConfig.ec_ack_threshold, 0.05), ph_ack_threshold: toNumberOr(savingConfig.ph_ack_threshold, 0.1), water_ack_threshold: toNumberOr(savingConfig.water_ack_threshold, 0.5), last_updated: ts,
        },
        dosing_calibration: {
          device_id: devId, ec_gain_per_ml: toNumberOr(savingConfig.ec_gain_per_ml, 0.1),
          ph_shift_up_per_ml: toNumberOr(savingConfig.ph_shift_up_per_ml, 0.2), ph_shift_down_per_ml: toNumberOr(savingConfig.ph_shift_down_per_ml, 0.2),
          active_mixing_sec: toNumberOr(savingConfig.active_mixing_sec, 5), sensor_stabilize_sec: toNumberOr(savingConfig.sensor_stabilize_sec, 5),
          ec_step_ratio: toNumberOr(savingConfig.ec_step_ratio, 0.4), ph_step_ratio: toNumberOr(savingConfig.ph_step_ratio, 0.1),
          pump_a_capacity_ml_per_sec: toNumberOr(savingConfig.pump_a_capacity_ml_per_sec, 1.2), pump_b_capacity_ml_per_sec: toNumberOr(savingConfig.pump_b_capacity_ml_per_sec, 1.2),
          pump_ph_up_capacity_ml_per_sec: toNumberOr(savingConfig.pump_ph_up_capacity_ml_per_sec, 1.2), pump_ph_down_capacity_ml_per_sec: toNumberOr(savingConfig.pump_ph_down_capacity_ml_per_sec, 1.2),
          soft_start_duration: toNumberOr(savingConfig.soft_start_duration, 3000), last_calibrated: ts,
          scheduled_mixing_interval_sec: toNumberOr(savingConfig.scheduled_mixing_interval_sec, 3600), scheduled_mixing_duration_sec: toNumberOr(savingConfig.scheduled_mixing_duration_sec, 300),
          dosing_pwm_percent: toNumberOr(savingConfig.dosing_pwm_percent, 50), osaka_mixing_pwm_percent: toNumberOr(savingConfig.osaka_mixing_pwm_percent, 60), osaka_misting_pwm_percent: toNumberOr(savingConfig.osaka_misting_pwm_percent, 100),
          dosing_min_pwm_percent: Math.trunc(toNumberOr(savingConfig.dosing_min_pwm_percent, 20)), pump_a_min_pwm_percent: Math.trunc(toNumberOr(savingConfig.pump_a_min_pwm_percent, 20)),
          pump_b_min_pwm_percent: Math.trunc(toNumberOr(savingConfig.pump_b_min_pwm_percent, 20)), pump_ph_up_min_pwm_percent: Math.trunc(toNumberOr(savingConfig.pump_ph_up_min_pwm_percent, 20)),
          pump_ph_down_min_pwm_percent: Math.trunc(toNumberOr(savingConfig.pump_ph_down_min_pwm_percent, 20)), dosing_pulse_on_ms: Math.trunc(toNumberOr(savingConfig.dosing_pulse_on_ms, 500)),
          dosing_pulse_off_ms: Math.trunc(toNumberOr(savingConfig.dosing_pulse_off_ms, 500)), dosing_min_dose_ml: toNumberOr(savingConfig.dosing_min_dose_ml, 1.0),
          dosing_max_pulse_count_per_cycle: Math.trunc(toNumberOr(savingConfig.dosing_max_pulse_count_per_cycle, 20)), scheduled_dosing_enabled: savingConfig.scheduled_dosing_enabled ?? false,
          scheduled_dosing_cron: String(savingConfig.scheduled_dosing_cron || '0 0 8 * * *'), scheduled_dose_a_ml: toNumberOr(savingConfig.scheduled_dose_a_ml, 10.0), scheduled_dose_b_ml: toNumberOr(savingConfig.scheduled_dose_b_ml, 10.0),
        },
        sensor_calibration: {
          device_id: devId, ph_v7: toNumberOr(savingConfig.ph_v7, 2.5), ph_v4: toNumberOr(savingConfig.ph_v4, 1.428),
          ph_v10: savingConfig.ph_v10 !== null ? Number(savingConfig.ph_v10) : null, ph_calibration_mode: savingConfig.ph_calibration_mode || '2-point',
          ec_factor: toNumberOr(savingConfig.ec_factor, 880.0), ec_offset: toNumberOr(savingConfig.ec_offset, 0.0),
          temp_offset: toNumberOr(savingConfig.temp_offset, 0.0), temp_compensation_beta: toNumberOr(savingConfig.temp_compensation_beta, 0.02),
          publish_interval: toNumberOr(savingConfig.publish_interval, 5000), moving_average_window: toNumberOr(savingConfig.moving_average_window, 15),
          enable_ph_sensor: savingConfig.enable_ph_sensor ?? true, enable_ec_sensor: savingConfig.enable_ec_sensor ?? true,
          enable_temp_sensor: savingConfig.enable_temp_sensor ?? true, enable_water_level_sensor: savingConfig.enable_water_level_sensor ?? true, last_calibrated: ts,
        }
      };

      await callApi(`/api/devices/${devId}/config/unified`, 'PUT', unifiedPayload);
      toast.success('Đã lưu cấu hình.', { id: toastId });
    } catch (error: any) { toast.error(`Lỗi: ${error?.message}`, { id: toastId }); }
    finally { setIsSaving(false); }
  };

  if (isLoading) return <LoadingState message="Đang tải..." />;

  // ------------------------- RENDER GIAO DIỆN (FLAT / MINIMALIST) -------------------------
  return (
    <div className="p-4 md:p-8 max-w-4xl mx-auto pb-32 min-h-screen font-sans">

      {/* Header gọn gàng */}
      <div className="flex flex-col space-y-2 mb-8">
        <h1 className="text-2xl font-semibold text-slate-100 flex items-center gap-3">
          <Settings2 size={24} className="text-slate-400" />
          Cài đặt hệ thống
        </h1>
        <p className="text-sm text-slate-500 pl-9">
          Tùy chỉnh các thông số vận hành của tủ điện
        </p>
      </div>

      {/* Nút bật chế độ Kỹ thuật viên (Chỉ dùng viền mỏng, không màu loè loẹt) */}
      <div className="flex items-center justify-between bg-slate-900/40 p-4 rounded-xl border border-slate-800 mb-8">
        <div className="flex items-center gap-4">
          <div className={`p-2 rounded-md ${isAdvancedMode ? 'bg-red-500/10 text-red-400' : 'bg-slate-800 text-slate-400'}`}>
            <LockKeyhole size={18} />
          </div>
          <div>
            <p className="text-sm font-medium text-slate-200">Chế độ nâng cao</p>
            <p className="text-xs text-slate-500 mt-0.5">Mở khóa thiết lập an toàn và hiệu chuẩn.</p>
          </div>
        </div>
        <Switch isOn={isAdvancedMode} onClick={setIsAdvancedMode} colorClass="bg-red-500" />
      </div>

      <div className="space-y-4">

        {/* NETWORK */}
        <AccordionSection id="network" title="Máy chủ" icon={Network} isOpen={openSection === 'network'} onToggle={() => handleToggleSection('network')}>
          <div className="space-y-4 p-1">
            <InputGroup label="Device ID" type="text" value={appSettings.device_id} onChange={(e: InputEvent) => setAppSettings({ ...appSettings, device_id: e.target.value })} />
            <InputGroup label="Backend URL" type="text" value={appSettings.backend_url} onChange={(e: InputEvent) => setAppSettings({ ...appSettings, backend_url: e.target.value })} />
            <InputGroup label="API Key" type="password" value={appSettings.api_key} onChange={(e: InputEvent) => setAppSettings({ ...appSettings, api_key: e.target.value })} />
          </div>
        </AccordionSection>

        {/* GENERAL */}
        <AccordionSection id="general" title="Tổng quan" icon={Power} isOpen={openSection === 'general'} onToggle={() => handleToggleSection('general')}>
          <div className="space-y-3">
            <div className={`flex items-center justify-between p-4 rounded-xl border ${config.is_enabled ? 'bg-blue-500/10 border-blue-500/30' : 'bg-slate-900/50 border-slate-800'}`}>
              <div>
                <p className={`text-sm font-medium ${config.is_enabled ? 'text-blue-400' : 'text-slate-300'}`}>Kích hoạt tự động</p>
                <p className="text-xs text-slate-500 mt-0.5">Cho phép máy bơm chạy theo kịch bản</p>
              </div>
              <Switch isOn={config.is_enabled} onClick={(val) => setConfig({ ...config, is_enabled: val })} colorClass="bg-blue-500" />
            </div>

            <div className={`flex items-center justify-between p-4 rounded-xl border ${config.emergency_shutdown ? 'bg-red-500/10 border-red-500/30' : 'bg-slate-900/50 border-slate-800'}`}>
              <div className="flex items-center gap-3">
                <ShieldAlert className={config.emergency_shutdown ? 'text-red-400' : 'text-slate-500'} size={20} />
                <div>
                  <p className={`text-sm font-medium ${config.emergency_shutdown ? 'text-red-400' : 'text-slate-300'}`}>Dừng khẩn cấp</p>
                  <p className="text-xs text-slate-500 mt-0.5">Ngắt điện toàn bộ rơ-le</p>
                </div>
              </div>
              <Switch isOn={config.emergency_shutdown} onClick={(val) => setConfig({ ...config, emergency_shutdown: val })} colorClass="bg-red-500" />
            </div>

            <div className="p-4 bg-slate-900/40 rounded-xl border border-slate-800">
              <label className="text-xs font-medium text-slate-400 mb-3 flex items-center gap-2"><Zap size={14} /> Chế độ vận hành</label>
              <div className="flex gap-2">
                <button onClick={() => setConfig({ ...config, control_mode: 'auto' })} className={`flex-1 py-2.5 rounded-lg text-xs font-medium transition-colors ${config.control_mode === 'auto' ? 'bg-slate-700 text-white' : 'bg-slate-900 text-slate-500 border border-slate-800'}`}>Tự động</button>
                <button onClick={() => setConfig({ ...config, control_mode: 'manual' })} className={`flex-1 py-2.5 rounded-lg text-xs font-medium transition-colors ${config.control_mode === 'manual' ? 'bg-slate-700 text-white' : 'bg-slate-900 text-slate-500 border border-slate-800'}`}>Thủ công</button>
              </div>
            </div>
          </div>
        </AccordionSection>

        {/* GROWTH */}
        <AccordionSection id="growth" title="Ngưỡng mục tiêu" icon={Target} isOpen={openSection === 'growth'} onToggle={() => handleToggleSection('growth')}>
          <SubCard title="Dinh Dưỡng (EC) & pH">
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
              <InputGroup label="Mức EC mong muốn" step="0.1" value={config.ec_target} onChange={(e: InputEvent) => setConfig({ ...config, ec_target: e.target.value })} />
              <InputGroup label="Sai số EC (±)" step="0.05" value={config.ec_tolerance} onChange={(e: InputEvent) => setConfig({ ...config, ec_tolerance: e.target.value })} />
              <InputGroup label="Mức pH mong muốn" step="0.1" value={config.ph_target} onChange={(e: InputEvent) => setConfig({ ...config, ph_target: e.target.value })} />
              <InputGroup label="Sai số pH (±)" step="0.05" value={config.ph_tolerance} onChange={(e: InputEvent) => setConfig({ ...config, ph_tolerance: e.target.value })} />
            </div>
          </SubCard>

          <SubCard title="Nhiệt Độ & Làm Mát" className="mt-4">
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
              <InputGroup label="Nhiệt độ tối ưu (°C)" step="0.5" value={config.temp_target} onChange={(e: InputEvent) => setConfig({ ...config, temp_target: e.target.value })} />
              <InputGroup label="Dung sai nhiệt độ (°C)" step="0.5" value={config.temp_tolerance} onChange={(e: InputEvent) => setConfig({ ...config, temp_tolerance: e.target.value })} />
              <div className="sm:col-span-2 mt-2">
                <InputGroup label="Kích hoạt làm mát nhanh khi > (°C)" step="0.5" value={config.misting_temp_threshold} onChange={(e: InputEvent) => setConfig({ ...config, misting_temp_threshold: e.target.value })} />
              </div>
              <div className="sm:col-span-2 pt-3 pb-1 border-t border-slate-800/50"><span className="text-xs font-semibold text-slate-500 uppercase">Trời mát (Mặc định)</span></div>
              <InputGroup label="Phun sương (ms)" step="1000" value={config.misting_on_duration_ms} onChange={(e: InputEvent) => setConfig({ ...config, misting_on_duration_ms: e.target.value })} />
              <InputGroup label="Nghỉ (ms)" step="1000" value={config.misting_off_duration_ms} onChange={(e: InputEvent) => setConfig({ ...config, misting_off_duration_ms: e.target.value })} />
              <div className="sm:col-span-2 pt-3 pb-1 border-t border-slate-800/50"><span className="text-xs font-semibold text-slate-500 uppercase">Trời nóng</span></div>
              <InputGroup label="Phun sương (ms)" step="1000" value={config.high_temp_misting_on_duration_ms} onChange={(e: InputEvent) => setConfig({ ...config, high_temp_misting_on_duration_ms: e.target.value })} />
              <InputGroup label="Nghỉ (ms)" step="1000" value={config.high_temp_misting_off_duration_ms} onChange={(e: InputEvent) => setConfig({ ...config, high_temp_misting_off_duration_ms: e.target.value })} />
            </div>
          </SubCard>
        </AccordionSection>

        {/* WATER */}
        <AccordionSection id="water" title="Quản lý Nước" icon={Waves} isOpen={openSection === 'water'} onToggle={() => handleToggleSection('water')}>
          <SubCard title="Mực Nước (Cảm biến siêu âm)">
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
              <div className="sm:col-span-2"><InputGroup label="Khoảng cách đến đáy (cm)" value={config.tank_height} onChange={(e: InputEvent) => setConfig({ ...config, tank_height: e.target.value })} /></div>
              <InputGroup label="Mức giữ (%)" value={config.water_level_target} onChange={(e: InputEvent) => setConfig({ ...config, water_level_target: e.target.value })} />
              <InputGroup label="Dung sai bù (%)" value={config.water_level_tolerance} onChange={(e: InputEvent) => setConfig({ ...config, water_level_tolerance: e.target.value })} />
              <InputGroup label="Báo cạn (%)" value={config.water_level_min} onChange={(e: InputEvent) => setConfig({ ...config, water_level_min: e.target.value })} />
              <InputGroup label="Báo tràn (%)" value={config.water_level_max} onChange={(e: InputEvent) => setConfig({ ...config, water_level_max: e.target.value })} />
              <div className="sm:col-span-2"><InputGroup label="Xả đáy còn (%)" value={config.water_level_drain} onChange={(e: InputEvent) => setConfig({ ...config, water_level_drain: e.target.value })} /></div>
            </div>
          </SubCard>

          <SubCard title="Van Cấp / Xả Tự Động" className="mt-4">
            <div className="space-y-4">
              <div className="flex items-center justify-between"><span className="text-sm text-slate-300">Tự động bù nước</span><Switch isOn={config.auto_refill_enabled} onClick={(val) => setConfig({ ...config, auto_refill_enabled: val })} /></div>
              <div className="flex items-center justify-between"><span className="text-sm text-slate-300">Tự động xả tràn</span><Switch isOn={config.auto_drain_overflow} onClick={(val) => setConfig({ ...config, auto_drain_overflow: val })} /></div>
              <div className="pt-3 border-t border-slate-800/50">
                <div className="flex items-center justify-between mb-3"><span className="text-sm text-slate-300">Tự xả loãng khi quá EC</span><Switch isOn={config.auto_dilute_enabled} onClick={(val) => setConfig({ ...config, auto_dilute_enabled: val })} /></div>
                {config.auto_dilute_enabled && <InputGroup label="Mức nước xả đi (cm)" step="0.5" value={config.dilute_drain_amount_cm} onChange={(e: InputEvent) => setConfig({ ...config, dilute_drain_amount_cm: e.target.value })} />}
              </div>
            </div>
          </SubCard>

          <SubCard title="Thay Nước Định Kỳ" className="mt-4">
            <div className="flex items-center justify-between mb-4"><span className="text-sm text-slate-300">Bật lịch xả nước cũ</span><Switch isOn={config.scheduled_water_change_enabled} onClick={(val) => setConfig({ ...config, scheduled_water_change_enabled: val })} /></div>
            {config.scheduled_water_change_enabled && (
              <div className="space-y-4">
                <VisualCronPicker label="Lịch" value={config.water_change_cron} onChange={(val) => setConfig({ ...config, water_change_cron: val })} />
                <InputGroup label="Lượng xả (cm)" value={config.scheduled_drain_amount_cm} onChange={(e: InputEvent) => setConfig({ ...config, scheduled_drain_amount_cm: e.target.value })} />
              </div>
            )}
          </SubCard>
        </AccordionSection>

        {/* DOSING */}
        <AccordionSection id="dosing" title="Máy Pha Phân" icon={FlaskConical} isOpen={openSection === 'dosing'} onToggle={() => handleToggleSection('dosing')}>
          {isAdvancedMode && (
            <div className="space-y-4 mb-4">
              <SubCard title="Tốc độ Bơm (PWM)">
                <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                  <InputGroup label="Bơm Phân (%)" value={config.dosing_pwm_percent} onChange={(e: InputEvent) => setConfig({ ...config, dosing_pwm_percent: e.target.value })} errorText={dosingValidationErrors.dosing_pwm_percent} />
                  <InputGroup label="Bơm Trộn (%)" value={config.osaka_mixing_pwm_percent} onChange={(e: InputEvent) => setConfig({ ...config, osaka_mixing_pwm_percent: e.target.value })} />
                  <InputGroup label="Bơm Sương (%)" value={config.osaka_misting_pwm_percent} onChange={(e: InputEvent) => setConfig({ ...config, osaka_misting_pwm_percent: e.target.value })} />
                  <InputGroup label="Soft Start (ms)" value={config.soft_start_duration} onChange={(e: InputEvent) => setConfig({ ...config, soft_start_duration: e.target.value })} />
                </div>
              </SubCard>

              <SubCard title="Nhịp Bơm Nhỏ Giọt (Pulse)">
                <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                  <InputGroup label="PWM tối thiểu (Chung) (%)" value={config.dosing_min_pwm_percent} onChange={(e: InputEvent) => setConfig({ ...config, dosing_min_pwm_percent: e.target.value })} errorText={dosingValidationErrors.dosing_min_pwm_percent} />
                  <InputGroup label="Mức kích hoạt nhịp (ml)" value={config.dosing_min_dose_ml} onChange={(e: InputEvent) => setConfig({ ...config, dosing_min_dose_ml: e.target.value })} />

                  <div className="sm:col-span-2 pt-3 pb-1 border-t border-slate-800/50"><span className="text-xs font-semibold text-slate-500 uppercase">PWM tối thiểu lẻ</span></div>
                  <InputGroup label="Bơm A" value={config.pump_a_min_pwm_percent} onChange={(e: InputEvent) => setConfig({ ...config, pump_a_min_pwm_percent: e.target.value })} />
                  <InputGroup label="Bơm B" value={config.pump_b_min_pwm_percent} onChange={(e: InputEvent) => setConfig({ ...config, pump_b_min_pwm_percent: e.target.value })} />
                  <InputGroup label="Bơm pH Lên" value={config.pump_ph_up_min_pwm_percent} onChange={(e: InputEvent) => setConfig({ ...config, pump_ph_up_min_pwm_percent: e.target.value })} />
                  <InputGroup label="Bơm pH Xuống" value={config.pump_ph_down_min_pwm_percent} onChange={(e: InputEvent) => setConfig({ ...config, pump_ph_down_min_pwm_percent: e.target.value })} />

                  <div className="sm:col-span-2 pt-3 pb-1 border-t border-slate-800/50"><span className="text-xs font-semibold text-slate-500 uppercase">Thời gian nhịp</span></div>
                  <InputGroup label="MỞ (ms)" value={config.dosing_pulse_on_ms} onChange={(e: InputEvent) => setConfig({ ...config, dosing_pulse_on_ms: e.target.value })} />
                  <InputGroup label="TẮT (ms)" value={config.dosing_pulse_off_ms} onChange={(e: InputEvent) => setConfig({ ...config, dosing_pulse_off_ms: e.target.value })} />
                  <div className="sm:col-span-2"><InputGroup label="Max xung/chu kỳ" value={config.dosing_max_pulse_count_per_cycle} onChange={(e: InputEvent) => setConfig({ ...config, dosing_max_pulse_count_per_cycle: e.target.value })} /></div>
                </div>
              </SubCard>
            </div>
          )}

          <SubCard title="Châm Cứng Theo Lịch">
            <div className="flex items-center justify-between mb-4"><span className="text-sm text-slate-300">Bật lịch châm cứng</span><Switch isOn={config.scheduled_dosing_enabled} onClick={(val) => setConfig({ ...config, scheduled_dosing_enabled: val })} /></div>
            {config.scheduled_dosing_enabled && (
              <div className="space-y-4">
                <VisualCronPicker label="Lịch" value={config.scheduled_dosing_cron} onChange={(val) => setConfig({ ...config, scheduled_dosing_cron: val })} />
                <div className="grid grid-cols-2 gap-4">
                  <InputGroup label="A (ml)" value={config.scheduled_dose_a_ml} onChange={(e: InputEvent) => setConfig({ ...config, scheduled_dose_a_ml: e.target.value })} errorText={dosingValidationErrors.scheduled_dose_a_ml} />
                  <InputGroup label="B (ml)" value={config.scheduled_dose_b_ml} onChange={(e: InputEvent) => setConfig({ ...config, scheduled_dose_b_ml: e.target.value })} errorText={dosingValidationErrors.scheduled_dose_b_ml} />
                </div>
              </div>
            )}
          </SubCard>

          <SubCard title="Khuấy Nước" className="mt-4">
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
              <InputGroup label="Chu kỳ khuấy (s)" value={config.scheduled_mixing_interval_sec} onChange={(e: InputEvent) => setConfig({ ...config, scheduled_mixing_interval_sec: e.target.value })} />
              <InputGroup label="Khuấy trong (s)" value={config.scheduled_mixing_duration_sec} onChange={(e: InputEvent) => setConfig({ ...config, scheduled_mixing_duration_sec: e.target.value })} />
              <div className="sm:col-span-2"><InputGroup label="Khuấy sau khi châm (s)" value={config.active_mixing_sec} onChange={(e: InputEvent) => setConfig({ ...config, active_mixing_sec: e.target.value })} /></div>
              <div className="sm:col-span-2"><InputGroup label="Chờ cảm biến ổn định (s)" value={config.sensor_stabilize_sec} onChange={(e: InputEvent) => setConfig({ ...config, sensor_stabilize_sec: e.target.value })} /></div>
            </div>
          </SubCard>

          {isAdvancedMode && (
            <SubCard title="Hệ số vật lý đầu dò" className="mt-4">
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                <div className="sm:col-span-2"><InputGroup label="Chờ nghỉ giữa bơm A & B (s)" value={config.delay_between_a_and_b_sec} onChange={(e: InputEvent) => setConfig({ ...config, delay_between_a_and_b_sec: e.target.value })} /></div>

                <div className="sm:col-span-2 pt-3 pb-1 border-b border-slate-800/50"><span className="text-xs font-semibold text-slate-500 uppercase">Lưu lượng (ml/s)</span></div>
                <InputGroup label="Bơm A" value={config.pump_a_capacity_ml_per_sec} onChange={(e: InputEvent) => setConfig({ ...config, pump_a_capacity_ml_per_sec: e.target.value })} errorText={dosingValidationErrors.pump_a_capacity_ml_per_sec} />
                <InputGroup label="Bơm B" value={config.pump_b_capacity_ml_per_sec} onChange={(e: InputEvent) => setConfig({ ...config, pump_b_capacity_ml_per_sec: e.target.value })} errorText={dosingValidationErrors.pump_b_capacity_ml_per_sec} />
                <InputGroup label="Bơm pH+" value={config.pump_ph_up_capacity_ml_per_sec} onChange={(e: InputEvent) => setConfig({ ...config, pump_ph_up_capacity_ml_per_sec: e.target.value })} errorText={dosingValidationErrors.pump_ph_up_capacity_ml_per_sec} />
                <InputGroup label="Bơm pH-" value={config.pump_ph_down_capacity_ml_per_sec} onChange={(e: InputEvent) => setConfig({ ...config, pump_ph_down_capacity_ml_per_sec: e.target.value })} errorText={dosingValidationErrors.pump_ph_down_capacity_ml_per_sec} />

                <div className="sm:col-span-2 pt-3 pb-1 border-b border-slate-800/50"><span className="text-xs font-semibold text-slate-500 uppercase">Hệ số đậm đặc</span></div>
                <InputGroup label="EC / ml" value={config.ec_gain_per_ml} onChange={(e: InputEvent) => setConfig({ ...config, ec_gain_per_ml: e.target.value })} />
                <InputGroup label="Rải phân EC (0-1)" value={config.ec_step_ratio} onChange={(e: InputEvent) => setConfig({ ...config, ec_step_ratio: e.target.value })} />
                <InputGroup label="pH+ / ml" value={config.ph_shift_up_per_ml} onChange={(e: InputEvent) => setConfig({ ...config, ph_shift_up_per_ml: e.target.value })} />
                <InputGroup label="pH- / ml" value={config.ph_shift_down_per_ml} onChange={(e: InputEvent) => setConfig({ ...config, ph_shift_down_per_ml: e.target.value })} />
                <div className="sm:col-span-2"><InputGroup label="Rải hóa chất pH (0-1)" value={config.ph_step_ratio} onChange={(e: InputEvent) => setConfig({ ...config, ph_step_ratio: e.target.value })} /></div>
              </div>
            </SubCard>
          )}
        </AccordionSection>

        {/* SAFETY */}
        {isAdvancedMode && (
          <AccordionSection id="safety" title="An Toàn" icon={ShieldAlert} isOpen={openSection === 'safety'} onToggle={() => handleToggleSection('safety')}>
            <SubCard title="Ngưỡng Còi Hú">
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                <InputGroup label="Nhiệt độ thấp (°C)" value={config.min_temp_limit} onChange={(e: InputEvent) => setConfig({ ...config, min_temp_limit: e.target.value })} />
                <InputGroup label="Nhiệt độ cao (°C)" value={config.max_temp_limit} onChange={(e: InputEvent) => setConfig({ ...config, max_temp_limit: e.target.value })} />
                <InputGroup label="EC thấp" value={config.min_ec_limit} onChange={(e: InputEvent) => setConfig({ ...config, min_ec_limit: e.target.value })} />
                <InputGroup label="EC cao" value={config.max_ec_limit} onChange={(e: InputEvent) => setConfig({ ...config, max_ec_limit: e.target.value })} />
                <InputGroup label="pH thấp" value={config.min_ph_limit} onChange={(e: InputEvent) => setConfig({ ...config, min_ph_limit: e.target.value })} />
                <InputGroup label="pH cao" value={config.max_ph_limit} onChange={(e: InputEvent) => setConfig({ ...config, max_ph_limit: e.target.value })} />
                <div className="sm:col-span-2"><InputGroup label="Nước cạn tắt bơm (cm)" value={config.water_level_critical_min} onChange={(e: InputEvent) => setConfig({ ...config, water_level_critical_min: e.target.value })} /></div>
              </div>
            </SubCard>

            <SubCard title="Giới hạn chạy thiết bị" className="mt-4">
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                <InputGroup label="Max bơm phân/chu kỳ (ml)" value={config.max_dose_per_cycle} onChange={(e: InputEvent) => setConfig({ ...config, max_dose_per_cycle: e.target.value })} />
                <InputGroup label="Max bơm phân/giờ (ml)" value={config.max_dose_per_hour} onChange={(e: InputEvent) => setConfig({ ...config, max_dose_per_hour: e.target.value })} />
                <div className="sm:col-span-2"><InputGroup label="Nghỉ tản nhiệt bơm (s)" value={config.cooldown_sec} onChange={(e: InputEvent) => setConfig({ ...config, cooldown_sec: e.target.value })} /></div>

                <div className="sm:col-span-2 pt-3 pb-1 border-t border-slate-800/50"><span className="text-xs font-semibold text-slate-500 uppercase">Mạch lọc chống nhiễu</span></div>
                <InputGroup label="Bỏ qua nhảy EC (Δ)" value={config.max_ec_delta} onChange={(e: InputEvent) => setConfig({ ...config, max_ec_delta: e.target.value })} />
                <InputGroup label="Bỏ qua nhảy pH (Δ)" value={config.max_ph_delta} onChange={(e: InputEvent) => setConfig({ ...config, max_ph_delta: e.target.value })} />
                <InputGroup label="Bắt đầu châm nếu lệch EC >" value={config.ec_ack_threshold} onChange={(e: InputEvent) => setConfig({ ...config, ec_ack_threshold: e.target.value })} />
                <InputGroup label="Bắt đầu châm nếu lệch pH >" value={config.ph_ack_threshold} onChange={(e: InputEvent) => setConfig({ ...config, ph_ack_threshold: e.target.value })} />
                <div className="sm:col-span-2"><InputGroup label="Bật máy bơm nước nếu lệch (%) >" value={config.water_ack_threshold} onChange={(e: InputEvent) => setConfig({ ...config, water_ack_threshold: e.target.value })} /></div>
              </div>
            </SubCard>

            <SubCard title="Chống cạn/tràn bồn" className="mt-4">
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                <InputGroup label="Max số lần bơm vào/giờ" value={config.max_refill_cycles_per_hour} onChange={(e: InputEvent) => setConfig({ ...config, max_refill_cycles_per_hour: e.target.value })} />
                <InputGroup label="Max thời gian chạy bơm (s)" value={config.max_refill_duration_sec} onChange={(e: InputEvent) => setConfig({ ...config, max_refill_duration_sec: e.target.value })} />
                <InputGroup label="Max số lần xả/giờ" value={config.max_drain_cycles_per_hour} onChange={(e: InputEvent) => setConfig({ ...config, max_drain_cycles_per_hour: e.target.value })} />
                <InputGroup label="Max thời gian xả (s)" value={config.max_drain_duration_sec} onChange={(e: InputEvent) => setConfig({ ...config, max_drain_duration_sec: e.target.value })} />
              </div>
            </SubCard>
          </AccordionSection>
        )}

        {/* CALIBRATION */}
        <AccordionSection id="sensor" title="Cảm biến" icon={Activity} isOpen={openSection === 'sensor'} onToggle={() => handleToggleSection('sensor')}>
          {isAdvancedMode && (
            <>
              <SubCard title="Truyền thông" className="mb-4">
                <div className="space-y-4">
                  <div>
                    <label className="text-sm font-medium text-slate-300 block mb-1">Cập nhật (ms)</label>
                    <select className="w-full bg-slate-950 border border-slate-800 text-slate-300 text-sm rounded-lg p-2.5 outline-none" value={config.publish_interval} onChange={(e: InputEvent) => setConfig({ ...config, publish_interval: parseInt(e.target.value) })}>
                      <option value={1000}>Nhanh (1s)</option>
                      <option value={5000}>Chuẩn (5s)</option>
                      <option value={10000}>Chậm (10s)</option>
                    </select>
                  </div>
                  <div>
                    <label className="text-sm font-medium text-slate-300 block mb-1">Độ mượt (Window)</label>
                    <div className="flex gap-2">
                      <button onClick={() => setConfig({ ...config, moving_average_window: 5 })} className={`flex-1 py-2 rounded-md text-xs font-medium ${config.moving_average_window <= 5 ? 'bg-slate-700 text-white' : 'bg-slate-900 border border-slate-800 text-slate-400'}`}>Thô (5)</button>
                      <button onClick={() => setConfig({ ...config, moving_average_window: 15 })} className={`flex-1 py-2 rounded-md text-xs font-medium ${config.moving_average_window > 5 && config.moving_average_window <= 20 ? 'bg-slate-700 text-white' : 'bg-slate-900 border border-slate-800 text-slate-400'}`}>Cân bằng (15)</button>
                      <button onClick={() => setConfig({ ...config, moving_average_window: 50 })} className={`flex-1 py-2 rounded-md text-xs font-medium ${config.moving_average_window > 20 ? 'bg-slate-700 text-white' : 'bg-slate-900 border border-slate-800 text-slate-400'}`}>Rất mượt (50)</button>
                    </div>
                  </div>
                </div>
              </SubCard>

              <SubCard title="Bật/Tắt Cảm Biến" className="mb-4">
                <div className="space-y-3">
                  <div className="flex items-center justify-between"><span className="text-sm text-slate-300">pH</span><Switch isOn={config.enable_ph_sensor} onClick={(val) => setConfig({ ...config, enable_ph_sensor: val })} /></div>
                  <div className="flex items-center justify-between"><span className="text-sm text-slate-300">EC</span><Switch isOn={config.enable_ec_sensor} onClick={(val) => setConfig({ ...config, enable_ec_sensor: val })} /></div>
                  <div className="flex items-center justify-between"><span className="text-sm text-slate-300">Nhiệt độ</span><Switch isOn={config.enable_temp_sensor} onClick={(val) => setConfig({ ...config, enable_temp_sensor: val })} /></div>
                  <div className="flex items-center justify-between"><span className="text-sm text-slate-300">Siêu âm</span><Switch isOn={config.enable_water_level_sensor} onClick={(val) => setConfig({ ...config, enable_water_level_sensor: val })} /></div>
                </div>
              </SubCard>
            </>
          )}

          <SubCard title="Hiệu Chuẩn pH">
            <div className="space-y-4">
              <div className="flex items-center gap-2">
                <button onClick={() => { setCalibrationPointsCount(2); setWizardStep(0); setCapturedPoints({}); }} className={`px-3 py-1.5 rounded-md text-xs font-medium ${calibrationPointsCount === 2 ? 'bg-blue-600 text-white' : 'bg-slate-800 text-slate-400'}`}>2 ĐIỂM (7, 4)</button>
                <button onClick={() => { setCalibrationPointsCount(3); setWizardStep(0); setCapturedPoints({}); }} className={`px-3 py-1.5 rounded-md text-xs font-medium ${calibrationPointsCount === 3 ? 'bg-blue-600 text-white' : 'bg-slate-800 text-slate-400'}`}>3 ĐIỂM (7, 4, 10)</button>
              </div>

              {isAdvancedMode && (
                <div className="bg-slate-900/50 p-4 rounded-xl border border-slate-800 space-y-3">
                  <p className="text-xs font-semibold text-slate-500 uppercase">Hành vi Adaptive</p>
                  <div className="flex items-center justify-between"><span className="text-xs text-slate-300">Pha 1: Thu mẫu</span><Switch isOn={adaptivePhases.observe} onClick={(v) => saveAdaptivePhases({ ...adaptivePhases, observe: v })} /></div>
                  <div className="flex items-center justify-between"><span className="text-xs text-slate-300">Pha 2: Chờ xác nhận</span><Switch isOn={adaptivePhases.recommend} onClick={(v) => saveAdaptivePhases({ ...adaptivePhases, recommend: v })} /></div>
                  <div className="flex items-center justify-between"><span className="text-xs text-slate-300">Pha 3: Tự động áp dụng</span><Switch isOn={adaptivePhases.auto_apply} onClick={(v) => saveAdaptivePhases({ ...adaptivePhases, auto_apply: v })} /></div>
                  <InputGroup label="Tin cậy cần thiết (%)" type="number" value={adaptivePhases.confidence_threshold} onChange={(e: InputEvent) => saveAdaptivePhases({ ...adaptivePhases, confidence_threshold: Math.max(0, Math.min(100, Number(e.target.value))) })} />
                </div>
              )}

              {isCalibrationBlocked && (
                <div className="p-3 rounded-lg border border-red-500/30 bg-red-500/10 text-red-400 text-sm">
                  Cảm biến ngoại tuyến hoặc đang lỗi đọc. Vui lòng kiểm tra.
                </div>
              )}

              {wizardStep < calibrationPoints.length ? (
                <div className="p-4 rounded-xl bg-slate-900 border border-slate-800">
                  <p className="text-xs text-slate-500 font-medium mb-1">BƯỚC {wizardStep + 1}/{calibrationPoints.length}</p>
                  <p className="text-sm text-slate-200 mb-3">Nhúng đầu dò vào dung dịch pH {activePoint}</p>
                  <div className="flex items-center gap-3">
                    <button onClick={handleCapturePoint} disabled={isCalibrationBlocked || isCapturingPoint} className="px-4 py-2 rounded-lg bg-blue-600 hover:bg-blue-500 text-white text-sm font-medium disabled:opacity-50">
                      {isCapturingPoint ? 'ĐANG ĐO...' : 'BẮT ĐẦU ĐO'}
                    </button>
                    {isCapturingPoint && <span className="text-xs text-slate-400">{countdown}s</span>}
                    {capturedPoints[activePoint] && !isCapturingPoint && (
                      <button onClick={goToNextPoint} className="px-3 py-2 rounded-lg bg-slate-800 text-slate-200 text-sm font-medium">TIẾP TỤC</button>
                    )}
                  </div>
                </div>
              ) : (
                <div className="p-4 rounded-xl bg-slate-900 border border-slate-800 space-y-4">
                  <div className="grid grid-cols-3 gap-2 text-center">
                    <div className="p-2 bg-slate-950 rounded border border-slate-800"><p className="text-xs text-slate-500">v7</p><p className="text-sm text-slate-200">{calibrationSummary.ph_v7}V</p></div>
                    <div className="p-2 bg-slate-950 rounded border border-slate-800"><p className="text-xs text-slate-500">v4</p><p className="text-sm text-slate-200">{calibrationSummary.ph_v4}V</p></div>
                    <div className="p-2 bg-slate-950 rounded border border-slate-800"><p className="text-xs text-slate-500">Tin cậy</p><p className="text-sm text-slate-200">{calibrationSummary.reliability}%</p></div>
                  </div>

                  <div className="flex gap-2">
                    <button onClick={async () => { const c = applyCalibrationToConfig(); if (c) await handleSave(c); }} className="flex-1 py-2 bg-blue-600 hover:bg-blue-500 text-white text-sm font-medium rounded-lg">
                      XÁC NHẬN LƯU
                    </button>
                  </div>
                </div>
              )}
            </div>
          </SubCard>

          {isAdvancedMode && (
            <SubCard title="Analog (EC/Nhiệt độ)" className="mt-4">
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                <InputGroup label="K Factor (EC)" value={config.ec_factor} onChange={(e: InputEvent) => setConfig({ ...config, ec_factor: e.target.value })} />
                <InputGroup label="Offset (EC)" value={config.ec_offset} onChange={(e: InputEvent) => setConfig({ ...config, ec_offset: e.target.value })} />
                <InputGroup label="Offset (Temp)" value={config.temp_offset} onChange={(e: InputEvent) => setConfig({ ...config, temp_offset: e.target.value })} />
                <InputGroup label="Bù nhiệt EC (Beta)" value={config.temp_compensation_beta} onChange={(e: InputEvent) => setConfig({ ...config, temp_compensation_beta: e.target.value })} />
              </div>
            </SubCard>
          )}
        </AccordionSection>
      </div>

      {/* 🟢 THANH ĐIỀU KHIỂN FIXED Ở ĐÁY (Minimalist) */}
      <div className="fixed bottom-[80px] md:bottom-8 left-0 right-0 z-40 pointer-events-none">
        <div className="max-w-xl mx-auto px-4 flex justify-center">
          <button
            onClick={() => handleSave()}
            disabled={isSaving || hasDosingValidationError}
            className="w-full md:w-auto pointer-events-auto px-8 py-3.5 bg-blue-600 hover:bg-blue-500 text-white rounded-xl font-medium shadow-lg transition-all disabled:opacity-50 flex items-center justify-center gap-2"
          >
            {isSaving ? (
              <span className="w-5 h-5 border-2 border-white/30 border-t-white rounded-full animate-spin"></span>
            ) : (
              <><Save size={18} /> Lưu Cài Đặt</>
            )}
          </button>
        </div>
      </div>
    </div>
  );
};

export default Settings;
