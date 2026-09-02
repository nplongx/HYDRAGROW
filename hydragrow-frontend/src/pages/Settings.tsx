import { useState, useEffect, useMemo, useCallback } from 'react';
import { LoadingState } from '../components/ui/LoadingState';

// --- IMPORT PLATFORM & UTILS ---
import { httpFetch } from '../platform/http';
import { forgetStoredApiKey, loadAppSettings, saveAppSettings } from '../platform/settings';
import { useAuth } from '../contexts/AuthContext';

// --- IMPORT LOGIC ĐÃ BIÊN DỊCH TỪ GLEAM ---
import { validate_dosing_config } from '../../gleam_core/build/dev/javascript/gleam_core/settings/validation.mjs';
import { calculate_summary } from '../../gleam_core/build/dev/javascript/gleam_core/settings/calibration.mjs';
import { build_full_unified_payload_json } from '../../gleam_core/build/dev/javascript/gleam_core/settings/payload.mjs';

import { Save, Settings2 } from 'lucide-react';
import toast from 'react-hot-toast';
import { useDeviceStore } from '../store/useDeviceStore';
import type { OtaStatus, WifiCandidate } from '../types/models';

import { GeneralSection } from './settings/GeneralSection';
import { ThresholdsSection } from './settings/ThresholdsSection';
import { ConnectivitySection } from './settings/ConnectivitySection';
import { DangerZoneSection } from './settings/DangerZoneSection';

type DosingFieldKey =
  | 'dosing_pwm_percent' | 'dosing_min_pwm_percent' | 'pump_a_capacity_ml_per_sec'
  | 'pump_b_capacity_ml_per_sec' | 'pump_ph_up_capacity_ml_per_sec' | 'pump_ph_down_capacity_ml_per_sec';
type DosingValidationErrors = Partial<Record<DosingFieldKey, string>>;

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
    const deviceId = ctxDeviceId;
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
    const deviceId = ctxDeviceId;
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
    ec_step_ratio: 0.4, ph_step_ratio: 0.1,
    ec_a_step_ratio: 0.4, ec_b_step_ratio: 0.4, ph_up_step_ratio: 0.2, ph_down_step_ratio: 0.2,
    delay_between_a_and_b_sec: 10,
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

  const [appSettings, setAppSettings] = useState({ api_key: '', backend_url: 'https://hydragrow.onrender.com' });

  const nodeRedEditorUrl = useMemo(() => {
    try {
      const url = new URL(appSettings.backend_url);
      return `${url.protocol}//${url.hostname}:1880`;
    } catch {
      return 'http://localhost:1880';
    }
  }, [appSettings.backend_url]);

  const integrationTopic = ctxDeviceId ? `hydragrow/${ctxDeviceId}/integrations/out` : 'hydragrow/<device_id>/integrations/out';
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

  const callApi = async (
    path: string,
    method: string = 'GET',
    body: any = null,
    currentSettings: any = appSettings,
    customTimeoutMs?: number,
    extraHeaders?: Record<string, string>
  ) => {
    const url = `${currentSettings.backend_url}${path}`;
    const options: any = {
      method,
      headers: {
        'Content-Type': 'application/json',
        'X-API-Key': currentSettings.api_key,
        ...extraHeaders,
      },
    };
    if (customTimeoutMs) { options.connectTimeout = customTimeoutMs; options.timeout = customTimeoutMs; }
    if (body) options.body = JSON.stringify(body);
    const res = await httpFetch(url, options);
    if (!res.ok) {
      let errDetail = `HTTP ${res.status}`;
      try { errDetail = `${res.status}: ${await res.text()}`; } catch { /* ignore text parse error */ }
      throw new Error(errDetail);
    }
    return await res.json();
  };

  useEffect(() => {
    const deviceId = ctxDeviceId;
    const settings = runtimeSettings || appSettings;
    if (!deviceId || !settings?.backend_url || !settings?.api_key) { setOtaStatus(null); return; }
    callApi(`/api/devices/${deviceId}/ota/status`, 'GET', null, settings)
      .then((status) => setOtaStatus(status as OtaStatus))
      .catch(() => setOtaStatus(null));
    /* eslint-disable-next-line react-hooks/exhaustive-deps */
  }, [appSettings.api_key, appSettings.backend_url, ctxDeviceId, runtimeSettings]);

  const handleTriggerOta = async () => {
    const deviceId = ctxDeviceId;
    const settings = runtimeSettings || appSettings;
    if (!deviceId || !otaStatus?.update_available || isTriggeringOta) return;
    if (!window.confirm(`Cập nhật firmware lên ${otaStatus.latest_version}?\nThiết bị sẽ khởi động lại và tạm ngừng điều khiển trong quá trình cập nhật.`)) return;
    setIsTriggeringOta(true);
    try {
      await callApi(
        `/api/devices/${deviceId}/ota/trigger`,
        'POST',
        {},
        settings,
        undefined,
        { 'X-User-Confirmed': 'true' }
      );
      toast.success('Đã gửi lệnh cập nhật. Theo dõi tiến trình trong Nhật ký hệ thống.');
    } catch { toast.error('Không gửi được lệnh cập nhật firmware.'); }
    finally { setIsTriggeringOta(false); }
  };

  const updateWifiCandidate = (index: number, patch: Partial<WifiCandidate>) => {
    setWifiCandidates((current) => current.map((candidate, candidateIndex) => candidateIndex === index ? { ...candidate, ...patch } : candidate));
  };

  const handleSaveWifiList = async () => {
    const deviceId = ctxDeviceId;
    const settings = runtimeSettings || appSettings;
    const candidates = wifiCandidates.filter((candidate) => candidate.ssid.trim() !== '');
    if (!deviceId) { toast.error('Thiếu Device ID.'); return; }
    if (!candidates.length) { toast.error('Cần nhập ít nhất một SSID.'); return; }
    if (!window.confirm(`Gửi ${candidates.length} mạng WiFi xuống thiết bị?\nThông tin sai có thể khiến thiết bị mất kết nối cho tới khi có người kiểm tra tại chỗ.`)) return;
    setIsSavingWifi(true);
    try {
      await callApi(
        `/api/devices/${deviceId}/wifi`,
        'POST',
        { candidates },
        settings,
        undefined,
        { 'X-User-Confirmed': 'true' }
      );
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
    const currentDeviceId = ctxDeviceId;
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
    } catch { toast.error(`Không thể đo pH ${activePoint}.`); }
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
    const currentDeviceId = ctxDeviceId;
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
      const settings: any = await loadAppSettings();
      if (settings) setAppSettings(settings);
      const currentDeviceId = ctxDeviceId;
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
    } catch { /* ignore config load error */ } finally { setIsLoading(false); }
    /* eslint-disable-next-line react-hooks/exhaustive-deps */
  }, [ctxDeviceId]);

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
    if (!ctxDeviceId || !appSettings.backend_url) { toast.error('Thiếu thông tin kết nối.'); return; }
    setIsSaving(true);
    const toastId = toast.loading("Đang lưu...");
    try {
      const savingConfig = configOverride || config;
      if (Object.keys(dosingValidationErrors).length > 0) { toast.error('Dữ liệu không hợp lệ.'); return; }
      const devId = ctxDeviceId;

      await saveAppSettings({ ...appSettings, device_id: devId });
      const ts = new Date().toISOString();

      const jsonStringPayload = build_full_unified_payload_json(
        devId,
        savingConfig.control_mode || 'manual',
        savingConfig.is_enabled ?? true,
        savingConfig.emergency_shutdown ?? false,
        String(savingConfig.ec_target ?? '1.5'),
        String(savingConfig.ec_tolerance ?? '0.05'),
        String(savingConfig.ph_target ?? '6.0'),
        String(savingConfig.ph_tolerance ?? '0.5'),
        String(savingConfig.delay_between_a_and_b_sec ?? '10'),
        String(savingConfig.tank_height ?? '50'),
        String(savingConfig.water_level_min ?? '20'),
        String(savingConfig.water_level_target ?? '80'),
        String(savingConfig.water_level_max ?? '90'),
        String(savingConfig.water_level_tolerance ?? '5'),
        savingConfig.auto_refill_enabled ?? true,
        savingConfig.auto_drain_overflow ?? true,
        savingConfig.auto_dilute_enabled ?? false,
        String(savingConfig.dilute_drain_amount_cm ?? '5'),
        savingConfig.scheduled_water_change_enabled ?? false,
        String(savingConfig.water_change_cron || '0 0 7 * * SUN'),
        String(savingConfig.scheduled_drain_amount_cm ?? '10'),
        String(savingConfig.misting_on_duration_ms ?? '10000'),
        String(savingConfig.misting_off_duration_ms ?? '180000'),
        String(savingConfig.misting_temp_threshold ?? '30'),
        String(savingConfig.high_temp_misting_on_duration_ms ?? '15000'),
        String(savingConfig.high_temp_misting_off_duration_ms ?? '60000'),
        String(savingConfig.min_ec_limit ?? '0.5'),
        String(savingConfig.max_ec_limit ?? '3.0'),
        String(savingConfig.min_ph_limit ?? '4.0'),
        String(savingConfig.max_ph_limit ?? '8.0'),
        String(savingConfig.max_ec_delta ?? '0.5'),
        String(savingConfig.max_ph_delta ?? '0.3'),
        String(savingConfig.max_dose_per_cycle ?? '50'),
        String(savingConfig.max_dose_per_hour ?? '200'),
        String(savingConfig.cooldown_sec ?? '60'),
        String(savingConfig.water_level_critical_min ?? '10'),
        String(savingConfig.max_refill_cycles_per_hour ?? '3'),
        String(savingConfig.max_drain_cycles_per_hour ?? '3'),
        String(savingConfig.max_refill_duration_sec ?? '120'),
        String(savingConfig.max_drain_duration_sec ?? '120'),
        String(savingConfig.min_temp_limit ?? '15'),
        String(savingConfig.max_temp_limit ?? '35'),
        String(savingConfig.ec_ack_threshold ?? '0.05'),
        String(savingConfig.ph_ack_threshold ?? '0.1'),
        String(savingConfig.water_ack_threshold ?? '0.5'),
        String(savingConfig.ec_gain_per_ml ?? '0.1'),
        String(savingConfig.ph_shift_up_per_ml ?? '0.2'),
        String(savingConfig.ph_shift_down_per_ml ?? '0.2'),
        String(savingConfig.active_mixing_sec ?? '5'),
        String(savingConfig.sensor_stabilize_sec ?? '5'),
        String(savingConfig.ec_step_ratio ?? '0.4'),
        String(savingConfig.ph_step_ratio ?? '0.1'),
        String(savingConfig.ec_a_step_ratio ?? savingConfig.ec_step_ratio ?? '0.4'),
        String(savingConfig.ec_b_step_ratio ?? savingConfig.ec_step_ratio ?? '0.4'),
        String(savingConfig.ph_up_step_ratio ?? savingConfig.ph_step_ratio ?? '0.2'),
        String(savingConfig.ph_down_step_ratio ?? savingConfig.ph_step_ratio ?? '0.2'),
        String(savingConfig.pump_a_capacity_ml_per_sec ?? '1.2'),
        String(savingConfig.pump_b_capacity_ml_per_sec ?? '1.2'),
        String(savingConfig.pump_ph_up_capacity_ml_per_sec ?? '1.2'),
        String(savingConfig.pump_ph_down_capacity_ml_per_sec ?? '1.2'),
        String(savingConfig.dosing_pwm_percent ?? '50'),
        String(savingConfig.osaka_mixing_pwm_percent ?? '60'),
        String(savingConfig.osaka_misting_pwm_percent ?? '100'),
        String(savingConfig.dosing_min_pwm_percent ?? '20'),
        String(savingConfig.pump_a_min_pwm_percent ?? savingConfig.dosing_min_pwm_percent ?? '20'),
        String(savingConfig.pump_b_min_pwm_percent ?? savingConfig.dosing_min_pwm_percent ?? '20'),
        String(savingConfig.pump_ph_up_min_pwm_percent ?? savingConfig.dosing_min_pwm_percent ?? '20'),
        String(savingConfig.pump_ph_down_min_pwm_percent ?? savingConfig.dosing_min_pwm_percent ?? '20'),
        String(savingConfig.dosing_pulse_on_ms ?? '500'),
        String(savingConfig.dosing_pulse_off_ms ?? '500'),
        String(savingConfig.dosing_min_dose_ml ?? '1.0'),
        String(savingConfig.dosing_max_pulse_count_per_cycle ?? '20'),
        String(savingConfig.soft_start_duration ?? '3000'),
        String(savingConfig.scheduled_mixing_interval_sec ?? '3600'),
        String(savingConfig.scheduled_mixing_duration_sec ?? '300'),
        String(savingConfig.ph_v7 ?? '2.5'),
        String(savingConfig.ph_v4 ?? '1.428'),
        String(savingConfig.ec_factor ?? '880.0'),
        String(savingConfig.ec_offset ?? '0.0'),
        String(savingConfig.temp_offset ?? '0.0'),
        String(savingConfig.temp_compensation_beta ?? '0.02'),
        String(savingConfig.publish_interval ?? '5000'),
        String(savingConfig.moving_average_window ?? '15'),
        savingConfig.enable_ph_sensor ?? true,
        savingConfig.enable_ec_sensor ?? true,
        savingConfig.enable_temp_sensor ?? true,
        savingConfig.enable_water_level_sensor ?? true,
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
      </div>

      <div className="space-y-6">
        <GeneralSection
          userEmail={user?.email}
          onLogout={() => logout()}
          onGoToPairing={() => { window.location.href = '/pairing'; }}
          isAdvancedMode={isAdvancedMode}
          onToggleAdvancedMode={setIsAdvancedMode}
        />

        <ThresholdsSection
          openSection={openSection}
          onToggleSection={handleToggleSection}
          config={config}
          setConfig={setConfig}
          isAdvancedMode={isAdvancedMode}
          dosingValidationErrors={dosingValidationErrors}
          wizardStep={wizardStep}
          calibrationPoints={calibrationPoints}
          activePoint={activePoint}
          isCalibrationBlocked={isCalibrationBlocked}
          isCapturingPoint={isCapturingPoint}
          countdown={countdown}
          capturedPoints={capturedPoints}
          calibrationSummary={calibrationSummary}
          handleCapturePoint={handleCapturePoint}
          goToNextPoint={goToNextPoint}
          handleFinishAndSaveCalibration={handleFinishAndSaveCalibration}
        />

        <ConnectivitySection
          openSection={openSection}
          onToggleSection={handleToggleSection}
          nodeRedEditorUrl={nodeRedEditorUrl}
          integrationTopic={integrationTopic}
          ctxDeviceId={ctxDeviceId}
          appSettings={appSettings}
          setAppSettings={setAppSettings}
          handleForgetApiKey={handleForgetApiKey}
          otaStatus={otaStatus}
          isTriggeringOta={isTriggeringOta}
          handleTriggerOta={handleTriggerOta}
          wifiCandidates={wifiCandidates}
          setWifiCandidates={setWifiCandidates}
          updateWifiCandidate={updateWifiCandidate}
          isSavingWifi={isSavingWifi}
          handleSaveWifiList={handleSaveWifiList}
        />

        <DangerZoneSection
          rebootLoading={rebootLoading}
          onReboot={sendReboot}
          factoryResetConfirm={factoryResetConfirm}
          onFactoryResetClick={() => setFactoryResetConfirm(true)}
          onConfirmFactoryReset={sendFactoryReset}
          onCancelFactoryReset={() => setFactoryResetConfirm(false)}
        />
      </div>

      {/* THANH ĐIỀU KHIỂN FIXED BOTTOM */}
      <div className="fixed bottom-[84px] md:bottom-[90px] left-0 right-0 z-40 pointer-events-none p-4 md:p-0 flex justify-center md:justify-end md:right-8">
        <button
          type="button"
          onClick={() => handleSave()}
          disabled={isSaving || hasDosingValidationError}
          className="w-full md:w-auto pointer-events-auto px-8 py-3.5 bg-sky-600 hover:bg-sky-500 text-white rounded-xl font-medium shadow-[0_10px_30px_-10px_rgba(2,132,199,0.8)] transition-all hover:-translate-y-1 disabled:opacity-50 disabled:hover:translate-y-0 flex items-center justify-center gap-2"
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
