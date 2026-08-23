import React, { useState, useEffect, useMemo, useCallback } from 'react';
import { SubCard } from '../components/ui/SubCard';
import { AccordionSection } from '../components/ui/AccordionSection';
import { LoadingState } from '../components/ui/LoadingState';

// --- IMPORT PLATFORM & UTILS ---
import { httpFetch } from '../platform/http';
import { forgetStoredApiKey, loadAppSettings, saveAppSettings } from '../platform/settings';
import { useAuth } from '../contexts/AuthContext';

// --- IMPORT LOGIC ĐÃ BIÊN DỊCH TỪ GLEAM ---
import { validate_dosing_config } from '../../gleam_core/build/dev/javascript/gleam_core/settings/validation.mjs';
import { calculate_summary } from '../../gleam_core/build/dev/javascript/gleam_core/settings/calibration.mjs';
import { parse_cron_safe } from '../../gleam_core/build/dev/javascript/gleam_core/settings/cron.mjs';
import { build_unified_payload_json } from '../../gleam_core/build/dev/javascript/gleam_core/settings/payload.mjs';

import { Activity, CalendarClock, FlaskConical, LockKeyhole, Network, Power, Save, Settings2, ShieldAlert, Target, Waves, Zap } from 'lucide-react';
import toast from 'react-hot-toast';
import { Switch } from '../components/ui/Switch';
import { InputGroup } from '../components/ui/InputGroup';
import { useDeviceStore } from '../store/useDeviceStore';
import type { OtaStatus, WifiCandidate } from '../types/models';

type InputEvent = React.ChangeEvent<HTMLInputElement | HTMLSelectElement>;
type DosingFieldKey =
  | 'dosing_pwm_percent' | 'dosing_min_pwm_percent' | 'pump_a_capacity_ml_per_sec'
  | 'pump_b_capacity_ml_per_sec' | 'pump_ph_up_capacity_ml_per_sec' | 'pump_ph_down_capacity_ml_per_sec';
type DosingValidationErrors = Partial<Record<DosingFieldKey, string>>;

// --- COMPONENT TRỰC QUAN HOÁ CRON ---
const VisualCronPicker = ({ value, onChange, label, desc }: {
  value: string; onChange: (val: string) => void; label: string; desc?: string;
}) => {
  const schedule = parse_cron_safe(value || "0 0 8 * * *");
  const minuteStr = String(schedule.minute).padStart(2, '0');
  const hourStr = String(schedule.hour).padStart(2, '0');
  const timeStr = `${hourStr}:${minuteStr}`;
  const isEveryDay = schedule.is_every_day;
  const selectedDays: string[] = schedule.days_str ? schedule.days_str.split(',') : [];

  const handleTimeChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const val = e.target.value;
    if (!val) return;
    const [h, m] = val.split(':');
    const dow = selectedDays.length === 0 || isEveryDay ? '*' : selectedDays.join(',');
    onChange(`0 ${parseInt(m, 10)} ${parseInt(h, 10)} * * ${dow}`);
  };

  const toggleDay = (dayVal: string) => {
    let newDays = [...selectedDays];
    if (newDays.includes(dayVal)) newDays = newDays.filter(d => d !== dayVal);
    else newDays.push(dayVal);
    const newDow = newDays.length === 0 ? '*' : newDays.join(',');
    onChange(`0 ${parseInt(minuteStr, 10)} ${parseInt(hourStr, 10)} * * ${newDow}`);
  };

  const setEveryDay = () => onChange(`0 ${parseInt(minuteStr, 10)} ${parseInt(hourStr, 10)} * * *`);

  const daysOfWeek = [
    { val: 'MON', label: 'T2' }, { val: 'TUE', label: 'T3' }, { val: 'WED', label: 'T4' },
    { val: 'THU', label: 'T5' }, { val: 'FRI', label: 'T6' }, { val: 'SAT', label: 'T7' }, { val: 'SUN', label: 'CN' },
  ];

  return (
    <div className="space-y-4 bg-white/85 border border-emerald-100 p-5 rounded-xl w-full">
      <div>
        <label className="text-sm font-medium text-emerald-950 flex items-center gap-2">
          <CalendarClock size={16} className="text-emerald-800/80" /> {label}
        </label>
        {desc && <p className="text-xs text-emerald-700/75 mt-1">{desc}</p>}
      </div>
      <div className="flex flex-col md:flex-row md:items-center gap-6">
        <div className="bg-white px-4 py-2 rounded-lg border border-emerald-100 flex-shrink-0">
          <input
            type="time"
            value={timeStr}
            onChange={handleTimeChange}
            className="bg-transparent text-emerald-950 text-xl font-medium outline-none text-center cursor-pointer [color-scheme:dark]"
          />
        </div>
        <div className="flex-1 space-y-3">
          <div className="flex items-center gap-3">
            <button
              onClick={setEveryDay}
              className={`px-3 py-1.5 rounded-md text-xs font-medium transition-colors ${isEveryDay ? 'bg-blue-600 text-white' : 'bg-emerald-100 text-emerald-800/80 hover:bg-emerald-200'}`}
            >
              Hằng ngày
            </button>
            <span className="text-xs text-emerald-700/75">hoặc chọn ngày:</span>
          </div>
          <div className="flex flex-wrap gap-2">
            {daysOfWeek.map(day => {
              const isSelected = !isEveryDay && selectedDays.includes(day.val);
              return (
                <button
                  key={day.val}
                  onClick={() => toggleDay(day.val)}
                  className={`w-9 h-9 rounded-full text-xs font-medium transition-colors flex items-center justify-center border ${isSelected ? 'bg-blue-500/20 border-blue-500 text-blue-700' : 'bg-emerald-50 border-emerald-200 text-emerald-800/80 hover:border-emerald-400 hover:text-emerald-950'}`}
                >
                  {day.label}
                </button>
              );
            })}
          </div>
        </div>
      </div>
    </div>
  );
};

// --- COMPONENT SETTINGS CHÍNH ---
const Settings = () => {
  const { user, logout } = useAuth();
  const sensorData = useDeviceStore((s) => s.sensorData);
  const isSensorOnline = useDeviceStore((s) => s.isSensorOnline);
  const runtimeSettings = useDeviceStore((s) => s.settings);
  const ctxDeviceId = useDeviceStore((s) => s.deviceId);

  const [isLoading, setIsLoading] = useState(true);
  const [isSaving, setIsSaving] = useState(false);
  const [openSection, setOpenSection] = useState<string | null>('general');
  const [isAdvancedMode, setIsAdvancedMode] = useState(false);

  const [rebootLoading, setRebootLoading] = useState(false);
  const [factoryResetConfirm, setFactoryResetConfirm] = useState(false);

  async function sendReboot() {
    if (!confirm('Xác nhận reboot thiết bị?')) return;
    setRebootLoading(true);
    const settings = runtimeSettings || appSettings;
    const deviceId = appSettings.device_id || ctxDeviceId;
    try {
      const res = await httpFetch(`${settings?.backend_url}/api/devices/${deviceId}/reboot`, {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
            'X-API-Key': settings?.api_key || '',
            'X-User-Confirmed': 'true'
        }
      });
      if(!res.ok) throw new Error(await res.text());
      toast.success('Lệnh reboot đã được gửi');
    } catch (e: any) {
      toast.error(e.message);
    } finally { setRebootLoading(false); }
  }

  async function sendFactoryReset() {
    const settings = runtimeSettings || appSettings;
    const deviceId = appSettings.device_id || ctxDeviceId;
    try {
      const res = await httpFetch(`${settings?.backend_url}/api/devices/${deviceId}/factory-reset`, {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
            'X-API-Key': settings?.api_key || '',
            'X-User-Confirmed': 'true'
        }
      });
      if(!res.ok) throw new Error(await res.text());
      setFactoryResetConfirm(false);
      toast.success('Lệnh factory reset đã được gửi');
    } catch (e: any) {
      toast.error(e.message);
    }
  }

  const handleToggleSection = (id: string) => setOpenSection(openSection === id ? null : id);

  const [config, setConfig] = useState<any>({
    control_mode: 'auto', is_enabled: true,
    ec_target: 1.5, ec_tolerance: 0.05, ph_target: 6.0, ph_tolerance: 0.5,
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
    dosing_min_pwm_percent: 20, pump_a_min_pwm_percent: 20, pump_b_min_pwm_percent: 20, pump_ph_up_min_pwm_percent: 20, pump_ph_down_min_pwm_percent: 20,
    dosing_pulse_on_ms: 500, dosing_pulse_off_ms: 500, dosing_min_dose_ml: 1.0, dosing_max_pulse_count_per_cycle: 20,
    min_ec_limit: 0.5, max_ec_limit: 3.0, min_ph_limit: 4.0, max_ph_limit: 8.0,
    min_temp_limit: 15.0, max_temp_limit: 35.0, max_ec_delta: 0.5, max_ph_delta: 0.3,
    max_dose_per_cycle: 50.0, max_dose_per_hour: 200.0, cooldown_sec: 60, water_level_critical_min: 10.0,
    max_refill_cycles_per_hour: 3, max_drain_cycles_per_hour: 3, max_refill_duration_sec: 120, max_drain_duration_sec: 120,
    emergency_shutdown: false, ec_ack_threshold: 0.05, ph_ack_threshold: 0.1, water_ack_threshold: 0.5,
    ph_v7: 2.5, ph_v4: 3.04, ph_v10: null, ph_calibration_mode: '2-point',
    ec_factor: 880.0, ec_offset: 0.0, temp_offset: 0.0, temp_compensation_beta: 0.02,
    publish_interval: 5000, moving_average_window: 15,
    enable_ph_sensor: true, enable_ec_sensor: true, enable_temp_sensor: true, enable_water_level_sensor: true,
  });

  const [appSettings, setAppSettings] = useState({ api_key: '', backend_url: 'https://hydragrow.onrender.com', device_id: '' });
  const [otaStatus, setOtaStatus] = useState<OtaStatus | null>(null);
  const [isTriggeringOta, setIsTriggeringOta] = useState(false);
  const [wifiCandidates, setWifiCandidates] = useState<WifiCandidate[]>([{ ssid: '', password: '', priority: 0 }]);
  const [isSavingWifi, setIsSavingWifi] = useState(false);
  const calibrationPoints = [7, 4];
  const [wizardStep, setWizardStep] = useState(0);
  const [isCapturingPoint, setIsCapturingPoint] = useState(false);
  const [countdown, setCountdown] = useState(0);
  const [_stabilityStatus, setStabilityStatus] = useState<'idle' | 'waiting' | 'stable'>('idle');
  const [capturedPoints, setCapturedPoints] = useState<Record<number, { voltage: number; confidence: number; capturedAt: string }>>({});

  const activePoint = calibrationPoints[wizardStep];
  const isPhError = sensorData?.err_ph === true;
  const isCalibrationBlocked = !isSensorOnline || isPhError;

  const callApi = async (path: string, method: string = 'GET', body: any = null, currentSettings: any = appSettings, customTimeoutMs?: number) => {
    const url = `${currentSettings.backend_url}${path}`;
    const options: any = { method, headers: { 'Content-Type': 'application/json', 'X-API-Key': currentSettings.api_key } };
    if (customTimeoutMs) { options.connectTimeout = customTimeoutMs; options.timeout = customTimeoutMs; }
    if (body) options.body = JSON.stringify(body);
    const res = await httpFetch(url, options);
    if (!res.ok) {
      let errDetail = `HTTP ${res.status}`;
      try { errDetail = `${res.status}: ${await res.text()}`; } catch (_) { }
      throw new Error(errDetail);
    }
    return await res.json();
  };

  useEffect(() => {
    const deviceId = appSettings.device_id || ctxDeviceId;
    const settings = runtimeSettings || appSettings;
    if (!deviceId || !settings?.backend_url || !settings?.api_key) { setOtaStatus(null); return; }
    callApi(`/api/devices/${deviceId}/ota/status`, 'GET', null, settings)
      .then((status) => setOtaStatus(status as OtaStatus))
      .catch(() => setOtaStatus(null));
  }, [appSettings.device_id, appSettings.api_key, appSettings.backend_url, ctxDeviceId, runtimeSettings]);

  const handleTriggerOta = async () => {
    const deviceId = appSettings.device_id || ctxDeviceId;
    const settings = runtimeSettings || appSettings;
    if (!deviceId || !otaStatus?.update_available || isTriggeringOta) return;
    if (!window.confirm(`Cập nhật firmware lên ${otaStatus.latest_version}?\nThiết bị sẽ khởi động lại và tạm ngừng điều khiển trong quá trình cập nhật.`)) return;
    setIsTriggeringOta(true);
    try {
      await callApi(`/api/devices/${deviceId}/ota/trigger`, 'POST', {}, settings);
      toast.success('Đã gửi lệnh cập nhật. Theo dõi tiến trình trong Nhật ký hệ thống.');
    } catch { toast.error('Không gửi được lệnh cập nhật firmware.'); }
    finally { setIsTriggeringOta(false); }
  };

  const updateWifiCandidate = (index: number, patch: Partial<WifiCandidate>) => {
    setWifiCandidates((current) => current.map((candidate, candidateIndex) => candidateIndex === index ? { ...candidate, ...patch } : candidate));
  };

  const handleSaveWifiList = async () => {
    const deviceId = appSettings.device_id || ctxDeviceId;
    const settings = runtimeSettings || appSettings;
    const candidates = wifiCandidates.filter((candidate) => candidate.ssid.trim() !== '');
    if (!deviceId) { toast.error('Thiếu Device ID.'); return; }
    if (!candidates.length) { toast.error('Cần nhập ít nhất một SSID.'); return; }
    if (!window.confirm(`Gửi ${candidates.length} mạng WiFi xuống thiết bị?\nThông tin sai có thể khiến thiết bị mất kết nối cho tới khi có người kiểm tra tại chỗ.`)) return;
    setIsSavingWifi(true);
    try {
      await callApi(`/api/devices/${deviceId}/wifi`, 'POST', { candidates }, settings);
      toast.success('Đã gửi danh sách WiFi; thiết bị áp dụng sau lần khởi động tiếp theo.');
    } catch { toast.error('Không gửi được danh sách WiFi.'); }
    finally { setIsSavingWifi(false); }
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
      try { await callApi(`/api/devices/${currentDeviceId}/calibration/ph/start`, 'POST', { mode: '2-point' }, currentSettings); }
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
    } catch (error) { toast.error(`Không thể đo pH ${activePoint}.`); }
    finally { clearInterval(timer); setIsCapturingPoint(false); setCountdown(0); setStabilityStatus('idle'); }
  };

  const goToNextPoint = () => { if (wizardStep < calibrationPoints.length - 1) { setWizardStep((prev) => prev + 1); return; } setWizardStep(calibrationPoints.length); };

  const calibrationSummary = useMemo(() => {
    const p7 = capturedPoints[7]?.voltage;
    const p4 = capturedPoints[4]?.voltage;
    const confList = Object.values(capturedPoints).map((p) => p.confidence);
    const avgConf = confList.length ? Math.round(confList.reduce((s, v) => s + v, 0) / confList.length) : 0;

    const summary = calculate_summary(
      p7 !== undefined ? p7 : null,
      p4 !== undefined ? p4 : null,
      avgConf
    );

    return {
      ph_v7: summary.ph_v7 !== null ? Number(summary.ph_v7.toFixed(3)) : null,
      ph_v4: summary.ph_v4 !== null ? Number(summary.ph_v4.toFixed(3)) : null,
      reliability: summary.reliability
    };
  }, [capturedPoints]);

  const applyCalibrationToConfig = (): any | null => {
    if (calibrationSummary.ph_v7 === null || calibrationSummary.ph_v4 === null) {
      toast.error(`Chưa đủ điểm hiệu chuẩn.`); return null;
    }
    const nextConfig = { ...config, ph_v7: calibrationSummary.ph_v7, ph_v4: calibrationSummary.ph_v4, ph_calibration_mode: '2-point' };
    setConfig(nextConfig); toast.success('Đã áp dụng kết quả.'); return nextConfig;
  };

  const handleFinishAndSaveCalibration = async () => {
    const c = applyCalibrationToConfig();
    if (!c) return;
    const currentDeviceId = appSettings.device_id || ctxDeviceId;
    const currentSettings = runtimeSettings || appSettings;
    if (!currentDeviceId || !currentSettings?.backend_url) return;
    try {
      await callApi(`/api/devices/${currentDeviceId}/calibration/ph/finish`, 'POST', {}, currentSettings);
    } catch (error: any) {
      console.warn('Finish calibration session error (non-fatal):', error.message);
    }
    await handleSave(c);
  };

  const loadConfig = useCallback(async () => {
    try {
      setIsLoading(true);
      let settings: any = await loadAppSettings();
      if (settings) setAppSettings(settings);
      const currentDeviceId = settings?.device_id || appSettings.device_id || ctxDeviceId;
      if (!currentDeviceId) return;
      const unifiedData = await callApi(`/api/devices/${currentDeviceId}/config/unified`, 'GET', null, settings).catch(() => null);
      if (unifiedData) {
        const merged = {
          ...unifiedData.device_config,
          ...unifiedData.water_config,
          ...unifiedData.safety_config,
          ...unifiedData.sensor_calibration,
          ...unifiedData.dosing_calibration
        };
        const ecAliases = {
          ec_target: merged.ec_target ?? merged.ec_target,
          ec_tolerance: merged.ec_tolerance ?? merged.ec_tolerance,
          min_ec_limit: merged.min_ec_limit ?? merged.min_ec_limit,
          max_ec_limit: merged.max_ec_limit ?? merged.max_ec_limit,
          max_ec_delta: merged.max_ec_delta ?? merged.max_ec_delta,
          ec_ack_threshold: merged.ec_ack_threshold ?? merged.ec_ack_threshold,
          ec_gain_per_ml: merged.ec_gain_per_ml ?? merged.ec_gain_per_ml,
          ec_step_ratio: merged.ec_step_ratio ?? merged.ec_step_ratio,
          best_ec_ratio: merged.best_ec_ratio ?? merged.best_ec_ratio,
          enable_ec_sensor: merged.enable_ec_sensor ?? merged.enable_ec_sensor
        };
        setConfig((prev: any) => ({ ...prev, ...merged, ...ecAliases }));
      }
    } catch (error) { } finally { setIsLoading(false); }
  }, [appSettings.device_id, ctxDeviceId]);

  useEffect(() => {
    loadConfig();
  }, [loadConfig]);

  const dosingValidationErrors = useMemo(() => {
    const gleamErrors = validate_dosing_config(
      String(config.dosing_pwm_percent ?? ''),
      String(config.dosing_min_pwm_percent ?? ''),
      String(config.pump_a_capacity_ml_per_sec ?? ''),
      String(config.pump_b_capacity_ml_per_sec ?? ''),
      String(config.pump_ph_up_capacity_ml_per_sec ?? ''),
      String(config.pump_ph_down_capacity_ml_per_sec ?? '')
    );

    const errors: DosingValidationErrors = {};
    if (Array.isArray(gleamErrors)) {
      gleamErrors.forEach((err: any) => {
        errors[err.field as DosingFieldKey] = `Giá trị không hợp lệ cho ${err.field}: ${err.message}.`;
      });
    }
    return errors;
  }, [config]);

  const hasDosingValidationError = Object.keys(dosingValidationErrors).length > 0;

  const handleSave = async (configOverride?: any) => {
    if (!appSettings.device_id || !appSettings.backend_url) { toast.error('Thiếu thông tin kết nối.'); return; }
    setIsSaving(true);
    const toastId = toast.loading("Đang lưu...");
    try {
      const savingConfig = configOverride || config;
      if (Object.keys(dosingValidationErrors).length > 0) { toast.error('Dữ liệu không hợp lệ.'); return; }
      const devId = appSettings.device_id;

      await saveAppSettings({ ...appSettings, device_id: devId });
      const ts = new Date().toISOString();

      const jsonStringPayload = build_unified_payload_json(
        devId,
        savingConfig.control_mode || 'manual',
        savingConfig.is_enabled ?? true,
        savingConfig.emergency_shutdown ?? false,
        String(savingConfig.ec_target ?? ''),
        String(savingConfig.ec_tolerance ?? ''),
        String(savingConfig.ph_target ?? ''),
        String(savingConfig.ph_tolerance ?? ''),
        String(savingConfig.delay_between_a_and_b_sec ?? ''),
        String(savingConfig.tank_height ?? ''),
        String(savingConfig.water_level_min ?? ''),
        String(savingConfig.water_level_target ?? ''),
        String(savingConfig.water_level_max ?? ''),
        String(savingConfig.water_level_tolerance ?? ''),
        savingConfig.auto_refill_enabled ?? true,
        savingConfig.auto_drain_overflow ?? true,
        String(savingConfig.water_change_cron || '0 0 7 * * SUN'),
        String(savingConfig.misting_on_duration_ms ?? ''),
        String(savingConfig.misting_off_duration_ms ?? ''),
        String(savingConfig.min_ec_limit ?? ''),
        String(savingConfig.max_ec_limit ?? ''),
        String(savingConfig.min_ph_limit ?? ''),
        String(savingConfig.max_ph_limit ?? ''),
        String(savingConfig.max_dose_per_cycle ?? ''),
        String(savingConfig.max_dose_per_hour ?? ''),
        String(savingConfig.pump_a_capacity_ml_per_sec ?? ''),
        String(savingConfig.pump_b_capacity_ml_per_sec ?? ''),
        String(savingConfig.pump_ph_up_capacity_ml_per_sec ?? ''),
        String(savingConfig.pump_ph_down_capacity_ml_per_sec ?? ''),
        String(savingConfig.dosing_pwm_percent ?? ''),
        String(savingConfig.ph_v7 ?? ''),
        String(savingConfig.ph_v4 ?? ''),
        ts
      );

      const res = await httpFetch(`${appSettings.backend_url}/api/devices/${devId}/config/unified`, {
        method: 'PUT',
        headers: {
          'Content-Type': 'application/json',
          'X-API-Key': appSettings.api_key
        },
        body: jsonStringPayload
      });

      if (!res.ok) {
        throw new Error(`HTTP ${res.status}`);
      }

      await loadConfig();
      window.dispatchEvent(new Event('hydragrow:settings-updated'));
      toast.success('Đã lưu cấu hình thành công.', { id: toastId });
    } catch (error: any) { toast.error(`Lỗi: ${error?.message}`, { id: toastId }); }
    finally { setIsSaving(false); }
  };

  const handleForgetApiKey = async () => {
    try {
      await forgetStoredApiKey();
      setAppSettings((current) => ({ ...current, api_key: '' }));
      window.dispatchEvent(new Event('hydragrow:settings-updated'));
      toast.success('Đã xóa API key khỏi bộ nhớ an toàn.');
    } catch (error: any) {
      toast.error(`Không thể xóa API key: ${error?.message || error}`);
    }
  };

  if (isLoading) return <LoadingState message="Đang tải cấu hình..." />;

  return (
    <div className="app-page max-w-5xl pb-36">
      {/* Header */}
      <div className="flex flex-col md:flex-row md:items-center justify-between gap-4 mb-6">
        <div className="flex flex-col space-y-1">
          <h1 className="text-2xl font-bold text-emerald-950 flex items-center gap-2">
            Cấu hình khí canh
            <Settings2 size={22} className="text-emerald-700/75" />
          </h1>
          <p className="text-sm text-emerald-800/75 max-w-2xl">
            Điều chỉnh mục tiêu EC, pH, thời gian phun sương. Các thông số nguy hiểm cần được cài đặt cẩn thận.
          </p>
        </div>
        <div className={`flex items-center justify-between gap-4 p-3 rounded-xl border flex-shrink-0 ${isAdvancedMode ? 'bg-amber-50 border-amber-200' : 'bg-white border-emerald-100'}`}>
          <div className="flex items-center gap-3">
            <div className={`p-2 rounded-lg transition-colors ${isAdvancedMode ? 'bg-amber-100 text-amber-800' : 'bg-emerald-100 text-emerald-800/80'}`}>
              <LockKeyhole size={16} />
            </div>
            <div>
              <p className="text-sm font-semibold text-emerald-950">Chế độ kỹ thuật</p>
              <p className="text-[11px] text-emerald-700/75">Mở rộng thông số an toàn & hiệu chuẩn</p>
            </div>
          </div>
          <Switch isOn={isAdvancedMode} onClick={setIsAdvancedMode} colorClass="bg-amber-600" />
        </div>
      </div>

      <div className="space-y-6">
        <SubCard title="Tài khoản đăng nhập">
          <p>Đang đăng nhập: <strong>{user?.email ?? 'Không xác định'}</strong></p>
          <button type="button" onClick={() => logout()}>
            Đăng xuất
          </button>
        </SubCard>

        {/* NETWORK */}
        <AccordionSection id="network" title="Thiết bị & Kết nối" icon={Network} isOpen={openSection === 'network'} onToggle={() => handleToggleSection('network')}>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4 p-1">
            <InputGroup label="Device ID" type="text" value={appSettings.device_id} onChange={(e: InputEvent) => setAppSettings({ ...appSettings, device_id: e.target.value })} />
            <div className="space-y-2">
              <InputGroup label="API Key" type="password" value={appSettings.api_key} onChange={(e: InputEvent) => setAppSettings({ ...appSettings, api_key: e.target.value })} />
              <div className="rounded-xl border border-amber-200 bg-amber-50 p-3 text-xs text-amber-900">
                Web build chỉ lưu API key trong phiên hiện tại; Tauri lưu khoá trong OS credential vault.
              </div>
              <button type="button" onClick={handleForgetApiKey} className="w-full rounded-xl border border-red-200 bg-white/90 px-3 py-2 text-xs font-semibold text-red-600 transition-colors hover:bg-red-50">
                Quên / xoá API key
              </button>
            </div>
          </div>
        </AccordionSection>

        <AccordionSection id="firmware" title="Cập nhật Firmware" icon={Zap} isOpen={openSection === 'firmware'} onToggle={() => handleToggleSection('firmware')}>
          <div className="space-y-3 p-1">
            {otaStatus ? <>
              <div className="flex items-center justify-between rounded-xl border border-emerald-100 bg-white/85 p-3">
                <div><p className="text-xs text-emerald-700/75">Phiên bản hiện tại</p><p className="text-sm font-semibold text-emerald-950">{otaStatus.current_version}</p></div>
                {otaStatus.update_available && <div className="text-right"><p className="text-xs text-amber-700">Có bản mới</p><p className="text-sm font-semibold text-amber-800">{otaStatus.latest_version}</p></div>}
              </div>
              <button type="button" disabled={!otaStatus.update_available || isTriggeringOta} onClick={handleTriggerOta} className="w-full rounded-xl border border-amber-300 bg-amber-500 px-3 py-2 text-sm font-semibold text-white transition-colors hover:bg-amber-600 disabled:cursor-not-allowed disabled:opacity-50">
                {isTriggeringOta ? 'Đang gửi lệnh cập nhật...' : otaStatus.update_available ? 'Cập nhật ngay (thiết bị sẽ khởi động lại)' : 'Đã ở phiên bản mới nhất'}
              </button>
            </> : <p className="text-xs text-emerald-700/75">Đang tải thông tin firmware...</p>}
          </div>
        </AccordionSection>

        <AccordionSection id="wifi" title="Mạng WiFi thiết bị (ưu tiên)" icon={Network} isOpen={openSection === 'wifi'} onToggle={() => handleToggleSection('wifi')}>
          <div className="space-y-3 p-1">
            {wifiCandidates.map((candidate, index) => <div key={`${index}-${candidate.priority}`} className="grid grid-cols-1 gap-2 md:grid-cols-[1fr_1fr_80px_32px] md:items-end">
              <InputGroup label={`SSID #${index + 1}`} type="text" value={candidate.ssid} onChange={(event: InputEvent) => updateWifiCandidate(index, { ssid: event.target.value })} />
              <InputGroup label="Mật khẩu" type="password" value={candidate.password} onChange={(event: InputEvent) => updateWifiCandidate(index, { password: event.target.value })} />
              <InputGroup label="Ưu tiên" type="number" value={String(candidate.priority)} onChange={(event: InputEvent) => updateWifiCandidate(index, { priority: Math.max(0, Math.min(255, Number(event.target.value) || 0)) })} />
              <button type="button" aria-label={`Xóa SSID ${index + 1}`} onClick={() => setWifiCandidates((current) => current.filter((_, candidateIndex) => candidateIndex !== index))} className="pb-2 text-xs text-red-500">✕</button>
            </div>)}
            <button type="button" onClick={() => setWifiCandidates((current) => [...current, { ssid: '', password: '', priority: current.length }])} className="text-xs font-medium text-emerald-700">+ Thêm mạng WiFi</button>
            <button type="button" disabled={isSavingWifi} onClick={handleSaveWifiList} className="w-full rounded-xl border border-emerald-300 bg-emerald-600 px-3 py-2 text-sm font-semibold text-white hover:bg-emerald-700 disabled:opacity-50">{isSavingWifi ? 'Đang gửi...' : 'Lưu danh sách WiFi (áp dụng sau khi khởi động lại)'}</button>
          </div>
        </AccordionSection>

        {/* GENERAL */}
        <AccordionSection id="general" title="Tổng quan" icon={Power} isOpen={openSection === 'general'} onToggle={() => handleToggleSection('general')}>
          <div className="grid grid-cols-1 lg:grid-cols-3 gap-4">
            <div className={`flex items-center justify-between p-4 rounded-xl border transition-all ${config.is_enabled ? 'bg-blue-50 border-blue-500/30' : 'bg-white/85 border-emerald-100'}`}>
              <p className={`text-sm font-medium ${config.is_enabled ? 'text-blue-700' : 'text-emerald-900'}`}>Kích hoạt hệ thống</p>
              <Switch isOn={config.is_enabled} onClick={(val) => setConfig({ ...config, is_enabled: val })} colorClass="bg-blue-500" />
            </div>
            <div className={`flex items-center justify-between p-4 rounded-xl border transition-all ${config.emergency_shutdown ? 'bg-red-500/10 border-red-500/30' : 'bg-white/85 border-emerald-100'}`}>
              <div className="flex items-center gap-3">
                <ShieldAlert className={config.emergency_shutdown ? 'text-red-400' : 'text-emerald-700/75'} size={20} />
                <p className={`text-sm font-medium ${config.emergency_shutdown ? 'text-red-400' : 'text-emerald-900'}`}>Dừng khẩn cấp</p>
              </div>
              <Switch isOn={config.emergency_shutdown} onClick={(val) => setConfig({ ...config, emergency_shutdown: val })} colorClass="bg-red-500" />
            </div>
            <div className="p-3 bg-white/80 rounded-xl border border-emerald-100 flex flex-col justify-center">
              <label className="text-xs font-medium text-emerald-800/80 mb-2 flex items-center gap-2"><Zap size={14} /> Chế độ vận hành</label>
              <div className="flex gap-2">
                <button onClick={() => setConfig({ ...config, control_mode: 'auto' })} className={`flex-1 py-2 rounded-lg text-xs font-medium transition-colors ${config.control_mode === 'auto' ? 'bg-emerald-700 text-white shadow-sm' : 'bg-white/90 text-emerald-700/75 border border-emerald-100 hover:bg-emerald-100'}`}>Tự động</button>
                <button onClick={() => setConfig({ ...config, control_mode: 'manual' })} className={`flex-1 py-2 rounded-lg text-xs font-medium transition-colors ${config.control_mode === 'manual' ? 'bg-emerald-700 text-white shadow-sm' : 'bg-white/90 text-emerald-700/75 border border-emerald-100 hover:bg-emerald-100'}`}>Thủ công</button>
              </div>
            </div>
          </div>
        </AccordionSection>

        {/* GROWTH */}
        <AccordionSection id="growth" title="Ngưỡng mục tiêu" icon={Target} isOpen={openSection === 'growth'} onToggle={() => handleToggleSection('growth')}>
          <SubCard title="Dinh dưỡng (EC) & pH">
            <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
              <InputGroup label="EC mục tiêu" step="0.1" value={config.ec_target} onChange={(e: InputEvent) => setConfig({ ...config, ec_target: e.target.value })} />
              <InputGroup label="Sai số EC (±)" step="0.05" value={config.ec_tolerance} onChange={(e: InputEvent) => setConfig({ ...config, ec_tolerance: e.target.value })} />
              <InputGroup label="pH mục tiêu" step="0.1" value={config.ph_target} onChange={(e: InputEvent) => setConfig({ ...config, ph_target: e.target.value })} />
              <InputGroup label="Sai số pH (±)" step="0.05" value={config.ph_tolerance} onChange={(e: InputEvent) => setConfig({ ...config, ph_tolerance: e.target.value })} />
            </div>
          </SubCard>
          <SubCard title="Nhiệt độ & Phun sương" className="mt-4">
            <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
              <div className="sm:col-span-2 mt-2">
                <InputGroup label="Kích hoạt sương mạnh khi > (°C)" step="0.5" value={config.misting_temp_threshold} onChange={(e: InputEvent) => setConfig({ ...config, misting_temp_threshold: e.target.value })} />
              </div>
              <div className="sm:col-span-2 lg:col-span-4 pt-3 pb-1 border-t border-emerald-100"><span className="text-xs font-semibold text-emerald-800/80 uppercase tracking-wider">Thời tiết bình thường</span></div>
              <InputGroup label="Phun sương (ms)" step="1000" value={config.misting_on_duration_ms} onChange={(e: InputEvent) => setConfig({ ...config, misting_on_duration_ms: e.target.value })} />
              <InputGroup label="Nghỉ (ms)" step="1000" value={config.misting_off_duration_ms} onChange={(e: InputEvent) => setConfig({ ...config, misting_off_duration_ms: e.target.value })} />
              <div className="hidden lg:block lg:col-span-2"></div>
              <div className="sm:col-span-2 lg:col-span-4 pt-3 pb-1 border-t border-emerald-100"><span className="text-xs font-semibold text-emerald-800/80 uppercase tracking-wider">Nắng nóng</span></div>
              <InputGroup label="Phun sương (ms)" step="1000" value={config.high_temp_misting_on_duration_ms} onChange={(e: InputEvent) => setConfig({ ...config, high_temp_misting_on_duration_ms: e.target.value })} />
              <InputGroup label="Nghỉ (ms)" step="1000" value={config.high_temp_misting_off_duration_ms} onChange={(e: InputEvent) => setConfig({ ...config, high_temp_misting_off_duration_ms: e.target.value })} />
            </div>
          </SubCard>
        </AccordionSection>

        {/* WATER */}
        <AccordionSection id="water" title="Quản lý nước" icon={Waves} isOpen={openSection === 'water'} onToggle={() => handleToggleSection('water')}>
          <SubCard title="Mực nước bồn">
            <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
              <InputGroup label="Chiều cao bồn (cm)" value={config.tank_height} onChange={(e: InputEvent) => setConfig({ ...config, tank_height: e.target.value })} />
              <InputGroup label="Mực nước mục tiêu (cm)" value={config.water_level_target} onChange={(e: InputEvent) => setConfig({ ...config, water_level_target: e.target.value })} />
              <InputGroup label="Sai số (cm)" value={config.water_level_tolerance} onChange={(e: InputEvent) => setConfig({ ...config, water_level_tolerance: e.target.value })} />
              <InputGroup label="Tối thiểu (cm)" value={config.water_level_min} onChange={(e: InputEvent) => setConfig({ ...config, water_level_min: e.target.value })} />
              <InputGroup label="Báo tràn (cm)" value={config.water_level_max} onChange={(e: InputEvent) => setConfig({ ...config, water_level_max: e.target.value })} />
            </div>
          </SubCard>
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-4 mt-4">
            <SubCard title="Bơm & Xả" className="h-full">
              <div className="space-y-4">
                <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                  <div className="flex items-center justify-between p-3 bg-white/80 rounded-lg border border-emerald-100"><span className="text-sm text-emerald-900 font-medium">Tự động cấp nước</span><Switch isOn={config.auto_refill_enabled} onClick={(val) => setConfig({ ...config, auto_refill_enabled: val })} /></div>
                  <div className="flex items-center justify-between p-3 bg-white/80 rounded-lg border border-emerald-100"><span className="text-sm text-emerald-900 font-medium">Tự động xả tràn</span><Switch isOn={config.auto_drain_overflow} onClick={(val) => setConfig({ ...config, auto_drain_overflow: val })} /></div>
                </div>
                <div className="pt-3 border-t border-emerald-100">
                  <div className="flex items-center justify-between mb-3"><span className="text-sm text-emerald-900 font-medium">Tự động pha loãng khi quá EC</span><Switch isOn={config.auto_dilute_enabled} onClick={(val) => setConfig({ ...config, auto_dilute_enabled: val })} /></div>
                  {config.auto_dilute_enabled && (
                    <InputGroup label="Lượng xả pha loãng (cm)" step="0.5" value={config.dilute_drain_amount_cm} onChange={(e: InputEvent) => setConfig({ ...config, dilute_drain_amount_cm: e.target.value })} />
                  )}
                </div>
              </div>
            </SubCard>
            <SubCard title="Thay nước định kỳ" className="h-full">
              <div className="flex items-center justify-between mb-4 p-3 bg-white/80 rounded-lg border border-emerald-100"><span className="text-sm text-emerald-900 font-medium">Bật lịch xả nước</span><Switch isOn={config.scheduled_water_change_enabled} onClick={(val) => setConfig({ ...config, scheduled_water_change_enabled: val })} /></div>
              {config.scheduled_water_change_enabled && (
                <div className="space-y-4">
                  <VisualCronPicker label="Lịch xả tự động" value={config.water_change_cron} onChange={(val) => setConfig({ ...config, water_change_cron: val })} />
                  <InputGroup label="Lượng xả (cm)" value={config.scheduled_drain_amount_cm} onChange={(e: InputEvent) => setConfig({ ...config, scheduled_drain_amount_cm: e.target.value })} />
                </div>
              )}
            </SubCard>
          </div>
        </AccordionSection>

        {/* DOSING */}
        <AccordionSection id="dosing" title="Máy châm phân" icon={FlaskConical} isOpen={openSection === 'dosing'} onToggle={() => handleToggleSection('dosing')}>
          {isAdvancedMode && (
            <div className="grid grid-cols-1 lg:grid-cols-2 gap-4 mb-4">
              <SubCard title="Công suất PWM">
                <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                  <InputGroup label="Bơm châm (%)" value={config.dosing_pwm_percent} onChange={(e: InputEvent) => setConfig({ ...config, dosing_pwm_percent: e.target.value })} errorText={dosingValidationErrors.dosing_pwm_percent} />
                  <InputGroup label="Bơm trộn (%)" value={config.osaka_mixing_pwm_percent} onChange={(e: InputEvent) => setConfig({ ...config, osaka_mixing_pwm_percent: e.target.value })} />
                  <InputGroup label="Bơm sương (%)" value={config.osaka_misting_pwm_percent} onChange={(e: InputEvent) => setConfig({ ...config, osaka_misting_pwm_percent: e.target.value })} />
                  <InputGroup label="Khởi động mềm (ms)" value={config.soft_start_duration} onChange={(e: InputEvent) => setConfig({ ...config, soft_start_duration: e.target.value })} />
                </div>
              </SubCard>
              <SubCard title="Cấu hình xung (Pulse)">
                <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                  <InputGroup label="PWM tối thiểu (%)" value={config.dosing_min_pwm_percent} onChange={(e: InputEvent) => setConfig({ ...config, dosing_min_pwm_percent: e.target.value })} errorText={dosingValidationErrors.dosing_min_pwm_percent} />
                  <InputGroup label="Mức kích hoạt nhịp (ml)" value={config.dosing_min_dose_ml} onChange={(e: InputEvent) => setConfig({ ...config, dosing_min_dose_ml: e.target.value })} />
                  <div className="sm:col-span-2 pt-2 pb-1 border-t border-emerald-100"><span className="text-xs font-semibold text-emerald-700/75 uppercase">Thời gian nhịp</span></div>
                  <InputGroup label="Bật (ms)" value={config.dosing_pulse_on_ms} onChange={(e: InputEvent) => setConfig({ ...config, dosing_pulse_on_ms: e.target.value })} />
                  <InputGroup label="Tắt (ms)" value={config.dosing_pulse_off_ms} onChange={(e: InputEvent) => setConfig({ ...config, dosing_pulse_off_ms: e.target.value })} />
                  <div className="sm:col-span-2"><InputGroup label="Max xung / chu kỳ" value={config.dosing_max_pulse_count_per_cycle} onChange={(e: InputEvent) => setConfig({ ...config, dosing_max_pulse_count_per_cycle: e.target.value })} /></div>
                </div>
              </SubCard>
            </div>
          )}
          <SubCard title="Khuấy trộn" className="mt-4">
            <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
              <InputGroup label="Chu kỳ khuấy (s)" value={config.scheduled_mixing_interval_sec} onChange={(e: InputEvent) => setConfig({ ...config, scheduled_mixing_interval_sec: e.target.value })} />
              <InputGroup label="Thời gian khuấy (s)" value={config.scheduled_mixing_duration_sec} onChange={(e: InputEvent) => setConfig({ ...config, scheduled_mixing_duration_sec: e.target.value })} />
              <InputGroup label="Khuấy sau châm (s)" value={config.active_mixing_sec} onChange={(e: InputEvent) => setConfig({ ...config, active_mixing_sec: e.target.value })} />
              <InputGroup label="Thời gian ổn định cảm biến (s)" value={config.sensor_stabilize_sec} onChange={(e: InputEvent) => setConfig({ ...config, sensor_stabilize_sec: e.target.value })} />
            </div>
          </SubCard>
        </AccordionSection>

        {/* SAFETY */}
        {isAdvancedMode && (
          <AccordionSection id="safety" title="An toàn" icon={ShieldAlert} isOpen={openSection === 'safety'} onToggle={() => handleToggleSection('safety')}>
            <SubCard title="Ngưỡng cảnh báo">
              <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
                <InputGroup label="Nhiệt độ thấp (°C)" value={config.min_temp_limit} onChange={(e: InputEvent) => setConfig({ ...config, min_temp_limit: e.target.value })} />
                <InputGroup label="Nhiệt độ cao (°C)" value={config.max_temp_limit} onChange={(e: InputEvent) => setConfig({ ...config, max_temp_limit: e.target.value })} />
                <InputGroup label="EC thấp" value={config.min_ec_limit} onChange={(e: InputEvent) => setConfig({ ...config, min_ec_limit: e.target.value })} />
                <InputGroup label="EC cao" value={config.max_ec_limit} onChange={(e: InputEvent) => setConfig({ ...config, max_ec_limit: e.target.value })} />
                <InputGroup label="pH thấp" value={config.min_ph_limit} onChange={(e: InputEvent) => setConfig({ ...config, min_ph_limit: e.target.value })} />
                <InputGroup label="pH cao" value={config.max_ph_limit} onChange={(e: InputEvent) => setConfig({ ...config, max_ph_limit: e.target.value })} />
                <div className="sm:col-span-2 lg:col-span-3"><InputGroup label="Nước tối thiểu ngắt khẩn (cm)" value={config.water_level_critical_min} onChange={(e: InputEvent) => setConfig({ ...config, water_level_critical_min: e.target.value })} /></div>
              </div>
            </SubCard>
          </AccordionSection>
        )}

        {/* CALIBRATION */}
        <AccordionSection id="sensor" title="Cảm biến & Hiệu chuẩn" icon={Activity} isOpen={openSection === 'sensor'} onToggle={() => handleToggleSection('sensor')}>
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
            <SubCard title="Hiệu chuẩn pH" className="h-full">
              <div className="space-y-4">
                {isCalibrationBlocked && (
                  <div className="p-3 rounded-lg border border-red-500/30 bg-red-500/10 text-red-400 text-xs flex items-center gap-2">
                    <ShieldAlert size={16} />
                    Cảm biến ngoại tuyến hoặc lỗi hệ thống
                  </div>
                )}
                {wizardStep < calibrationPoints.length ? (
                  <div className="p-5 rounded-xl bg-white border border-emerald-100 shadow-inner">
                    <p className="text-xs text-blue-700 font-bold tracking-wider mb-1">BƯỚC {wizardStep + 1}/{calibrationPoints.length}</p>
                    <p className="text-sm text-emerald-950 mb-4">Nhúng vào dung dịch <span className="font-bold text-emerald-800">pH {activePoint}</span></p>
                    <div className="flex items-center gap-3">
                      <button onClick={handleCapturePoint} disabled={isCalibrationBlocked || isCapturingPoint} className="px-4 py-2 rounded-lg bg-blue-600 hover:bg-blue-500 text-white text-sm font-medium disabled:opacity-50 transition-all">
                        {isCapturingPoint ? 'ĐANG ĐO...' : 'BẮT ĐẦU ĐO'}
                      </button>
                      {isCapturingPoint && <span className="text-sm font-mono text-emerald-900 bg-white px-3 py-1.5 rounded-md">{countdown}s</span>}
                      {capturedPoints[activePoint] && !isCapturingPoint && (
                        <button onClick={goToNextPoint} className="px-4 py-2 rounded-lg bg-emerald-100 hover:bg-emerald-200 text-emerald-950 text-sm font-medium transition-all">TIẾP THEO</button>
                      )}
                    </div>
                  </div>
                ) : (
                  <div className="p-5 rounded-xl bg-white border border-emerald-100 shadow-inner space-y-4">
                    <div className="grid grid-cols-2 gap-2 text-center">
                      <div className="p-2 bg-white rounded-lg border border-emerald-100"><p className="text-[10px] text-emerald-700/75 mb-0.5">V7</p><p className="text-sm font-mono text-emerald-950">{calibrationSummary.ph_v7}V</p></div>
                      <div className="p-2 bg-white rounded-lg border border-emerald-100"><p className="text-[10px] text-emerald-700/75 mb-0.5">V4</p><p className="text-sm font-mono text-emerald-950">{calibrationSummary.ph_v4}V</p></div>
                      <div className="col-span-2 p-2 bg-white rounded-lg border border-emerald-100"><p className="text-[10px] text-emerald-700/75 mb-0.5">Độ tin cậy</p><p className={`text-sm font-mono ${calibrationSummary.reliability >= 80 ? 'text-green-600' : 'text-yellow-600'}`}>{calibrationSummary.reliability}%</p></div>
                    </div>
                    <button onClick={handleFinishAndSaveCalibration} className="w-full py-2.5 bg-blue-600 hover:bg-blue-500 text-white font-medium rounded-lg transition-all text-sm">
                      XÁC NHẬN & LƯU HIỆU CHUẨN
                    </button>
                  </div>
                )}
              </div>
            </SubCard>
          </div>
        </AccordionSection>

        <section className="mt-8 border-t pt-6">
          <h2 className="text-lg font-semibold text-red-600 mb-4">Vùng Nguy Hiểm</h2>
          <div className="flex gap-3">
            <button
              onClick={sendReboot}
              disabled={rebootLoading}
              className="px-4 py-2 border border-orange-400 text-orange-600 rounded-lg text-sm hover:bg-orange-50"
            >
              Reboot Thiết Bị
            </button>
            <button
              onClick={() => setFactoryResetConfirm(true)}
              className="px-4 py-2 border border-red-400 text-red-600 rounded-lg text-sm hover:bg-red-50"
            >
              Factory Reset
            </button>
          </div>
          {factoryResetConfirm && (
            <div className="mt-3 p-4 bg-red-50 rounded-lg">
              <p className="text-sm text-red-700 font-medium mb-3">
                ⚠️ Thao tác này xoá TOÀN BỘ cấu hình (WiFi, recipe, safety budget) và reboot.
                Không thể hoàn tác!
              </p>
              <div className="flex gap-2">
                <button onClick={sendFactoryReset} className="px-3 py-1.5 bg-red-600 text-white rounded text-sm">
                  Xác Nhận Factory Reset
                </button>
                <button onClick={() => setFactoryResetConfirm(false)} className="px-3 py-1.5 border rounded text-sm">
                  Huỷ
                </button>
              </div>
            </div>
          )}
        </section>
      </div>

      {/* THANH ĐIỀU KHIỂN FIXED BOTTOM */}
      <div className="fixed bottom-[84px] md:bottom-[90px] left-0 right-0 z-40 pointer-events-none p-4 md:p-0 flex justify-center md:justify-end md:right-8">
        <button
          onClick={() => handleSave()}
          disabled={isSaving || hasDosingValidationError}
          className="w-full md:w-auto pointer-events-auto px-8 py-3.5 bg-blue-600 hover:bg-blue-500 text-white rounded-xl font-medium shadow-[0_10px_30px_-10px_rgba(37,99,235,0.8)] transition-all hover:-translate-y-1 disabled:opacity-50 disabled:hover:translate-y-0 flex items-center justify-center gap-2"
        >
          {isSaving ? (
            <span className="w-5 h-5 border-2 border-white/30 border-t-white rounded-full animate-spin"></span>
          ) : (
            <><Save size={18} /> Lưu thay đổi</>
          )}
        </button>
      </div>
    </div>
  );
};

export default Settings;
