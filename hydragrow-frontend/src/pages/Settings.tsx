import React, { useState, useEffect } from 'react';
import {
  Save, Target, ShieldAlert, Waves,
  FlaskConical, Activity, Settings2, Power, Network, Zap, Cpu, Clock
} from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { fetch } from '@tauri-apps/plugin-http';
import toast from 'react-hot-toast';

import { Switch } from '../components/ui/Switch';
import { InputGroup } from '../components/ui/InputGroup';
import { SubCard } from '../components/ui/SubCard';
import { AccordionSection } from '../components/ui/AccordionSection';
import { useDeviceContext } from '../context/DeviceContext';

type InputEvent = React.ChangeEvent<HTMLInputElement | HTMLSelectElement>;

const Settings = () => {
  const { sensorData, isSensorOnline, settings: runtimeSettings, deviceId: ctxDeviceId, systemEvents } = useDeviceContext();
  const [isLoading, setIsLoading] = useState(true);
  const [isSaving, setIsSaving] = useState(false);
  const [openSection, setOpenSection] = useState<string | null>('general');

  const handleToggleSection = (id: string) => setOpenSection(openSection === id ? null : id);

  const [config, setConfig] = useState<any>({
    control_mode: 'auto', is_enabled: true,
    ec_target: 1.5, ec_tolerance: 0.05, ph_target: 6.0, ph_tolerance: 0.5, temp_target: 24.0, temp_tolerance: 2.0,
    misting_on_duration_ms: 10000, misting_off_duration_ms: 180000,
    misting_temp_threshold: 30.0, high_temp_misting_on_duration_ms: 15000, high_temp_misting_off_duration_ms: 60000,

    tank_height: 50,
    water_level_min: 20.0, water_level_target: 80.0, water_level_max: 90.0, water_level_drain: 5.0,
    circulation_mode: 'always_on', circulation_on_sec: 1800, circulation_off_sec: 900, water_level_tolerance: 5.0,
    auto_refill_enabled: true, auto_drain_overflow: true, auto_dilute_enabled: false, dilute_drain_amount_cm: 5.0,
    scheduled_water_change_enabled: false, water_change_cron: '0 0 7 * * SUN', scheduled_drain_amount_cm: 10.0,

    tank_volume_l: 50.0, ec_gain_per_ml: 0.1, ph_shift_up_per_ml: 0.2, ph_shift_down_per_ml: 0.2,
    ec_step_ratio: 0.4, ph_step_ratio: 0.1, delay_between_a_and_b_sec: 10,
    pump_a_capacity_ml_per_sec: 1.2, pump_b_capacity_ml_per_sec: 1.2,
    pump_ph_up_capacity_ml_per_sec: 1.2, pump_ph_down_capacity_ml_per_sec: 1.2,

    active_mixing_sec: 5, sensor_stabilize_sec: 5, scheduled_mixing_interval_sec: 3600, scheduled_mixing_duration_sec: 300,
    dosing_pwm_percent: 50, osaka_mixing_pwm_percent: 60, osaka_misting_pwm_percent: 100, soft_start_duration: 3000,
    scheduled_dosing_enabled: false, scheduled_dosing_cron: '0 0 8 * * *', scheduled_dose_a_ml: 10.0, scheduled_dose_b_ml: 10.0,
    ec_gain_dynamic: 0.01, ph_up_dynamic: 0.01, ph_down_dynamic: 0.01,
    dynamic_sample_count: 0, dynamic_confidence: 0, dynamic_model_version: 'v1',

    min_ec_limit: 0.5, max_ec_limit: 3.0, min_ph_limit: 4.0, max_ph_limit: 8.0,
    min_temp_limit: 15.0, max_temp_limit: 35.0, max_ec_delta: 0.5, max_ph_delta: 0.3,
    max_dose_per_cycle: 50.0, max_dose_per_hour: 200.0, cooldown_sec: 60, water_level_critical_min: 10.0,
    max_refill_cycles_per_hour: 3, max_drain_cycles_per_hour: 3, max_refill_duration_sec: 120, max_drain_duration_sec: 120,
    emergency_shutdown: false, ec_ack_threshold: 0.05, ph_ack_threshold: 0.1, water_ack_threshold: 0.5,

    ph_v7: 2.5, ph_v4: 1.428, ec_factor: 880.0, ec_offset: 0.0, temp_offset: 0.0, temp_compensation_beta: 0.02,
    publish_interval: 5000, moving_average_window: 15,
    is_ph_enabled: true, is_ec_enabled: true, is_temp_enabled: true, is_water_level_enabled: true,
  });

  const [appSettings, setAppSettings] = useState({
    api_key: '', backend_url: 'http://localhost:8000', device_id: ''
  });
  const [calibrationPointsCount, setCalibrationPointsCount] = useState<2 | 3>(2);
  const [wizardStep, setWizardStep] = useState(0);
  const [isCapturingPoint, setIsCapturingPoint] = useState(false);
  const [countdown, setCountdown] = useState(0);
  const [stabilityStatus, setStabilityStatus] = useState<'idle' | 'waiting' | 'stable'>('idle');
  const [capturedPoints, setCapturedPoints] = useState<Record<number, { voltage: number; confidence: number; capturedAt: string }>>({});
  const [adaptivePhases, setAdaptivePhases] = useState({
    observe: true,
    recommend: true,
    auto_apply: false,
    confidence_threshold: 85
  });

  const calibrationPoints = calibrationPointsCount === 3 ? [7, 4, 10] : [7, 4];
  const activePoint = calibrationPoints[wizardStep];
  const isPhError = sensorData?.err_ph === true;
  const isCalibrationBlocked = !isSensorOnline || isPhError;

  const callApi = async (path: string, method: string = 'GET', body: any = null, currentSettings: any = appSettings) => {
    const url = `${currentSettings.backend_url}${path}`;
    const options: any = { method, headers: { 'Content-Type': 'application/json', 'X-API-Key': currentSettings.api_key } };
    if (body) options.body = JSON.stringify(body);
    const res = await fetch(url, options);
    if (!res.ok) {
      let errDetail = `HTTP ${res.status}`;
      try {
        const errBody = await res.text();
        console.error(`[API Error] ${method} ${path} →`, res.status, errBody);
        errDetail = `${res.status}: ${errBody}`;
      } catch (_) { }
      throw new Error(errDetail);
    }
    return await res.json();
  };

  const normalizeVoltage = (payload: any): number | null => {
    if (!payload) return null;
    const candidates = [
      payload.voltage,
      payload.ph_voltage,
      payload.raw_voltage,
      payload?.data?.voltage,
      payload?.data?.ph_voltage,
      payload?.result?.voltage,
      payload?.result?.ph_voltage
    ];
    for (const value of candidates) {
      const numberValue = Number(value);
      if (Number.isFinite(numberValue)) return numberValue;
    }
    return null;
  };

  const normalizeConfidence = (payload: any): number => {
    const candidates = [payload?.confidence, payload?.data?.confidence, payload?.result?.confidence];
    for (const value of candidates) {
      const numberValue = Number(value);
      if (Number.isFinite(numberValue)) return Math.max(0, Math.min(100, numberValue));
    }
    return 0;
  };

  const handleCapturePoint = async () => {
    if (!activePoint || isCalibrationBlocked || isCapturingPoint) return;
    const currentDeviceId = appSettings.device_id || ctxDeviceId;
    const currentSettings = runtimeSettings || appSettings;

    if (!currentDeviceId || !currentSettings?.backend_url) {
      toast.error('Thiếu Device ID hoặc Backend URL để đo điểm chuẩn.');
      return;
    }

    setIsCapturingPoint(true);
    setCountdown(8);
    setStabilityStatus('waiting');

    const timer = setInterval(() => {
      setCountdown((prev) => {
        if (prev <= 1) {
          clearInterval(timer);
          setStabilityStatus('stable');
          return 0;
        }
        return prev - 1;
      });
    }, 1000);

    try {
      await new Promise((resolve) => setTimeout(resolve, 8000));
      const captureRes = await callApi(
        `/api/devices/${currentDeviceId}/calibration/ph/capture`,
        'POST',
        { point_ph: activePoint },
        currentSettings
      );
      const voltage = normalizeVoltage(captureRes);
      if (voltage === null) throw new Error('Không nhận được giá trị điện áp từ API capture');
      const confidence = normalizeConfidence(captureRes);

      setCapturedPoints((prev) => ({
        ...prev,
        [activePoint]: { voltage, confidence, capturedAt: new Date().toISOString() }
      }));
      toast.success(`Đã ghi nhận điểm pH ${activePoint}.`);
    } catch (error) {
      console.error(error);
      toast.error(`Không thể đo điểm pH ${activePoint}. Vui lòng thử lại.`);
    } finally {
      clearInterval(timer);
      setIsCapturingPoint(false);
      setCountdown(0);
      setStabilityStatus('idle');
    }
  };

  const goToNextPoint = () => {
    if (wizardStep < calibrationPoints.length - 1) {
      setWizardStep((prev) => prev + 1);
      return;
    }
    setWizardStep(calibrationPoints.length);
  };

  const calibrationSummary = (() => {
    const p7 = capturedPoints[7]?.voltage;
    const p4 = capturedPoints[4]?.voltage;
    const p10 = capturedPoints[10]?.voltage;
    const confidenceList = Object.values(capturedPoints).map((point) => point.confidence);
    const avgConfidence = confidenceList.length
      ? Math.round(confidenceList.reduce((sum, value) => sum + value, 0) / confidenceList.length)
      : 0;
    const spread = Number.isFinite(p7) && Number.isFinite(p4) ? Math.abs((p7 as number) - (p4 as number)) : 0;
    const spreadBonus = spread >= 0.2 ? 15 : spread >= 0.1 ? 8 : 0;
    const reliability = Math.max(0, Math.min(100, avgConfidence + spreadBonus));
    return {
      ph_v7: Number.isFinite(p7) ? Number((p7 as number).toFixed(3)) : null,
      ph_v4: Number.isFinite(p4) ? Number((p4 as number).toFixed(3)) : null,
      ph_v10: Number.isFinite(p10) ? Number((p10 as number).toFixed(3)) : null,
      reliability
    };
  })();

  const phaseConfigStorageKey = 'adaptive-calibration-phase-by-device';
  const effectiveDeviceId = appSettings.device_id || ctxDeviceId || '';

  useEffect(() => {
    if (!effectiveDeviceId) return;
    try {
      const raw = localStorage.getItem(phaseConfigStorageKey);
      const all = raw ? JSON.parse(raw) : {};
      const perDevice = all?.[effectiveDeviceId];
      if (perDevice && typeof perDevice === 'object') {
        setAdaptivePhases((prev) => ({
          ...prev,
          ...perDevice,
          confidence_threshold: Number(perDevice.confidence_threshold ?? prev.confidence_threshold)
        }));
      }
    } catch (error) {
      console.error('Không đọc được phase config từ localStorage', error);
    }
  }, [effectiveDeviceId]);

  const saveAdaptivePhases = (nextValue: any) => {
    setAdaptivePhases(nextValue);
    if (!effectiveDeviceId) return;
    try {
      const raw = localStorage.getItem(phaseConfigStorageKey);
      const all = raw ? JSON.parse(raw) : {};
      all[effectiveDeviceId] = nextValue;
      localStorage.setItem(phaseConfigStorageKey, JSON.stringify(all));
    } catch (error) {
      console.error('Không lưu được phase config vào localStorage', error);
    }
  };

  const hasSafetyWarningIn24h = systemEvents.some((ev: any) => {
    const ts = ev?.timestamp ? new Date(ev.timestamp).getTime() : 0;
    if (!ts || Number.isNaN(ts)) return false;
    const within24h = Date.now() - ts <= 24 * 60 * 60 * 1000;
    const level = String(ev?.level || '').toLowerCase();
    const title = String(ev?.title || '').toLowerCase();
    const category = String(ev?.category || '').toLowerCase();
    return within24h && (level === 'warning' || level === 'critical') && (category.includes('safe') || title.includes('cảnh báo') || title.includes('safety'));
  });

  const calibrationDeviation = {
    ph_v7: calibrationSummary.ph_v7 !== null ? Number((calibrationSummary.ph_v7 - Number(config.ph_v7 || 0)).toFixed(3)) : null,
    ph_v4: calibrationSummary.ph_v4 !== null ? Number((calibrationSummary.ph_v4 - Number(config.ph_v4 || 0)).toFixed(3)) : null
  };

  const applyCalibrationToConfig = (): any | null => {
    if (calibrationSummary.ph_v7 === null || calibrationSummary.ph_v4 === null) {
      toast.error('Cần đủ dữ liệu pH 7 và pH 4 để áp dụng.');
      return null;
    }
    const nextConfig = {
      ...config,
      ph_v7: calibrationSummary.ph_v7,
      ph_v4: calibrationSummary.ph_v4
    };
    setConfig(nextConfig);
    toast.success('Đã áp dụng kết quả calib vào cấu hình.');
    return nextConfig;
  };

  useEffect(() => {
    const loadConfig = async () => {
      try {
        setIsLoading(true);
        let settings: any = null;
        try {
          settings = await invoke('load_settings');
          if (settings) setAppSettings(settings);
        } catch (e) { console.error("Chưa load được store"); }

        const currentDeviceId = settings?.device_id || appSettings.device_id;
        if (!currentDeviceId) return; // Nếu chưa setup thiết bị thì bỏ qua load API

        const unifiedData = await callApi(`/api/devices/${currentDeviceId}/config/unified`, 'GET', null, settings).catch(() => null);

        if (unifiedData) {
          setConfig((prev: any) => ({
            ...prev,
            ...unifiedData.device_config,
            ...unifiedData.water_config,
            ...unifiedData.safety_config,
            ...unifiedData.sensor_calibration,
            ...unifiedData.dosing_calibration
          }));
        }
      } catch (error) {
        console.error("Lỗi khi load cấu hình:", error);
      } finally {
        setIsLoading(false);
      }
    };
    loadConfig();
  }, []);

  // Tìm và thay toàn bộ hàm handleSave trong Settings.tsx bằng phiên bản này.
  // Vị trí: khoảng dòng 305-490 trong file gốc.

  const handleSave = async (configOverride?: any) => {
    if (!appSettings.device_id || !appSettings.backend_url) {
      toast.error('Vui lòng điền đầy đủ Device ID và URL Máy chủ!');
      return;
    }

    setIsSaving(true);
    const toastId = toast.loading("Đang đồng bộ dữ liệu với máy chủ...");

    try {
      const savingConfig = configOverride || config;
      const devId = appSettings.device_id;

      // Helper đã có sẵn — đảm bảo không bao giờ trả về NaN
      const toNumberOr = (value: any, fallback: number) => {
        const parsed = Number(value);
        return Number.isFinite(parsed) ? parsed : fallback;
      };

      try { await invoke('save_settings', { apiKey: appSettings.api_key, backendUrl: appSettings.backend_url, deviceId: devId }); } catch (e) { }

      const ts = new Date().toISOString();

      // ── device_config ──────────────────────────────────────────────────────
      const devConf = {
        device_id: devId,
        control_mode: savingConfig.control_mode || 'manual',
        is_enabled: savingConfig.is_enabled ?? true,
        ec_target: toNumberOr(savingConfig.ec_target, 1.5),
        ec_tolerance: toNumberOr(savingConfig.ec_tolerance, 0.05),
        ph_target: toNumberOr(savingConfig.ph_target, 6.0),
        ph_tolerance: toNumberOr(savingConfig.ph_tolerance, 0.5),
        temp_target: toNumberOr(savingConfig.temp_target, 24.0),
        temp_tolerance: toNumberOr(savingConfig.temp_tolerance, 2.0),
        last_updated: ts,
        delay_between_a_and_b_sec: toNumberOr(savingConfig.delay_between_a_and_b_sec, 10),
      };

      // ── water_config ───────────────────────────────────────────────────────
      const waterConf = {
        device_id: devId,
        tank_height: toNumberOr(savingConfig.tank_height, 50),
        water_level_min: toNumberOr(savingConfig.water_level_min, 20.0),
        water_level_target: toNumberOr(savingConfig.water_level_target, 80.0),
        water_level_max: toNumberOr(savingConfig.water_level_max, 90.0),
        water_level_drain: toNumberOr(savingConfig.water_level_drain, 5.0),
        circulation_mode: savingConfig.circulation_mode || 'always_on',
        circulation_on_sec: toNumberOr(savingConfig.circulation_on_sec, 1800),
        circulation_off_sec: toNumberOr(savingConfig.circulation_off_sec, 900),
        water_level_tolerance: toNumberOr(savingConfig.water_level_tolerance, 5.0),
        auto_refill_enabled: savingConfig.auto_refill_enabled ?? true,
        auto_drain_overflow: savingConfig.auto_drain_overflow ?? true,
        auto_dilute_enabled: savingConfig.auto_dilute_enabled ?? false,
        dilute_drain_amount_cm: toNumberOr(savingConfig.dilute_drain_amount_cm, 5.0),
        scheduled_water_change_enabled: savingConfig.scheduled_water_change_enabled ?? false,
        water_change_cron: String(savingConfig.water_change_cron || '0 0 7 * * SUN'),
        scheduled_drain_amount_cm: toNumberOr(savingConfig.scheduled_drain_amount_cm, 10.0),
        misting_on_duration_ms: toNumberOr(savingConfig.misting_on_duration_ms, 10000),
        misting_off_duration_ms: toNumberOr(savingConfig.misting_off_duration_ms, 180000),
        last_updated: ts,
      };

      // ── safety_config ──────────────────────────────────────────────────────
      const safeConf = {
        device_id: devId,
        emergency_shutdown: savingConfig.emergency_shutdown ?? false,
        max_ec_limit: toNumberOr(savingConfig.max_ec_limit, 3.0),
        min_ec_limit: toNumberOr(savingConfig.min_ec_limit, 0.5),
        min_ph_limit: toNumberOr(savingConfig.min_ph_limit, 4.0),
        max_ph_limit: toNumberOr(savingConfig.max_ph_limit, 8.0),
        max_ec_delta: toNumberOr(savingConfig.max_ec_delta, 0.5),
        max_ph_delta: toNumberOr(savingConfig.max_ph_delta, 0.3),
        max_dose_per_cycle: toNumberOr(savingConfig.max_dose_per_cycle, 50.0),
        cooldown_sec: toNumberOr(savingConfig.cooldown_sec, 60),
        max_dose_per_hour: toNumberOr(savingConfig.max_dose_per_hour, 200.0),
        water_level_critical_min: toNumberOr(savingConfig.water_level_critical_min, 10.0),
        max_refill_cycles_per_hour: toNumberOr(savingConfig.max_refill_cycles_per_hour, 3),
        max_drain_cycles_per_hour: toNumberOr(savingConfig.max_drain_cycles_per_hour, 3),
        max_refill_duration_sec: toNumberOr(savingConfig.max_refill_duration_sec, 120),
        max_drain_duration_sec: toNumberOr(savingConfig.max_drain_duration_sec, 120),
        min_temp_limit: toNumberOr(savingConfig.min_temp_limit, 15.0),
        max_temp_limit: toNumberOr(savingConfig.max_temp_limit, 35.0),
        ec_ack_threshold: toNumberOr(savingConfig.ec_ack_threshold, 0.05),
        ph_ack_threshold: toNumberOr(savingConfig.ph_ack_threshold, 0.1),
        water_ack_threshold: toNumberOr(savingConfig.water_ack_threshold, 0.5),
        last_updated: ts,
      };

      // ── dosing_calibration ─────────────────────────────────────────────────
      const doseConf = {
        device_id: devId,
        tank_volume_l: toNumberOr(savingConfig.tank_volume_l, 50.0),
        ec_gain_per_ml: toNumberOr(savingConfig.ec_gain_per_ml, 0.1),
        ph_shift_up_per_ml: toNumberOr(savingConfig.ph_shift_up_per_ml, 0.2),
        ph_shift_down_per_ml: toNumberOr(savingConfig.ph_shift_down_per_ml, 0.2),
        active_mixing_sec: toNumberOr(savingConfig.active_mixing_sec, 5),
        sensor_stabilize_sec: toNumberOr(savingConfig.sensor_stabilize_sec, 5),
        ec_step_ratio: toNumberOr(savingConfig.ec_step_ratio, 0.4),
        ph_step_ratio: toNumberOr(savingConfig.ph_step_ratio, 0.1),
        pump_a_capacity_ml_per_sec: toNumberOr(savingConfig.pump_a_capacity_ml_per_sec, 1.2),
        pump_b_capacity_ml_per_sec: toNumberOr(savingConfig.pump_b_capacity_ml_per_sec, 1.2),
        pump_ph_up_capacity_ml_per_sec: toNumberOr(savingConfig.pump_ph_up_capacity_ml_per_sec, 1.2),
        pump_ph_down_capacity_ml_per_sec: toNumberOr(savingConfig.pump_ph_down_capacity_ml_per_sec, 1.2),
        soft_start_duration: toNumberOr(savingConfig.soft_start_duration, 3000),
        last_calibrated: ts,
        scheduled_mixing_interval_sec: toNumberOr(savingConfig.scheduled_mixing_interval_sec, 3600),
        scheduled_mixing_duration_sec: toNumberOr(savingConfig.scheduled_mixing_duration_sec, 300),
        dosing_pwm_percent: toNumberOr(savingConfig.dosing_pwm_percent, 50),
        osaka_mixing_pwm_percent: toNumberOr(savingConfig.osaka_mixing_pwm_percent, 60),
        osaka_misting_pwm_percent: toNumberOr(savingConfig.osaka_misting_pwm_percent, 100),
        scheduled_dosing_enabled: savingConfig.scheduled_dosing_enabled ?? false,
        scheduled_dosing_cron: String(savingConfig.scheduled_dosing_cron || '0 0 8 * * *'),
        scheduled_dose_a_ml: toNumberOr(savingConfig.scheduled_dose_a_ml, 10.0),
        scheduled_dose_b_ml: toNumberOr(savingConfig.scheduled_dose_b_ml, 10.0),
        ec_gain_dynamic: toNumberOr(savingConfig.ec_gain_dynamic, 0.01),
        ph_up_dynamic: toNumberOr(savingConfig.ph_up_dynamic, 0.01),
        ph_down_dynamic: toNumberOr(savingConfig.ph_down_dynamic, 0.01),
        dynamic_sample_count: Math.trunc(toNumberOr(savingConfig.dynamic_sample_count, 0)),
        dynamic_confidence: toNumberOr(savingConfig.dynamic_confidence, 0),
        // QUAN TRỌNG: Option<DateTime<Utc>> → phải là null hoặc ISO string, không phải số
        last_dynamic_update: (typeof savingConfig.last_dynamic_update === 'string' && savingConfig.last_dynamic_update)
          ? savingConfig.last_dynamic_update
          : null,
        dynamic_model_version: String(savingConfig.dynamic_model_version || 'v1'),
      };

      // ── sensor_calibration ─────────────────────────────────────────────────
      const sensConf = {
        device_id: devId,
        ph_v7: toNumberOr(savingConfig.ph_v7, 2.5),
        ph_v4: toNumberOr(savingConfig.ph_v4, 1.428),
        ec_factor: toNumberOr(savingConfig.ec_factor, 880.0),
        ec_offset: toNumberOr(savingConfig.ec_offset, 0.0),
        temp_offset: toNumberOr(savingConfig.temp_offset, 0.0),
        temp_compensation_beta: toNumberOr(savingConfig.temp_compensation_beta, 0.02),
        publish_interval: toNumberOr(savingConfig.publish_interval, 5000),
        moving_average_window: toNumberOr(savingConfig.moving_average_window, 15),
        is_ph_enabled: savingConfig.is_ph_enabled ?? true,
        is_ec_enabled: savingConfig.is_ec_enabled ?? true,
        is_temp_enabled: savingConfig.is_temp_enabled ?? true,
        is_water_level_enabled: savingConfig.is_water_level_enabled ?? true,
        last_calibrated: ts,
      };

      const unifiedPayload = {
        device_config: devConf,
        water_config: waterConf,
        safety_config: safeConf,
        sensor_calibration: sensConf,
        dosing_calibration: doseConf,
      };

      await callApi(`/api/devices/${devId}/config/unified`, 'PUT', unifiedPayload);

      toast.success('Đồng bộ cấu hình thành công!', { id: toastId });
    } catch (error: any) {
      console.error('Save error:', error);
      toast.error(`Lỗi: ${error?.message || 'Không thể kết nối máy chủ'}`, { id: toastId });
    } finally {
      setIsSaving(false);
    }
  };

  if (isLoading) return (
    <div className="flex h-screen items-center justify-center bg-slate-950 relative overflow-hidden">
      <div className="absolute inset-0 flex items-center justify-center pointer-events-none">
        <div className="w-[300px] h-[300px] border border-emerald-500/20 rounded-full animate-[ping_3s_cubic-bezier(0,0,0.2,1)_infinite]"></div>
        <div className="w-[150px] h-[150px] border border-emerald-500/40 rounded-full absolute animate-[ping_2s_cubic-bezier(0,0,0.2,1)_infinite]"></div>
      </div>
      <div className="flex flex-col items-center space-y-4 relative z-10">
        <Cpu className="text-emerald-400 animate-pulse" size={48} />
        <span className="text-emerald-500/70 font-black tracking-widest text-xs uppercase animate-pulse">Đang tải cấu hình thiết bị...</span>
      </div>
    </div>
  );

  return (
    <div className="p-4 md:p-6 space-y-6 animate-in fade-in slide-in-from-bottom-4 duration-500 pb-40 max-w-4xl mx-auto relative min-h-screen">

      <div className="absolute top-[-10%] left-[-10%] w-[50%] h-[30%] bg-indigo-500/10 rounded-full blur-[100px] pointer-events-none"></div>

      <div className="relative z-10 flex flex-col space-y-1 animate-in slide-in-from-top-4 duration-500 mb-8">
        <h1 className="text-3xl font-black flex items-center gap-3">
          <div className="p-2.5 bg-slate-800/50 backdrop-blur-md rounded-2xl border border-slate-700 shadow-[0_0_20px_rgba(148,163,184,0.15)]">
            <Settings2 size={24} className="text-slate-300" />
          </div>
          <span className="bg-clip-text text-transparent bg-gradient-to-r from-slate-100 to-slate-500 tracking-tight">
            CÀI ĐẶT HỆ THỐNG
          </span>
        </h1>
        <p className="text-xs text-slate-400 ml-[52px] font-medium tracking-wide uppercase">
          Tùy chỉnh thông số vận hành tủ điện
        </p>
      </div>

      <div className="space-y-4 relative z-10">

        {/* 0. KẾT NỐI HỆ THỐNG */}
        <AccordionSection id="network" title="Kết Nối Máy Chủ" icon={Network} color="text-slate-300" isOpen={openSection === 'network'} onToggle={() => handleToggleSection('network')}>
          <div className="space-y-3 bg-slate-900/30 p-4 rounded-2xl border border-white/5 shadow-inner">
            <InputGroup label="Mã Thiết Bị (Device ID)" type="text" value={appSettings.device_id} onChange={(e: InputEvent) => setAppSettings({ ...appSettings, device_id: e.target.value })} desc="ID định danh cấp cho tủ điện" />
            <InputGroup label="Địa chỉ Máy Chủ (Backend URL)" type="text" value={appSettings.backend_url} onChange={(e: InputEvent) => setAppSettings({ ...appSettings, backend_url: e.target.value })} desc="Ví dụ: http://192.168.1.5:8000" />
            <InputGroup label="Khóa Bảo Mật (API Key)" type="password" value={appSettings.api_key} onChange={(e: InputEvent) => setAppSettings({ ...appSettings, api_key: e.target.value })} />
          </div>
        </AccordionSection>

        {/* 1. CHẾ ĐỘ & TỔNG QUAN */}
        <AccordionSection id="general" title="Bảng Điều Khiển Chính" icon={Power} color="text-emerald-400" isOpen={openSection === 'general'} onToggle={() => handleToggleSection('general')}>
          <div className="space-y-4">
            <div className={`flex items-center justify-between p-4 rounded-2xl border transition-all duration-500 ${config.is_enabled ? 'bg-emerald-500/10 border-emerald-500/30 shadow-[0_0_20px_rgba(16,185,129,0.15)]' : 'bg-slate-900/50 border-slate-800'}`}>
              <div>
                <p className={`text-sm font-black tracking-wide ${config.is_enabled ? 'text-emerald-400' : 'text-slate-400'}`}>KÍCH HOẠT HỆ THỐNG</p>
                <p className="text-[10px] text-slate-500 font-bold uppercase mt-1">Cho phép tủ điện chạy tự động</p>
              </div>
              <Switch isOn={config.is_enabled} onClick={(val) => setConfig({ ...config, is_enabled: val })} colorClass="bg-emerald-500 shadow-[0_0_10px_rgba(16,185,129,0.5)]" />
            </div>

            <div className={`flex items-center justify-between p-4 rounded-2xl border transition-all duration-500 ${config.emergency_shutdown ? 'bg-rose-500/20 border-rose-500/50 shadow-[0_0_30px_rgba(244,63,94,0.3)] animate-pulse' : 'bg-slate-900/50 border-slate-800'}`}>
              <div className="flex items-start gap-3">
                <ShieldAlert className={config.emergency_shutdown ? 'text-rose-400' : 'text-slate-600'} size={20} />
                <div>
                  <p className={`text-sm font-black tracking-wide ${config.emergency_shutdown ? 'text-rose-400' : 'text-slate-400'}`}>DỪNG KHẨN CẤP (E-STOP)</p>
                  <p className="text-[10px] text-slate-500 font-bold uppercase mt-1">Ngắt ngay lập tức mọi thiết bị điện</p>
                </div>
              </div>
              <Switch isOn={config.emergency_shutdown} onClick={(val) => setConfig({ ...config, emergency_shutdown: val })} colorClass="bg-rose-500 shadow-[0_0_15px_rgba(244,63,94,0.6)]" />
            </div>

            <div className="p-4 bg-slate-900/40 rounded-2xl border border-white/5 backdrop-blur-md">
              <label className="text-[10px] font-black text-slate-500 tracking-widest uppercase mb-3 flex items-center gap-2">
                <Zap size={12} className="text-amber-500" /> Chế độ vận hành
              </label>
              <div className="flex space-x-2 bg-slate-950/50 p-1.5 rounded-xl border border-slate-800/50 shadow-inner">
                <button
                  onClick={() => setConfig({ ...config, control_mode: 'auto' })}
                  className={`flex-1 py-3 rounded-lg text-xs font-black tracking-widest transition-all duration-300 ${config.control_mode === 'auto'
                    ? 'bg-emerald-500 text-slate-950 shadow-[0_0_15px_rgba(16,185,129,0.5)] scale-[1.02]'
                    : 'bg-transparent text-slate-500 hover:text-slate-300 hover:bg-slate-800/50'
                    }`}
                >
                  TỰ ĐỘNG
                </button>
                <button
                  onClick={() => setConfig({ ...config, control_mode: 'manual' })}
                  className={`flex-1 py-3 rounded-lg text-xs font-black tracking-widest transition-all duration-300 ${config.control_mode === 'manual'
                    ? 'bg-orange-500 text-slate-950 shadow-[0_0_15px_rgba(249,115,22,0.5)] scale-[1.02]'
                    : 'bg-transparent text-slate-500 hover:text-slate-300 hover:bg-slate-800/50'
                    }`}
                >
                  THỦ CÔNG
                </button>
              </div>
            </div>
          </div>
        </AccordionSection>

        {/* 2. MỤC TIÊU SINH TRƯỞNG */}
        <AccordionSection id="growth" title="Môi Trường & Mục Tiêu" icon={Target} color="text-blue-400" isOpen={openSection === 'growth'} onToggle={() => handleToggleSection('growth')}>
          <SubCard title="Dinh Dưỡng (EC)">
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
              <InputGroup label="Mức EC mong muốn" step="0.1" value={config.ec_target} onChange={(e: InputEvent) => setConfig({ ...config, ec_target: e.target.value })} />
              <InputGroup label="Khoảng dao động cho phép (±)" step="0.05" value={config.ec_tolerance} onChange={(e: InputEvent) => setConfig({ ...config, ec_tolerance: e.target.value })} desc="Máy sẽ bù phân khi EC tụt quá mức này." />
            </div>
          </SubCard>

          <SubCard title="Độ pH" className="mt-4">
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
              <InputGroup label="Mức pH mong muốn" step="0.1" value={config.ph_target} onChange={(e: InputEvent) => setConfig({ ...config, ph_target: e.target.value })} />
              <InputGroup label="Khoảng dao động cho phép (±)" step="0.05" value={config.ph_tolerance} onChange={(e: InputEvent) => setConfig({ ...config, ph_tolerance: e.target.value })} />
            </div>
          </SubCard>

          <SubCard title="Nhiệt Độ Trồng & Phun Sương Không Khí" className="mt-4 border-blue-500/20 bg-gradient-to-br from-blue-500/5 to-transparent">
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
              <InputGroup label="Nhiệt độ phòng tối ưu (°C)" step="0.5" value={config.temp_target} onChange={(e: InputEvent) => setConfig({ ...config, temp_target: e.target.value })} />
              <InputGroup label="Dung sai nhiệt độ (°C)" step="0.5" value={config.temp_tolerance} onChange={(e: InputEvent) => setConfig({ ...config, temp_tolerance: e.target.value })} />

              <div className="sm:col-span-2 mt-2">
                <InputGroup label="Nhiệt độ kích hoạt phun sương tăng cường (°C)" step="0.5" value={config.misting_temp_threshold} onChange={(e: InputEvent) => setConfig({ ...config, misting_temp_threshold: e.target.value })} desc="Nếu trời nóng vượt mức này, bơm sương sẽ chạy nhịp làm mát nhanh." />
              </div>

              <div className="sm:col-span-2 pt-4 pb-1"><span className="text-[9px] text-blue-400 font-black uppercase tracking-widest bg-blue-500/10 border border-blue-500/20 py-1.5 px-3 rounded-lg shadow-inner">Nhịp phun cơ bản (Trời mát)</span></div>
              <InputGroup label="Thời gian Phun (ms)" step="1000" value={config.misting_on_duration_ms} onChange={(e: InputEvent) => setConfig({ ...config, misting_on_duration_ms: e.target.value })} desc="1000ms = 1 Giây" />
              <InputGroup label="Thời gian Nghỉ (ms)" step="1000" value={config.misting_off_duration_ms} onChange={(e: InputEvent) => setConfig({ ...config, misting_off_duration_ms: e.target.value })} />

              <div className="sm:col-span-2 pt-4 pb-1 border-t border-slate-800"><span className="text-[9px] text-rose-400 font-black uppercase tracking-widest bg-rose-500/10 border border-rose-500/20 py-1.5 px-3 rounded-lg shadow-inner">Nhịp làm mát nhanh (Trời nóng)</span></div>
              <InputGroup label="Thời gian Phun (ms)" step="1000" value={config.high_temp_misting_on_duration_ms} onChange={(e: InputEvent) => setConfig({ ...config, high_temp_misting_on_duration_ms: e.target.value })} />
              <InputGroup label="Thời gian Nghỉ (ms)" step="1000" value={config.high_temp_misting_off_duration_ms} onChange={(e: InputEvent) => setConfig({ ...config, high_temp_misting_off_duration_ms: e.target.value })} />
            </div>
          </SubCard>
        </AccordionSection>

        {/* 3. CẤU HÌNH NƯỚC */}
        <AccordionSection id="water" title="Quản Lý Bơm Nước" icon={Waves} color="text-cyan-400" isOpen={openSection === 'water'} onToggle={() => handleToggleSection('water')}>
          <SubCard title="Đo Mực Nước Bằng Siêu Âm">
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
              <div className="sm:col-span-2">
                <InputGroup label="Chiều cao từ cảm biến đến đáy bồn (cm)" value={config.tank_height} onChange={(e: InputEvent) => setConfig({ ...config, tank_height: e.target.value })} />
              </div>
              <InputGroup label="Mức nước muốn giữ (%)" value={config.water_level_target} onChange={(e: InputEvent) => setConfig({ ...config, water_level_target: e.target.value })} />
              <InputGroup label="Dung sai bơm bù (%)" value={config.water_level_tolerance} onChange={(e: InputEvent) => setConfig({ ...config, water_level_tolerance: e.target.value })} />
              <InputGroup label="Mức báo động cạn (%)" value={config.water_level_min} onChange={(e: InputEvent) => setConfig({ ...config, water_level_min: e.target.value })} />
              <InputGroup label="Mức báo động tràn (%)" value={config.water_level_max} onChange={(e: InputEvent) => setConfig({ ...config, water_level_max: e.target.value })} />
              <div className="sm:col-span-2">
                <InputGroup label="Mức nước khi xả cạn bồn (%)" value={config.water_level_drain} onChange={(e: InputEvent) => setConfig({ ...config, water_level_drain: e.target.value })} />
              </div>
            </div>
          </SubCard>

          <SubCard title="Bơm Tuần Hoàn / Sục Khí" className="mt-4">
            <div className="space-y-4">
              <div>
                <label className="text-xs font-bold text-slate-300 tracking-wide uppercase mb-2 block">
                  Chế độ chạy máy bơm sục khí
                </label>
                <select
                  className="w-full bg-slate-900 border border-slate-700 text-slate-300 text-sm rounded-xl p-3 focus:ring-2 focus:ring-cyan-500 outline-none transition-all hover:border-slate-600"
                  value={config.circulation_mode}
                  onChange={(e: InputEvent) => setConfig({ ...config, circulation_mode: e.target.value })}
                >
                  <option value="always_on">Chạy liên tục 24/7</option>
                  <option value="timer">Chạy theo chu kỳ (Bật / Tắt)</option>
                  <option value="off">Tắt hoàn toàn</option>
                </select>
              </div>
              {config.circulation_mode === 'timer' && (
                <div className="grid grid-cols-1 sm:grid-cols-2 gap-4 animate-in fade-in slide-in-from-top-2">
                  <InputGroup label="Bật sục khí (Giây)" value={config.circulation_on_sec} onChange={(e: InputEvent) => setConfig({ ...config, circulation_on_sec: e.target.value })} />
                  <InputGroup label="Tắt sục khí (Giây)" value={config.circulation_off_sec} onChange={(e: InputEvent) => setConfig({ ...config, circulation_off_sec: e.target.value })} />
                </div>
              )}
            </div>
          </SubCard>

          <SubCard title="Van Cấp / Xả Tự Động" className="mt-4 bg-slate-900/30">
            <div className="space-y-5">
              <div className="flex items-center justify-between">
                <span className="text-xs font-bold text-slate-300 tracking-wide uppercase">Tự động bơm thêm nước ngầm</span>
                <Switch isOn={config.auto_refill_enabled} onClick={(val) => setConfig({ ...config, auto_refill_enabled: val })} colorClass="bg-cyan-500 shadow-[0_0_10px_rgba(6,182,212,0.4)]" />
              </div>
              <div className="flex items-center justify-between">
                <span className="text-xs font-bold text-slate-300 tracking-wide uppercase">Tự động xả nếu bồn đầy tràn</span>
                <Switch isOn={config.auto_drain_overflow} onClick={(val) => setConfig({ ...config, auto_drain_overflow: val })} colorClass="bg-cyan-500 shadow-[0_0_10px_rgba(6,182,212,0.4)]" />
              </div>

              <div className="pt-4 border-t border-slate-800/50">
                <div className="flex items-center justify-between">
                  <div>
                    <span className="text-xs font-bold text-slate-300 tracking-wide uppercase">Tự xả pha loãng khi quá liều EC</span>
                    <p className="text-[10px] text-slate-500 mt-1">Xả bớt dung dịch đậm đặc để tự bơm nước lã vào</p>
                  </div>
                  <Switch isOn={config.auto_dilute_enabled} onClick={(val) => setConfig({ ...config, auto_dilute_enabled: val })} colorClass="bg-cyan-500 shadow-[0_0_10px_rgba(6,182,212,0.4)]" />
                </div>
                {config.auto_dilute_enabled && (
                  <div className="mt-4 pl-4 border-l-2 border-cyan-500/50 animate-in fade-in slide-in-from-left-2">
                    <InputGroup label="Mức nước sẽ xả đi (cm)" step="0.5" value={config.dilute_drain_amount_cm} onChange={(e: InputEvent) => setConfig({ ...config, dilute_drain_amount_cm: e.target.value })} />
                  </div>
                )}
              </div>
            </div>
          </SubCard>

          <SubCard title="Thay Nước Định Kỳ" className="mt-4">
            <div className="flex items-center justify-between mb-4">
              <span className="text-xs font-bold text-slate-300 tracking-wide uppercase">Bật lịch tự động xả nước cũ</span>
              <Switch isOn={config.scheduled_water_change_enabled} onClick={(val) => setConfig({ ...config, scheduled_water_change_enabled: val })} colorClass="bg-cyan-500 shadow-[0_0_10px_rgba(6,182,212,0.4)]" />
            </div>
            {config.scheduled_water_change_enabled && (
              <div className="grid grid-cols-1 gap-4 animate-in fade-in slide-in-from-top-2 bg-slate-900/50 p-4 rounded-xl border border-white/5 shadow-inner">
                {/* Khu vực Nhập Cron thông minh */}
                <div className="space-y-3 border border-slate-700/50 p-3 rounded-lg bg-slate-950/50">
                  <InputGroup type="text" label="Giờ thay nước (Chuỗi Cron)" value={config.water_change_cron} onChange={(e: InputEvent) => setConfig({ ...config, water_change_cron: e.target.value })} desc="Cú pháp: Phút Giờ Ngày Tháng Thứ" />

                  <div>
                    <span className="text-[10px] font-bold text-slate-500 uppercase flex items-center gap-1 mb-2"><Clock size={10} /> Chọn nhanh lịch:</span>
                    <div className="flex flex-wrap gap-2">
                      <button onClick={() => setConfig({ ...config, water_change_cron: "0 0 7 * * SUN" })} className="px-3 py-1.5 bg-slate-800 hover:bg-cyan-600 hover:text-white text-[11px] text-slate-300 rounded-md transition-colors border border-slate-700">7h Sáng Chủ Nhật</button>
                      <button onClick={() => setConfig({ ...config, water_change_cron: "0 0 6 1,15 * *" })} className="px-3 py-1.5 bg-slate-800 hover:bg-cyan-600 hover:text-white text-[11px] text-slate-300 rounded-md transition-colors border border-slate-700">Ngày 1 và 15 (6h Sáng)</button>
                      <button onClick={() => setConfig({ ...config, water_change_cron: "0 0 8 * * *" })} className="px-3 py-1.5 bg-slate-800 hover:bg-cyan-600 hover:text-white text-[11px] text-slate-300 rounded-md transition-colors border border-slate-700">8h Sáng mỗi ngày</button>
                    </div>
                  </div>
                </div>

                <InputGroup label="Lượng xả đi mỗi lần (cm)" value={config.scheduled_drain_amount_cm} onChange={(e: InputEvent) => setConfig({ ...config, scheduled_drain_amount_cm: e.target.value })} />
              </div>
            )}
          </SubCard>
        </AccordionSection>

        {/* 4. ĐỊNH LƯỢNG */}
        <AccordionSection id="dosing" title="Máy Pha Phân & Hóa Chất" icon={FlaskConical} color="text-fuchsia-400" isOpen={openSection === 'dosing'} onToggle={() => handleToggleSection('dosing')}>
          <SubCard title="Công Suất Bơm Vi Lượng (Chống giật tia)">
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
              <InputGroup label="Tốc độ Bơm Phân (%)" step="1" value={config.dosing_pwm_percent} onChange={(e: InputEvent) => setConfig({ ...config, dosing_pwm_percent: e.target.value })} desc="Giảm tốc độ để châm phân từ từ, chính xác hơn." />
              <InputGroup label="Tốc độ Bơm Trộn (%)" step="1" value={config.osaka_mixing_pwm_percent} onChange={(e: InputEvent) => setConfig({ ...config, osaka_mixing_pwm_percent: e.target.value })} />
              <InputGroup label="Tốc độ Bơm Phun Sương (%)" step="1" value={config.osaka_misting_pwm_percent} onChange={(e: InputEvent) => setConfig({ ...config, osaka_misting_pwm_percent: e.target.value })} />
              <InputGroup label="Độ trễ khởi động bơm (ms)" step="100" value={config.soft_start_duration} onChange={(e: InputEvent) => setConfig({ ...config, soft_start_duration: e.target.value })} desc="Bảo vệ nguồn điện tử, tránh sụt áp đột ngột." />
            </div>
          </SubCard>

          <SubCard title="Châm Phân Bổ Sung Theo Giờ" className="mt-4">
            <div className="flex items-center justify-between mb-4">
              <span className="text-xs font-bold text-slate-300 tracking-wide uppercase">Bật lịch châm cứng</span>
              <Switch isOn={config.scheduled_dosing_enabled} onClick={(val) => setConfig({ ...config, scheduled_dosing_enabled: val })} colorClass="bg-fuchsia-500 shadow-[0_0_10px_rgba(217,70,239,0.4)]" />
            </div>
            {config.scheduled_dosing_enabled && (
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-4 animate-in fade-in slide-in-from-top-2 bg-slate-900/50 p-4 rounded-xl border border-white/5 shadow-inner">
                {/* Khu vực Nhập Cron thông minh */}
                <div className="sm:col-span-2 space-y-3 border border-slate-700/50 p-3 rounded-lg bg-slate-950/50">
                  <InputGroup type="text" label="Lịch trình bơm (Chuỗi Cron)" value={config.scheduled_dosing_cron} onChange={(e: InputEvent) => setConfig({ ...config, scheduled_dosing_cron: e.target.value })} desc="Cú pháp: Phút Giờ Ngày Tháng Thứ" />

                  <div>
                    <span className="text-[10px] font-bold text-slate-500 uppercase flex items-center gap-1 mb-2"><Clock size={10} /> Chọn nhanh lịch:</span>
                    <div className="flex flex-wrap gap-2">
                      <button onClick={() => setConfig({ ...config, scheduled_dosing_cron: "0 0 6 * * *" })} className="px-3 py-1.5 bg-slate-800 hover:bg-fuchsia-600 hover:text-white text-[11px] text-slate-300 rounded-md transition-colors border border-slate-700">6h Sáng mỗi ngày</button>
                      <button onClick={() => setConfig({ ...config, scheduled_dosing_cron: "0 0 8,16 * * *" })} className="px-3 py-1.5 bg-slate-800 hover:bg-fuchsia-600 hover:text-white text-[11px] text-slate-300 rounded-md transition-colors border border-slate-700">8h Sáng & 4h Chiều</button>
                      <button onClick={() => setConfig({ ...config, scheduled_dosing_cron: "0 0 7 * * SUN" })} className="px-3 py-1.5 bg-slate-800 hover:bg-fuchsia-600 hover:text-white text-[11px] text-slate-300 rounded-md transition-colors border border-slate-700">7h Sáng Chủ Nhật</button>
                    </div>
                  </div>
                </div>

                <InputGroup label="Lượng Bơm A (ml)" step="0.5" value={config.scheduled_dose_a_ml} onChange={(e: InputEvent) => setConfig({ ...config, scheduled_dose_a_ml: e.target.value })} />
                <InputGroup label="Lượng Bơm B (ml)" step="0.5" value={config.scheduled_dose_b_ml} onChange={(e: InputEvent) => setConfig({ ...config, scheduled_dose_b_ml: e.target.value })} />
              </div>
            )}
          </SubCard>

          <SubCard title="Cấu Hình Đảo Trộn Nước" className="mt-4">
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
              <InputGroup label="Bao lâu đảo 1 lần (Giây)" step="60" value={config.scheduled_mixing_interval_sec} onChange={(e: InputEvent) => setConfig({ ...config, scheduled_mixing_interval_sec: e.target.value })} />
              <InputGroup label="Đảo trong bao lâu (Giây)" step="10" value={config.scheduled_mixing_duration_sec} onChange={(e: InputEvent) => setConfig({ ...config, scheduled_mixing_duration_sec: e.target.value })} />
              <div className="sm:col-span-2">
                <InputGroup label="Đảo ngay sau khi châm phân (Giây)" step="1" value={config.active_mixing_sec} onChange={(e: InputEvent) => setConfig({ ...config, active_mixing_sec: e.target.value })} />
              </div>
              <div className="sm:col-span-2">
                <InputGroup label="Thời gian chờ cảm biến ổn định số liệu (Giây)" step="1" value={config.sensor_stabilize_sec} onChange={(e: InputEvent) => setConfig({ ...config, sensor_stabilize_sec: e.target.value })} desc="Tạm dừng đọc số sau khi trộn để tránh bị nhiễu do nước xáo trộn." />
              </div>
            </div>
          </SubCard>

          <SubCard title="Thuật Toán Châm & Lưu Lượng Thực Tế" className="mt-4">
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
              <InputGroup label="Thể tích bồn chứa (Lít)" value={config.tank_volume_l} onChange={(e: InputEvent) => setConfig({ ...config, tank_volume_l: e.target.value })} />
              <InputGroup label="Thời gian chờ giữa Bơm A và B (Giây)" step="1" value={config.delay_between_a_and_b_sec} onChange={(e: InputEvent) => setConfig({ ...config, delay_between_a_and_b_sec: e.target.value })} desc="Chống kết tủa Canxi và Photpho" />

              <div className="sm:col-span-2 pt-4 pb-1 border-b border-slate-800"><span className="text-xs text-fuchsia-400 font-bold uppercase tracking-widest">Đo lường đầu dò bơm</span></div>
              <InputGroup label="Lưu lượng Bơm A (ml/giây)" step="0.1" value={config.pump_a_capacity_ml_per_sec} onChange={(e: InputEvent) => setConfig({ ...config, pump_a_capacity_ml_per_sec: e.target.value })} />
              <InputGroup label="Lưu lượng Bơm B (ml/giây)" step="0.1" value={config.pump_b_capacity_ml_per_sec} onChange={(e: InputEvent) => setConfig({ ...config, pump_b_capacity_ml_per_sec: e.target.value })} />
              <InputGroup label="Lưu lượng Bơm pH Tăng (ml/giây)" step="0.1" value={config.pump_ph_up_capacity_ml_per_sec} onChange={(e: InputEvent) => setConfig({ ...config, pump_ph_up_capacity_ml_per_sec: e.target.value })} />
              <InputGroup label="Lưu lượng Bơm pH Giảm (ml/giây)" step="0.1" value={config.pump_ph_down_capacity_ml_per_sec} onChange={(e: InputEvent) => setConfig({ ...config, pump_ph_down_capacity_ml_per_sec: e.target.value })} />

              <div className="sm:col-span-2 pt-4 pb-1 border-b border-slate-800"><span className="text-xs text-fuchsia-400 font-bold uppercase tracking-widest">Độ đậm đặc của dung dịch</span></div>
              <InputGroup label="Mức tăng EC khi châm 1ml" step="0.01" value={config.ec_gain_per_ml} onChange={(e: InputEvent) => setConfig({ ...config, ec_gain_per_ml: e.target.value })} />
              <InputGroup label="Hệ số rải phân EC (0-1)" step="0.1" value={config.ec_step_ratio} onChange={(e: InputEvent) => setConfig({ ...config, ec_step_ratio: e.target.value })} desc="Càng nhỏ máy sẽ châm càng từ từ, không bị quá tay." />
              <InputGroup label="Mức tăng pH khi châm 1ml" step="0.01" value={config.ph_shift_up_per_ml} onChange={(e: InputEvent) => setConfig({ ...config, ph_shift_up_per_ml: e.target.value })} />
              <InputGroup label="Mức giảm pH khi châm 1ml" step="0.01" value={config.ph_shift_down_per_ml} onChange={(e: InputEvent) => setConfig({ ...config, ph_shift_down_per_ml: e.target.value })} />
              <div className="sm:col-span-2">
                <InputGroup label="Hệ số rải hóa chất pH (0-1)" step="0.1" value={config.ph_step_ratio} onChange={(e: InputEvent) => setConfig({ ...config, ph_step_ratio: e.target.value })} />
              </div>
            </div>
          </SubCard>
        </AccordionSection>

        {/* 5. AN TOÀN */}
        <AccordionSection id="safety" title="Bảo Vệ Chống Chập/Hư Máy" icon={ShieldAlert} color="text-amber-400" isOpen={openSection === 'safety'} onToggle={() => handleToggleSection('safety')}>
          <SubCard title="Giới Hạn Báo Động (Còi Hú)" className="border-rose-500/20 bg-rose-500/5">
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
              <InputGroup label="Nhiệt độ báo động Lạnh (°C)" step="0.1" value={config.min_temp_limit} onChange={(e: InputEvent) => setConfig({ ...config, min_temp_limit: e.target.value })} />
              <InputGroup label="Nhiệt độ báo động Nóng (°C)" step="0.1" value={config.max_temp_limit} onChange={(e: InputEvent) => setConfig({ ...config, max_temp_limit: e.target.value })} />
              <InputGroup label="EC quá loãng (Báo động)" step="0.1" value={config.min_ec_limit} onChange={(e: InputEvent) => setConfig({ ...config, min_ec_limit: e.target.value })} />
              <InputGroup label="EC quá đặc (Báo động)" step="0.1" value={config.max_ec_limit} onChange={(e: InputEvent) => setConfig({ ...config, max_ec_limit: e.target.value })} />
              <InputGroup label="pH quá thấp (Báo động)" step="0.1" value={config.min_ph_limit} onChange={(e: InputEvent) => setConfig({ ...config, min_ph_limit: e.target.value })} />
              <InputGroup label="pH quá cao (Báo động)" step="0.1" value={config.max_ph_limit} onChange={(e: InputEvent) => setConfig({ ...config, max_ph_limit: e.target.value })} />
              <div className="sm:col-span-2">
                <InputGroup label="Báo động hụt nước bồn (cm)" value={config.water_level_critical_min} onChange={(e: InputEvent) => setConfig({ ...config, water_level_critical_min: e.target.value })} desc="Tủ sẽ tự động ngắt điện mọi rơ-le để bảo vệ chống cháy máy bơm." />
              </div>
            </div>
          </SubCard>

          <SubCard title="Ngăn Bơm Chạy Quá Sức" className="mt-4">
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
              <InputGroup label="Giới hạn ml bơm mỗi lần" value={config.max_dose_per_cycle} onChange={(e: InputEvent) => setConfig({ ...config, max_dose_per_cycle: e.target.value })} />
              <InputGroup label="Giới hạn ml bơm trong 1 giờ" value={config.max_dose_per_hour} onChange={(e: InputEvent) => setConfig({ ...config, max_dose_per_hour: e.target.value })} />
              <div className="sm:col-span-2">
                <InputGroup label="Thời gian nghỉ để tản nhiệt bơm (Giây)" value={config.cooldown_sec} onChange={(e: InputEvent) => setConfig({ ...config, cooldown_sec: e.target.value })} />
              </div>

              <div className="sm:col-span-2 pt-4 pb-1 border-t border-slate-800"><span className="text-xs text-amber-400 font-bold uppercase tracking-widest">Bộ lọc sốc tín hiệu (Chống nhiễu)</span></div>
              <InputGroup label="Bỏ qua nhiễu EC nếu nhảy đột ngột (Δ)" step="0.1" value={config.max_ec_delta} onChange={(e: InputEvent) => setConfig({ ...config, max_ec_delta: e.target.value })} />
              <InputGroup label="Bỏ qua nhiễu pH nếu nhảy đột ngột (Δ)" step="0.1" value={config.max_ph_delta} onChange={(e: InputEvent) => setConfig({ ...config, max_ph_delta: e.target.value })} />
              <InputGroup label="Chênh lệch EC tối thiểu để bắt đầu châm" step="0.01" value={config.ec_ack_threshold} onChange={(e: InputEvent) => setConfig({ ...config, ec_ack_threshold: e.target.value })} />
              <InputGroup label="Chênh lệch pH tối thiểu để bắt đầu châm" step="0.01" value={config.ph_ack_threshold} onChange={(e: InputEvent) => setConfig({ ...config, ph_ack_threshold: e.target.value })} />
              <div className="sm:col-span-2">
                <InputGroup label="Chênh lệch Nước (%) tối thiểu để kích hoạt bơm" step="0.1" value={config.water_ack_threshold} onChange={(e: InputEvent) => setConfig({ ...config, water_ack_threshold: e.target.value })} />
              </div>
            </div>
          </SubCard>

          <SubCard title="Bảo Vệ Máy Bơm Nước Chống Cháy" className="mt-4">
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
              <InputGroup label="Số lần bơm nước lã tối đa / Giờ" value={config.max_refill_cycles_per_hour} onChange={(e: InputEvent) => setConfig({ ...config, max_refill_cycles_per_hour: e.target.value })} />
              <InputGroup label="Thời gian chạy bơm rốn tối đa (Giây)" value={config.max_refill_duration_sec} onChange={(e: InputEvent) => setConfig({ ...config, max_refill_duration_sec: e.target.value })} desc="Ngắt máy bơm nước lên nếu quá thời gian (chống kẹt hụt nước ngầm)" />
              <InputGroup label="Số lần xả cặn tối đa / Giờ" value={config.max_drain_cycles_per_hour} onChange={(e: InputEvent) => setConfig({ ...config, max_drain_cycles_per_hour: e.target.value })} />
              <InputGroup label="Thời gian chạy van xả tối đa (Giây)" value={config.max_drain_duration_sec} onChange={(e: InputEvent) => setConfig({ ...config, max_drain_duration_sec: e.target.value })} />
            </div>
          </SubCard>
        </AccordionSection>

        {/* 6. HIỆU CHUẨN ĐẦU DÒ */}
        <AccordionSection id="sensor" title="Cảm Biến & Cân Chỉnh (Calib)" icon={Activity} color="text-indigo-400" isOpen={openSection === 'sensor'} onToggle={() => handleToggleSection('sensor')}>
          <SubCard title="Truyền Thông App & Hiển Thị" className="mb-4">
            <div className="space-y-5">
              <div>
                <label className="text-xs font-bold text-slate-300 tracking-wide uppercase mb-2 block">
                  Bao lâu cập nhật số liệu lên App 1 lần?
                </label>
                <select
                  className="w-full bg-slate-900 border border-slate-700 text-slate-300 text-sm rounded-xl p-3 focus:ring-2 focus:ring-indigo-500 outline-none transition-all hover:border-slate-600"
                  value={config.publish_interval}
                  onChange={(e: InputEvent) => setConfig({ ...config, publish_interval: parseInt(e.target.value) })}
                >
                  <option value={1000}>Tức thời (1 Giây / Lần)</option>
                  <option value={5000}>Bình thường (5 Giây / Lần)</option>
                  <option value={10000}>Tiết kiệm (10 Giây / Lần)</option>
                  <option value={60000}>Ít dùng (60 Giây / Lần)</option>
                </select>
              </div>

              <div>
                <label className="text-xs font-bold text-slate-300 tracking-wide uppercase mb-2 block">
                  Mức độ làm mượt đường đồ thị (Lọc nhiễu cảm biến)
                </label>
                <div className="flex space-x-2 bg-slate-950/50 p-1.5 rounded-xl border border-slate-800/50 shadow-inner">
                  <button
                    onClick={() => setConfig({ ...config, moving_average_window: 5 })}
                    className={`flex-1 py-2.5 rounded-lg text-xs font-black tracking-widest transition-all ${config.moving_average_window <= 5 ? 'bg-indigo-500 text-white shadow-[0_0_15px_rgba(99,102,241,0.5)]' : 'text-slate-500 hover:bg-slate-800/50 hover:text-slate-300'}`}
                  >
                    NHANH (Dễ giật)
                  </button>
                  <button
                    onClick={() => setConfig({ ...config, moving_average_window: 15 })}
                    className={`flex-1 py-2.5 rounded-lg text-xs font-black tracking-widest transition-all ${config.moving_average_window > 5 && config.moving_average_window <= 20 ? 'bg-indigo-500 text-white shadow-[0_0_15px_rgba(99,102,241,0.5)]' : 'text-slate-500 hover:bg-slate-800/50 hover:text-slate-300'}`}
                  >
                    CÂN BẰNG
                  </button>
                  <button
                    onClick={() => setConfig({ ...config, moving_average_window: 50 })}
                    className={`flex-1 py-2.5 rounded-lg text-xs font-black tracking-widest transition-all ${config.moving_average_window > 20 ? 'bg-indigo-500 text-white shadow-[0_0_15px_rgba(99,102,241,0.5)]' : 'text-slate-500 hover:bg-slate-800/50 hover:text-slate-300'}`}
                  >
                    MƯỢT (Trễ số)
                  </button>
                </div>
              </div>
            </div>
          </SubCard>

          <SubCard title="Đọc Tín Hiệu Các Đầu Dò" className="mb-4 bg-indigo-900/20 border-indigo-500/20">
            <div className="space-y-4">
              <div className="flex items-center justify-between">
                <span className="text-xs font-bold uppercase tracking-widest text-slate-300">Nhận tín hiệu pH</span>
                <Switch isOn={config.is_ph_enabled} onClick={(val) => setConfig({ ...config, is_ph_enabled: val })} colorClass="bg-indigo-500 shadow-[0_0_10px_rgba(99,102,241,0.5)]" />
              </div>
              <div className="flex items-center justify-between">
                <span className="text-xs font-bold uppercase tracking-widest text-slate-300">Nhận tín hiệu Dinh dưỡng (EC)</span>
                <Switch isOn={config.is_ec_enabled} onClick={(val) => setConfig({ ...config, is_ec_enabled: val })} colorClass="bg-indigo-500 shadow-[0_0_10px_rgba(99,102,241,0.5)]" />
              </div>
              <div className="flex items-center justify-between">
                <span className="text-xs font-bold uppercase tracking-widest text-slate-300">Nhận tín hiệu Nhiệt độ nước</span>
                <Switch isOn={config.is_temp_enabled} onClick={(val) => setConfig({ ...config, is_temp_enabled: val })} colorClass="bg-indigo-500 shadow-[0_0_10px_rgba(99,102,241,0.5)]" />
              </div>
              <div className="flex items-center justify-between">
                <span className="text-xs font-bold uppercase tracking-widest text-slate-300">Nhận tín hiệu Radar Mực nước</span>
                <Switch isOn={config.is_water_level_enabled} onClick={(val) => setConfig({ ...config, is_water_level_enabled: val })} colorClass="bg-indigo-500 shadow-[0_0_10px_rgba(99,102,241,0.5)]" />
              </div>
            </div>
          </SubCard>

          <SubCard title="Wizard cân chỉnh đầu dò pH">
            <div className="space-y-4">
              <div className="px-5 py-4 bg-indigo-500/10 border border-indigo-500/30 rounded-2xl flex items-start space-x-3 shadow-inner">
                <FlaskConical size={20} className="text-indigo-400 flex-shrink-0 animate-pulse" />
                <p className="text-xs text-indigo-200 leading-relaxed font-medium">
                  Chuẩn bị dung dịch chuẩn sạch, rửa đầu dò bằng nước cất trước mỗi lần đo. Wizard sẽ tự gọi API <b>capture</b> ở từng điểm, không cần nhập điện áp thủ công.
                </p>
              </div>

              <div className="flex flex-wrap items-center gap-2 bg-slate-900/40 border border-slate-700 rounded-xl p-2">
                <button
                  onClick={() => {
                    setCalibrationPointsCount(2);
                    setWizardStep(0);
                    setCapturedPoints({});
                  }}
                  className={`px-3 py-2 rounded-lg text-xs font-black tracking-widest transition-all ${calibrationPointsCount === 2 ? 'bg-indigo-500 text-white shadow-[0_0_15px_rgba(99,102,241,0.5)]' : 'text-slate-400 hover:bg-slate-800'}`}
                >
                  2 ĐIỂM (pH 7, pH 4)
                </button>
                <button
                  onClick={() => {
                    setCalibrationPointsCount(3);
                    setWizardStep(0);
                    setCapturedPoints({});
                  }}
                  className={`px-3 py-2 rounded-lg text-xs font-black tracking-widest transition-all ${calibrationPointsCount === 3 ? 'bg-indigo-500 text-white shadow-[0_0_15px_rgba(99,102,241,0.5)]' : 'text-slate-400 hover:bg-slate-800'}`}
                >
                  3 ĐIỂM (pH 7, pH 4, pH 10)
                </button>
              </div>

              <div className="rounded-xl border border-cyan-500/20 bg-cyan-500/5 p-4 space-y-3">
                <p className="text-xs font-black uppercase tracking-widest text-cyan-300">Pha triển khai hệ số (theo device_id)</p>
                <div className="space-y-2">
                  <div className="flex items-center justify-between">
                    <span className="text-xs text-slate-200">Pha 1 - Observe (thu mẫu + tính hệ số đề xuất)</span>
                    <Switch isOn={adaptivePhases.observe} onClick={(val) => saveAdaptivePhases({ ...adaptivePhases, observe: val })} colorClass="bg-cyan-500" />
                  </div>
                  <div className="flex items-center justify-between">
                    <span className="text-xs text-slate-200">Pha 2 - Recommend (so sánh hệ số cũ/mới + xác nhận tay)</span>
                    <Switch isOn={adaptivePhases.recommend} onClick={(val) => saveAdaptivePhases({ ...adaptivePhases, recommend: val })} colorClass="bg-cyan-500" />
                  </div>
                  <div className="flex items-center justify-between">
                    <span className="text-xs text-slate-200">Pha 3 - Auto Apply (tự áp dụng nếu đủ điều kiện)</span>
                    <Switch isOn={adaptivePhases.auto_apply} onClick={(val) => saveAdaptivePhases({ ...adaptivePhases, auto_apply: val })} colorClass="bg-cyan-500" />
                  </div>
                </div>
                <InputGroup
                  label="Ngưỡng confidence cho auto-apply (%)"
                  type="number"
                  step="1"
                  value={adaptivePhases.confidence_threshold}
                  onChange={(e: InputEvent) => saveAdaptivePhases({
                    ...adaptivePhases,
                    confidence_threshold: Math.max(0, Math.min(100, Number(e.target.value || 0)))
                  })}
                />
                <p className="text-[11px] text-slate-400">
                  Cấu hình này được lưu cục bộ theo <b>device_id = {effectiveDeviceId || '--'}</b>.
                </p>
              </div>

              {isCalibrationBlocked && (
                <div className="p-4 rounded-xl border border-rose-500/30 bg-rose-500/10 text-rose-100">
                  <p className="text-xs font-black uppercase tracking-wider mb-2">Không thể bắt đầu calib</p>
                  <ul className="text-xs space-y-1 list-disc pl-4">
                    {!isSensorOnline && <li>Sensor đang offline. Kiểm tra nguồn cảm biến hoặc kết nối mạng.</li>}
                    {isPhError && <li>err_ph=true. Rửa đầu dò, kiểm tra dây tín hiệu và chờ nhiệt độ ổn định trước khi đo lại.</li>}
                    <li>Đảm bảo đầu dò ngập đủ dung dịch chuẩn và không có bọt khí bám đầu cảm biến.</li>
                  </ul>
                </div>
              )}

              {wizardStep < calibrationPoints.length ? (
                <div className="p-4 rounded-xl border border-indigo-500/25 bg-slate-900/60 space-y-3">
                  <p className="text-xs text-slate-400 uppercase tracking-widest font-black">
                    Bước {wizardStep + 1}/{calibrationPoints.length}
                  </p>
                  <p className="text-sm font-bold text-slate-100">
                    Nhúng đầu dò vào dung dịch <span className="text-indigo-300">pH {activePoint}</span>, sau đó bấm <span className="text-emerald-300">Bắt đầu đo</span>.
                  </p>

                  <div className="flex flex-wrap items-center gap-2">
                    <button
                      onClick={handleCapturePoint}
                      disabled={isCalibrationBlocked || isCapturingPoint}
                      className="px-4 py-2 rounded-lg text-xs font-black tracking-widest bg-emerald-500 text-slate-950 disabled:opacity-50 disabled:cursor-not-allowed"
                    >
                      {isCapturingPoint ? 'ĐANG ĐO...' : 'BẮT ĐẦU ĐO'}
                    </button>

                    {isCapturingPoint && (
                      <span className="text-xs text-amber-300 font-bold flex items-center gap-1">
                        <Clock size={14} />
                        Countdown: {countdown}s · {stabilityStatus === 'stable' ? 'Đã ổn định' : 'Đang chờ ổn định'}
                      </span>
                    )}

                    {capturedPoints[activePoint] && !isCapturingPoint && (
                      <>
                        <span className="text-xs text-emerald-300 font-bold">Đã ghi nhận: {capturedPoints[activePoint].voltage.toFixed(3)}V</span>
                        <button
                          onClick={goToNextPoint}
                          className="px-3 py-2 rounded-lg text-xs font-black tracking-widest bg-indigo-500 text-white"
                        >
                          ĐÃ GHI NHẬN
                        </button>
                      </>
                    )}
                  </div>
                </div>
              ) : (
                <div className="p-4 rounded-xl border border-emerald-500/30 bg-emerald-500/10 space-y-3">
                  <p className="text-xs uppercase tracking-widest font-black text-emerald-300">Kết quả tự tính sau calib</p>
                  <div className="grid grid-cols-1 sm:grid-cols-3 gap-3 text-sm">
                    <div className="rounded-lg bg-slate-900/60 p-3 border border-white/10">
                      <p className="text-slate-400 text-xs">ph_v7</p>
                      <p className="font-black text-slate-100">{calibrationSummary.ph_v7 ?? '--'} V</p>
                    </div>
                    <div className="rounded-lg bg-slate-900/60 p-3 border border-white/10">
                      <p className="text-slate-400 text-xs">ph_v4</p>
                      <p className="font-black text-slate-100">{calibrationSummary.ph_v4 ?? '--'} V</p>
                    </div>
                    <div className="rounded-lg bg-slate-900/60 p-3 border border-white/10">
                      <p className="text-slate-400 text-xs">Độ tin cậy</p>
                      <p className="font-black text-slate-100">{calibrationSummary.reliability}%</p>
                    </div>
                  </div>
                  {calibrationSummary.ph_v10 !== null && (
                    <p className="text-xs text-slate-300">Điểm mở rộng pH 10: <b>{calibrationSummary.ph_v10}V</b> (dùng để đánh giá tuyến tính).</p>
                  )}
                  <div className="rounded-lg bg-slate-900/50 border border-white/10 p-3 text-xs text-slate-200 space-y-2">
                    <p className="font-bold text-amber-300">Sai lệch so với hệ số cũ:</p>
                    <p>Δ ph_v7: <b>{calibrationDeviation.ph_v7 ?? '--'} V</b> · Δ ph_v4: <b>{calibrationDeviation.ph_v4 ?? '--'} V</b></p>
                  </div>

                  <div className="flex flex-wrap gap-2">
                    {adaptivePhases.observe && (
                      <div className="px-3 py-2 rounded-lg text-[11px] font-bold uppercase tracking-widest bg-indigo-500/20 border border-indigo-400/30 text-indigo-200">
                        Pha 1: Chỉ quan sát, chưa điều khiển
                      </div>
                    )}

                    {adaptivePhases.recommend && (
                      <button
                        onClick={async () => {
                          const nextConfig = applyCalibrationToConfig();
                          if (nextConfig) await handleSave(nextConfig);
                        }}
                        className="px-4 py-2 rounded-lg text-xs font-black tracking-widest bg-amber-400 text-slate-950"
                      >
                        Pha 2: XÁC NHẬN THỦ CÔNG & LƯU
                      </button>
                    )}

                    {adaptivePhases.auto_apply && (
                      <button
                        onClick={async () => {
                          const confidenceOk = calibrationSummary.reliability >= adaptivePhases.confidence_threshold;
                          if (!confidenceOk) {
                            toast.error(`Confidence ${calibrationSummary.reliability}% chưa đạt ngưỡng ${adaptivePhases.confidence_threshold}%`);
                            return;
                          }
                          if (hasSafetyWarningIn24h) {
                            toast.error('Trong 24h gần nhất có cảnh báo an toàn. Auto-apply bị chặn.');
                            return;
                          }
                          const nextConfig = applyCalibrationToConfig();
                          if (nextConfig) await handleSave(nextConfig);
                        }}
                        className="px-4 py-2 rounded-lg text-xs font-black tracking-widest bg-emerald-500 text-slate-950"
                      >
                        Pha 3: AUTO-APPLY NGAY
                      </button>
                    )}
                  </div>
                </div>
              )}
            </div>
          </SubCard>

          <SubCard title="Thông Số Calib Analog (khác)">
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
              <InputGroup label="Hệ số nhân EC (K Factor)" step="1.0" value={config.ec_factor} onChange={(e: InputEvent) => setConfig({ ...config, ec_factor: e.target.value })} />
              <InputGroup label="Bù trừ sai số EC tĩnh (Offset)" step="0.1" value={config.ec_offset} onChange={(e: InputEvent) => setConfig({ ...config, ec_offset: e.target.value })} />
              <InputGroup label="Bù sai số Nhiệt độ đo được (Offset)" step="0.1" value={config.temp_offset} onChange={(e: InputEvent) => setConfig({ ...config, temp_offset: e.target.value })} />
              <InputGroup label="Hệ số bù Nhiệt cho EC (Beta)" step="0.01" value={config.temp_compensation_beta} onChange={(e: InputEvent) => setConfig({ ...config, temp_compensation_beta: e.target.value })} />
            </div>
          </SubCard>
        </AccordionSection>
      </div>

      {/* 🟢 THANH ĐIỀU KHIỂN FIXED Ở ĐÁY */}
      <div className="fixed bottom-[90px] md:bottom-28 left-0 right-0 z-40 pointer-events-none">
        <div className="max-w-4xl mx-auto px-4">
          <div className="absolute bottom-0 left-0 w-full h-32 bg-gradient-to-t from-slate-950 via-slate-950/80 to-transparent -z-10 pointer-events-none"></div>

          <button
            onClick={handleSave}
            disabled={isSaving}
            className="w-full pointer-events-auto bg-gradient-to-r from-emerald-500 to-cyan-500 text-slate-950 py-4 rounded-2xl font-black text-[13px] uppercase tracking-widest shadow-[0_10px_30px_rgba(16,185,129,0.4)] hover:shadow-[0_10px_40px_rgba(16,185,129,0.6)] hover:scale-[1.01] active:scale-95 transition-all duration-300 disabled:opacity-50 disabled:hover:scale-100 flex items-center justify-center space-x-2 relative overflow-hidden"
          >
            <div className="absolute inset-0 bg-white/20 -translate-x-full animate-[shimmer_3s_infinite]"></div>

            {isSaving ? (
              <span className="animate-spin w-5 h-5 border-[3px] border-slate-950/30 border-t-slate-950 rounded-full relative z-10"></span>
            ) : (
              <>
                <Save size={18} className="relative z-10" />
                <span className="relative z-10">LƯU CÀI ĐẶT & GỬI XUỐNG TỦ ĐIỆN</span>
              </>
            )}
          </button>
        </div>
      </div>
    </div>
  );
};

export default Settings;
