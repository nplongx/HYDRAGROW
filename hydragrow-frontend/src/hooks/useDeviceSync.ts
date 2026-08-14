import { useEffect, useRef, useCallback } from 'react';
import { useDeviceStore } from '../store/useDeviceStore';
import { httpFetch } from '../platform/http';
import { getItem, setItem } from '../platform/storage';
import { hasRequiredRemoteConfig, isTauriRuntime, loadAppSettings } from '../platform/settings';
import toast from 'react-hot-toast';
import { PumpStatus, SensorData } from '../types/models';

const defaultPumpStatus: PumpStatus = {
  pump_a: false, pump_b: false, ph_up: false, ph_down: false,
  osaka_pump: false, mist_valve: false, water_pump_in: false, water_pump_out: false,
};

const PUMP_STATUS_STORE_KEY = 'last_pump_status';
const PWM_PREFS_STORE_KEY = 'pump_pwm_prefs';

const phaseToString = (phase: any): string | null => {
  if (phase == null) return null;
  if (typeof phase === 'string') {
    if (phase.startsWith('{')) {
      try { return phaseToString(JSON.parse(phase)); } catch (_) {}
    }
    return phase;
  }
  if (typeof phase === 'object') {
    const key = Object.keys(phase)[0];
    const value = key ? phase[key] : null;
    if (key === 'Fault') return `SystemFault:${value || ''}`.trim();
    if (key === 'EmergencyStop') return `EmergencyStop:${value || ''}`.trim();
    return key || JSON.stringify(phase);
  }
  return String(phase);
};

const normalizePumpStatus = (rawPumpStatus: any = {}): PumpStatus => {
  if (!rawPumpStatus || typeof rawPumpStatus !== 'object') return defaultPumpStatus as any;
  const mapped: Record<string, string> = {
    PUMP_A: 'pump_a', PUMP_B: 'pump_b', PH_UP: 'ph_up', PH_DOWN: 'ph_down',
    OSAKA: 'osaka_pump', OSAKA_PUMP: 'osaka_pump', MIST: 'mist_valve', MIST_VALVE: 'mist_valve',
    WATER_PUMP_IN: 'water_pump_in', WATER_PUMP_OUT: 'water_pump_out'
  };
  const normalized: any = { ...defaultPumpStatus };
  const booleanKeys = ['pump_a', 'pump_b', 'ph_up', 'ph_down', 'osaka_pump', 'mist_valve', 'water_pump_in', 'water_pump_out'];
  Object.entries(rawPumpStatus).forEach(([key, value]) => {
    const normalizedKey = mapped[key] || mapped[key.toUpperCase()] || key.toLowerCase();
    if (booleanKeys.includes(normalizedKey)) {
      normalized[normalizedKey] = Boolean(value);
    } else if (normalizedKey.includes('pwm')) {
      normalized[normalizedKey] = Number(value);
    }
  });
  return normalized;
};

export function useDeviceSync() {
  const deviceId = useDeviceStore((s) => s.deviceId);
  const settings = useDeviceStore((s) => s.settings);

  const sensorTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const resetSensorTimeout = useCallback(() => {
    if (sensorTimeoutRef.current) clearTimeout(sensorTimeoutRef.current);
    sensorTimeoutRef.current = setTimeout(() => {
      useDeviceStore.getState().setIsSensorOnline(false);
      toast.error("Mất tín hiệu cảm biến; hiển thị giá trị cuối.");
    }, 65000);
  }, []);

  const refreshSettings = useCallback(async () => {
    const s: any = await loadAppSettings();
    if (s && s.device_id && s.backend_url) {
      let mergedSettings = s;
      if (s.api_key) {
        try {
          const configRes = await httpFetch(`${s.backend_url}/api/devices/${s.device_id}/config/unified`, {
            method: 'GET',
            headers: { 'X-API-Key': s.api_key || '' }
          });
          if (configRes.ok) {
            const unifiedConfig = await configRes.json();
            mergedSettings = { ...s, ...(unifiedConfig.device_config || {}), ...(unifiedConfig.water_config || {}), ...(unifiedConfig.safety_config || {}), ...(unifiedConfig.sensor_calibration || {}), ...(unifiedConfig.dosing_calibration || {}) };
          }
        } catch (_) {}
      }
      useDeviceStore.getState().setSettings(mergedSettings);
      useDeviceStore.getState().setDeviceId(s.device_id || null);
      useDeviceStore.getState().setIsMissingConfig(!isTauriRuntime() && !hasRequiredRemoteConfig(s));
    } else if (!isTauriRuntime()) {
      useDeviceStore.getState().setIsMissingConfig(true);
    }
  }, []);

  const applyPumpStatus = useCallback((pumpStatus: PumpStatus) => {
    setItem(PUMP_STATUS_STORE_KEY, pumpStatus).catch(() => {});
    useDeviceStore.getState().setSensorData((prev) => ({
      ...((prev || {}) as SensorData),
      device_id: prev?.device_id || useDeviceStore.getState().deviceId || '',
      ec: prev?.ec ?? 0, ph: prev?.ph ?? 0, temp: prev?.temp ?? 0, water_level: prev?.water_level ?? 0,
      time: prev?.time || new Date().toISOString(),
      pump_status: pumpStatus,
    }));
  }, []);

  const applyDeviceSnapshot = useCallback((snapshot: any) => {
    if (!snapshot || typeof snapshot !== 'object') return;
    const state = snapshot.fsm_state || snapshot.fsm_phase || snapshot.current_phase || snapshot.current_state;
    if (state) useDeviceStore.getState().setFsmState(phaseToString(state) || 'Monitoring');
    if (snapshot.budgets) useDeviceStore.getState().setDeviceStatus(prev => ({ ...prev, budgets: snapshot.budgets }));
    if (snapshot.diagnostics) useDeviceStore.getState().setControllerHealth(prev => ({ ...(prev || {}), ...snapshot.diagnostics }));
    if (snapshot.pump_status) applyPumpStatus(normalizePumpStatus(snapshot.pump_status));
  }, [applyPumpStatus]);

  const refreshDeviceSnapshot = useCallback(async () => {
    const currentDeviceId = useDeviceStore.getState().deviceId;
    const currentSettings = useDeviceStore.getState().settings;
    if (!currentDeviceId || !currentSettings?.backend_url) return;
    const cachedPwm = await getItem<Record<string, number>>(PWM_PREFS_STORE_KEY);
    if (cachedPwm) useDeviceStore.getState().setPwmPreferences(cachedPwm);
    const headers = { 'Content-Type': 'application/json', 'X-API-Key': currentSettings.api_key || '' };
    try {
      const response = await httpFetch(`${currentSettings.backend_url}/api/devices/${currentDeviceId}/sensors/latest`, { method: 'GET', headers });
      if (response.ok) applyDeviceSnapshot((await response.json()).data);
    } catch (_) {}
  }, [applyDeviceSnapshot]);

  // Khởi tạo ban đầu
  useEffect(() => {
    refreshSettings().then(() => useDeviceStore.getState().setIsLoading(false));
    const onUpdate = () => refreshSettings();
    window.addEventListener('hydragrow:settings-updated', onUpdate);
    window.addEventListener('focus', onUpdate);
    return () => {
      window.removeEventListener('hydragrow:settings-updated', onUpdate);
      window.removeEventListener('focus', onUpdate);
    };
  }, [refreshSettings]);

  // Vòng đời kết nối WebSocket
  useEffect(() => {
    if (!deviceId || !settings) return;

    let ws: WebSocket;
    let pingInterval: ReturnType<typeof setTimeout>;
    let reconnectTimeout: ReturnType<typeof setTimeout>;

const connectWs = () => {
      const cleanBaseUrl = settings.backend_url.replace(/\/$/, "");
      
      // Thêm query parameter ?api_key=... vào WS URL
      const wsUrl = `${cleanBaseUrl.replace(/^http/, 'ws')}/api/devices/${deviceId}/ws?api_key=${encodeURIComponent(settings.api_key || '')}`;
      
      ws = new WebSocket(wsUrl);

      ws.onopen = () => {
        // Gửi tin nhắn auth bổ sung nếu backend yêu cầu song song
        ws.send(JSON.stringify({ type: 'auth', api_key: settings.api_key }));
        useDeviceStore.getState().setIsControllerStatusKnown(false);
        resetSensorTimeout();
        refreshDeviceSnapshot();
        pingInterval = setInterval(() => { if (ws.readyState === WebSocket.OPEN) ws.send('ping'); }, 25000);
      };

      ws.onmessage = (event) => {
        try {
          const data = JSON.parse(event.data);
          if (data.type === 'sensor_update') {
            const incomingPayload = data.payload.data || data.payload;
            useDeviceStore.getState().setSensorData(prev => ({
              ...prev,
              ...incomingPayload,
              pump_status: incomingPayload?.pump_status ? normalizePumpStatus(incomingPayload.pump_status) : prev?.pump_status
            }));
            useDeviceStore.getState().setIsSensorOnline(true);
            resetSensorTimeout();
          } else if (data.type === 'fsm_state_update' || data.type === 'controller_status') {
            const newState = data.payload?.current_phase || data.payload?.current_state;
            if (newState) useDeviceStore.getState().setFsmState(phaseToString(newState) || 'Monitoring');
          }
        } catch (e) {}
      };

      ws.onclose = () => {
        useDeviceStore.getState().setIsSensorOnline(false);
        clearInterval(pingInterval);
        reconnectTimeout = setTimeout(connectWs, 5000);
      };
    };

    connectWs();
    return () => {
      clearInterval(pingInterval);
      clearTimeout(reconnectTimeout);
      if (ws) ws.close();
    };
  }, [deviceId, settings?.backend_url, settings?.api_key, resetSensorTimeout, refreshDeviceSnapshot]);
}
