import { createContext, useContext, useState, useEffect, useCallback, ReactNode, useRef, useMemo } from 'react';
import { SensorData, StatusPayload, PumpStatus, AppSettings } from '../types/models';
import toast from 'react-hot-toast';
import { httpFetch } from '../platform/http';
import { getItem, setItem } from '../platform/storage';
import { hasRequiredRemoteConfig, isTauriRuntime, loadAppSettings } from '../platform/settings';
import { debugLog } from '../lib/redact';

interface FriendlyState {
  label: string;
  description: string;
  type: 'default' | 'success' | 'warn' | 'danger' | 'info' | 'mist';
}

interface ComputedHealth {
  score: number;
  label: string;
  color: string;
  description: string;
}

interface DeviceContextType {
  deviceId: string | null;
  settings: AppSettings | null;
  isMissingConfig: boolean;
  sensorData: SensorData | null;
  deviceStatus: StatusPayload;
  isControllerStatusKnown: boolean;
  controllerHealth: any;
  fsmState: string;
  friendlyState: FriendlyState; // 🌟 THÊM MỚI: Trạng thái thân thiện với người dùng cuối
  computedHealth: ComputedHealth; // 🌟 THÊM MỚI: Trạng thái sức khỏe trực quan hóa đơn giản
  isLoading: boolean;
  systemEvents: any[];
  isSensorOnline: boolean;
  pwmPreferences: Record<string, number>;
  savePwmPreference: (pumpId: string, pwm: number) => void;
  refreshSettings: () => Promise<void>;
  refreshDeviceSnapshot: () => Promise<void>;
}

const DeviceContext = createContext<DeviceContextType | undefined>(undefined);

const defaultPumpStatus: PumpStatus = {
  pump_a: false,
  pump_b: false,
  ph_up: false,
  ph_down: false,
  osaka_pump: false,
  mist_valve: false,
  water_pump_in: false,
  water_pump_out: false
};

const createOfflineSensorSnapshot = (deviceId: string): SensorData => ({
  device_id: deviceId,
  ec: 0,
  ph: 0,
  temp: 0,
  water_level: 0,
  time: new Date().toISOString(),
  pump_status: defaultPumpStatus,
  err_water: true,
  err_temp: true,
  err_ph: true,
  err_ec: true,
});

const normalizePumpStatus = (rawPumpStatus: any = {}): PumpStatus => {
  if (!rawPumpStatus || typeof rawPumpStatus !== 'object') return defaultPumpStatus as any;

  const mapped: Record<string, string> = {
    PUMP_A: 'pump_a',
    PUMP_B: 'pump_b',
    PH_UP: 'ph_up',
    PH_DOWN: 'ph_down',
    OSAKA: 'osaka_pump',
    OSAKA_PUMP: 'osaka_pump',
    MIST: 'mist_valve',
    MIST_VALVE: 'mist_valve',
    WATER_PUMP_IN: 'water_pump_in',
    WATER_PUMP_OUT: 'water_pump_out'
  };

  const normalized: any = { ...defaultPumpStatus };
  const booleanKeys = [
    'pump_a',
    'pump_b',
    'ph_up',
    'ph_down',
    'osaka_pump',
    'mist_valve',
    'water_pump_in',
    'water_pump_out'
  ];

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

const PUMP_STATUS_STORE_KEY = 'last_pump_status';
const PWM_PREFS_STORE_KEY = 'pump_pwm_prefs';

const savePumpStatusToStore = async (pumpStatus: PumpStatus) => {
  try {
    await setItem(PUMP_STATUS_STORE_KEY, pumpStatus);
  } catch (e) { /* bỏ qua */ }
};

const loadPumpStatusFromStore = async (): Promise<PumpStatus | null> => {
  try {
    return await getItem<PumpStatus>(PUMP_STATUS_STORE_KEY);
  } catch (e) { return null; }
};

const loadPwmPrefsFromStore = async (): Promise<Record<string, number> | null> => {
  try {
    return await getItem<Record<string, number>>(PWM_PREFS_STORE_KEY);
  } catch (e) { return null; }
};

const flattenUnifiedConfig = (raw: any) => {
  if (!raw || typeof raw !== 'object') return {};
  return {
    ...(raw.device_config || {}),
    ...(raw.water_config || {}),
    ...(raw.safety_config || {}),
    ...(raw.sensor_calibration || {}),
    ...(raw.dosing_calibration || {})
  };
};

const phaseToString = (phase: any): string | null => {
  if (phase == null) return null;
  if (typeof phase === 'string') return phase;
  if (typeof phase === 'object') {
    const key = Object.keys(phase)[0];
    const value = key ? phase[key] : null;
    if (key === 'Fault') return `SystemFault:${value || ''}`.trim();
    if (key === 'EmergencyStop') return `EmergencyStop:${value || ''}`.trim();
    return key || JSON.stringify(phase);
  }
  return String(phase);
};

export const DeviceProvider = ({ children }: { children: ReactNode }) => {
  const [deviceId, setDeviceId] = useState<string | null>(null);
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [isMissingConfig, setIsMissingConfig] = useState(false);

  const [sensorData, setSensorData] = useState<SensorData | null>(null);
  const [controllerHealth, setControllerHealth] = useState<any>(null);

  const [deviceStatus, setDeviceStatus] = useState<StatusPayload>({ is_online: false, last_seen: '' });
  const [isControllerStatusKnown, setIsControllerStatusKnown] = useState(false);
  const [fsmState, setFsmState] = useState<string>("Offline");
  const [systemEvents, setSystemEvents] = useState<any[]>([]);
  const [isLoading, setIsLoading] = useState(true);

  const [isSensorOnline, setIsSensorOnline] = useState<boolean>(false);
  const [pwmPreferences, setPwmPreferences] = useState<Record<string, number>>({});

  const sensorTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

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
            mergedSettings = { ...s, ...flattenUnifiedConfig(unifiedConfig) };
          }
        } catch (_) { }
      }

      setSettings(mergedSettings);
      setDeviceId(s.device_id || null);
      if (!isTauriRuntime() && !hasRequiredRemoteConfig(s)) {
        setIsMissingConfig(true);
      } else {
        setIsMissingConfig(false);
      }
    } else if (!isTauriRuntime()) {
      setIsMissingConfig(true);
    }
  }, []);

  const resetSensorTimeout = useCallback(() => {
    if (sensorTimeoutRef.current) clearTimeout(sensorTimeoutRef.current);

    sensorTimeoutRef.current = setTimeout(() => {
      setIsSensorOnline(false);
      toast.error("Mất tín hiệu từ bồn chứa. Đang hiển thị dữ liệu lưu lần cuối.");
    }, 65000);
  }, []);

  const savePwmPreference = useCallback(async (pumpId: string, pwm: number) => {
    setPwmPreferences(prev => {
      const updated = { ...prev, [pumpId]: pwm };
      setItem(PWM_PREFS_STORE_KEY, updated).catch(() => { });
      return updated;
    });
  }, []);

  const applyPumpStatus = useCallback((pumpStatus: PumpStatus) => {
    savePumpStatusToStore(pumpStatus);
    setSensorData(prev => ({
      ...((prev || {}) as SensorData),
      device_id: prev?.device_id || deviceId || '',
      ec: prev?.ec ?? 0,
      ph: prev?.ph ?? 0,
      temp: prev?.temp ?? 0,
      water_level: prev?.water_level ?? 0,
      time: prev?.time || new Date().toISOString(),
      pump_status: pumpStatus
    }));
    if (pumpStatus.pump_a_pwm) savePwmPreference('PUMP_A', pumpStatus.pump_a_pwm);
    if (pumpStatus.pump_b_pwm) savePwmPreference('PUMP_B', pumpStatus.pump_b_pwm);
    if (pumpStatus.osaka_pwm) savePwmPreference('OSAKA', pumpStatus.osaka_pwm);
    if (pumpStatus.ph_down_pwm) savePwmPreference('PH_DOWN', pumpStatus.ph_down_pwm);
    if (pumpStatus.ph_up_pwm) savePwmPreference('PH_UP', pumpStatus.ph_up_pwm);
  }, [deviceId, savePwmPreference]);

  const applyDeviceSnapshot = useCallback((snapshot: any) => {
    if (!snapshot || typeof snapshot !== 'object') return;

    const state = snapshot.fsm_state || snapshot.fsm_phase || snapshot.current_phase || snapshot.current_state;
    if (state) {
      setFsmState(phaseToString(state) || 'Monitoring');
    }
    if (snapshot.budgets) {
      setDeviceStatus(prev => ({ ...prev, budgets: snapshot.budgets }));
    }
    if (snapshot.diagnostics) {
      setControllerHealth((prev: any) => ({ ...(prev || {}), diagnostics: snapshot.diagnostics, ...snapshot.diagnostics }));
    }
    if (snapshot.pump_status) {
      applyPumpStatus(normalizePumpStatus(snapshot.pump_status));
    }

    if (
      snapshot.ec !== undefined ||
      snapshot.ph !== undefined ||
      snapshot.temp !== undefined ||
      snapshot.water_level !== undefined ||
      snapshot.time !== undefined
    ) {
      const incomingPumpStatus = snapshot.pump_status ? normalizePumpStatus(snapshot.pump_status) : null;
      setSensorData(prev => ({
        ...((prev || {}) as SensorData),
        ...snapshot,
        device_id: snapshot.device_id || prev?.device_id || deviceId || '',
        ec: snapshot.ec !== undefined ? snapshot.ec : (prev?.ec ?? 0),
        ph: snapshot.ph !== undefined ? snapshot.ph : (prev?.ph ?? 0),
        temp: snapshot.temp !== undefined ? snapshot.temp : (prev?.temp ?? 0),
        water_level: snapshot.water_level !== undefined ? snapshot.water_level : (prev?.water_level ?? 0),
        time: snapshot.time || prev?.time || new Date().toISOString(),
        pump_status: incomingPumpStatus || prev?.pump_status || defaultPumpStatus
      }));
      if (incomingPumpStatus) savePumpStatusToStore(incomingPumpStatus);
    }
  }, [applyPumpStatus, deviceId]);

  const refreshDeviceSnapshot = useCallback(async () => {
    if (!deviceId || !settings?.backend_url) return;

    const cachedPwmPrefs = await loadPwmPrefsFromStore();
    if (cachedPwmPrefs) setPwmPreferences(cachedPwmPrefs);

    const headers = { 'Content-Type': 'application/json', 'X-API-Key': settings.api_key || '' };

    try {
      const response = await httpFetch(`${settings.backend_url}/api/devices/${deviceId}/sensors/latest`, {
        method: 'GET',
        headers
      });
      if (response.ok) {
        const resData = await response.json();
        applyDeviceSnapshot(resData.data || resData);
      }
    } catch (_) { }

    try {
      const response = await httpFetch(`${settings.backend_url}/api/devices/${deviceId}/control/state`, {
        method: 'GET',
        headers
      });
      if (response.ok) {
        const resData = await response.json();
        applyDeviceSnapshot(resData.data || resData);
      }
    } catch (_) { }
  }, [applyDeviceSnapshot, deviceId, settings]);

  // 🌟 CƠ CHẾ UX MỚI 1: Trừu tượng hóa FSM Core cứng nhắc thành câu thoại ngôn ngữ tự nhiên
  const friendlyState = useMemo<FriendlyState>(() => {
    if (!deviceStatus.is_online || fsmState === 'Offline') {
      return { label: 'Ngoại tuyến', description: 'Trạm điều khiển đang mất kết nối mạng.', type: 'danger' };
    }
    if (fsmState.startsWith('SystemFault:')) {
      const code = fsmState.replace('SystemFault:', '').trim();
      return { label: `Cần kiểm tra: ${code}`, description: 'Hệ thống đã tạm dừng để đảm bảo an toàn cho cây.', type: 'danger' };
    }
    if (fsmState.startsWith('EmergencyStop:')) {
      return { label: 'Dừng khẩn cấp', description: 'Phần cứng đã bị ngắt kích hoạt toàn bộ do lệnh cưỡng chế.', type: 'danger' };
    }
    if (fsmState.startsWith('SensorCalibration:')) {
      return { label: 'Đang hiệu chuẩn', description: 'Đang trong quá trình tinh chỉnh độ nhạy đầu dò cảm biến.', type: 'info' };
    }

    switch (fsmState) {
      case 'Booting':
      case 'SystemBooting':
        return { label: 'Đang khởi động', description: 'Thiết bị đang rà soát cấu trúc hệ thống thủy lực.', type: 'info' };
      case 'Monitoring':
        return { label: 'Đang chăm sóc tự động', description: 'Mô hình thông minh đang tối ưu môi trường sinh trưởng lý tưởng.', type: 'success' };
      case 'MimoDosing':
        return { label: 'Đang bổ sung vi chất', description: 'Hệ thống đang tự cân bằng dinh dưỡng và độ pH bồn chứa.', type: 'mist' };
      case 'ActiveMixing':
        return { label: 'Đang sục trộn phân đều', description: 'Bơm trộn tuần hoàn đang hòa tan đều hóa chất trong nước.', type: 'info' };
      case 'Stabilizing':
        return { label: 'Đang lắng bão hòa', description: 'Chờ dòng nước tĩnh lặng để cảm biến chốt số liệu bão hòa.', type: 'warn' };
      case 'Cooldown':
        return { label: 'Đang nghỉ ngơi dưỡng bồn', description: 'Hệ thống tạm khóa bảo vệ để hóa chất thẩm thấu an toàn.', type: 'warn' };
      case 'ManualMode':
        return { label: 'Chế độ thủ công', description: 'Người dùng đang vận hành rơ-le bằng tay qua bảng điều khiển.', type: 'warn' };
      default:
        return { label: fsmState, description: 'Đang thực thi tác vụ nền.', type: 'default' };
    }
  }, [fsmState, deviceStatus.is_online]);

  // 🌟 CƠ CHẾ UX MỚI 2: Đơn giản hóa bảng chẩn đoán lỗi thành thanh Sức khỏe người dùng (Consumer UI)
  const computedHealth = useMemo<ComputedHealth>(() => {
    const rawScore = controllerHealth?.health_score_percent ?? controllerHealth?.diagnostics?.health_score_percent;
    const score = typeof rawScore === 'number' ? rawScore : 100;

    if (!deviceStatus.is_online) {
      return { score: 0, label: 'Mất kết nối', color: 'bg-rose-500', description: 'Không có dữ liệu chẩn đoán từ thiết bị ngoại vi.' };
    }
    if (score >= 90) {
      return { score, label: 'Hệ thống hoàn hảo', color: 'bg-emerald-500', description: 'Mọi mạch điện, rơ-le và đường ống silicon đều hoạt động tuyệt vời.' };
    }
    if (score >= 60) {
      return { score, label: 'Cần lưu ý', color: 'bg-amber-500', description: 'Phát hiện có mạch châm bị gợn bọt khí hoặc phản ứng hóa chất chậm nhẹ.' };
    }
    return { score, label: 'Yêu cầu kiểm tra', color: 'bg-rose-500', description: 'Phát hiện có đường ống bị nghẹt hoặc một bình chứa thuốc đã cạn.' };
  }, [controllerHealth, deviceStatus.is_online]);

  useEffect(() => {
    const loadSettings = async () => {
      try {
        await refreshSettings();
        setIsLoading(false);
      } catch (error) {
        console.error("Lỗi load settings:", error);
        setIsLoading(false);
      }
    };
    loadSettings();
  }, [refreshSettings]);

  useEffect(() => {
    const onSettingsUpdated = () => {
      refreshSettings().catch(() => { });
    };
    window.addEventListener('hydragrow:settings-updated', onSettingsUpdated);
    window.addEventListener('focus', onSettingsUpdated);
    return () => {
      window.removeEventListener('hydragrow:settings-updated', onSettingsUpdated);
      window.removeEventListener('focus', onSettingsUpdated);
    };
  }, [refreshSettings]);

  useEffect(() => {
    if (!deviceId || !settings) return;

    let ws: WebSocket;
    let pingInterval: ReturnType<typeof setTimeout>;
    let reconnectTimeout: ReturnType<typeof setTimeout>;

    const setupConnection = async () => {
      setIsLoading(true);

      const cachedPumpStatus = await loadPumpStatusFromStore();
      if (cachedPumpStatus) applyPumpStatus(cachedPumpStatus);
      setSensorData(prev => prev || createOfflineSensorSnapshot(deviceId));
      setIsLoading(false);
      refreshDeviceSnapshot().catch(() => { });

      try {
        const res = await httpFetch(`${settings.backend_url}/api/devices/${deviceId}/events`, {
          method: 'GET',
          headers: { 'X-API-Key': settings.api_key || '' }
        });
        if (res.ok) {
          const json = await res.json();
          if (json.data && Array.isArray(json.data)) setSystemEvents(json.data);
        }
      } catch (err) { /* empty */ }

      const connectWs = () => {
        const cleanBaseUrl = settings.backend_url.replace(/\/$/, "");
        const wsUrl = `${cleanBaseUrl.replace(/^http/, 'ws')}/api/devices/${deviceId}/ws`;
        ws = new WebSocket(wsUrl);

        ws.onopen = () => {
          console.log('🟢 [GlobalContext] Đã kết nối tới Server WebSocket');
          ws.send(JSON.stringify({ type: 'auth', api_key: settings.api_key }));
          setIsControllerStatusKnown(false);
          resetSensorTimeout();

          httpFetch(`${settings.backend_url}/api/devices/${deviceId}/control/sync`, {
            method: 'POST',
            headers: { 'X-API-Key': settings.api_key }
          }).catch(() => debugLog("Lỗi gửi lệnh Sync ban đầu"));
          refreshDeviceSnapshot().catch(() => { });

          pingInterval = setInterval(() => {
            if (ws.readyState === WebSocket.OPEN) ws.send('ping');
          }, 25000);
        };

        ws.onmessage = (event) => {
          try {
            const data = JSON.parse(event.data);
            console.log("📥 RAW WS MESSAGE:", data.type || data._msg_type);

            if (data._msg_type === 'fsm_status' || data.type === 'fsm_status') {
              const payload = data.payload || data;

              let newState = payload.current_state || payload.current_phase;
              if (newState) {
                setFsmState(phaseToString(newState) || 'Monitoring');
              }

              if (payload.budgets) {
                setDeviceStatus(prev => ({ ...prev, budgets: payload.budgets }));
              }

              if (payload.pump_status && Object.keys(payload.pump_status).length > 0) {
                applyPumpStatus(normalizePumpStatus(payload.pump_status));
              }

              return;
            }

            if (data.type === 'fsm_state_update') {
              const payload = data.payload || {};
              const newState = payload.current_phase || payload.current_state || payload.fsm_state;

              if (newState) {
                setFsmState(phaseToString(newState) || 'Monitoring');
              }
              if (typeof payload.online === 'boolean') {
                setDeviceStatus(prev => ({
                  ...prev,
                  is_online: payload.online,
                  last_seen: new Date().toISOString()
                }));
                setIsControllerStatusKnown(true);
              }
              if (payload.budgets) {
                setDeviceStatus(prev => ({ ...prev, budgets: payload.budgets }));
              }
              if (payload.diagnostics) {
                setControllerHealth((prev: any) => ({ ...(prev || {}), diagnostics: payload.diagnostics, ...payload.diagnostics }));
              }
              if (payload.pump_status) {
                applyPumpStatus(normalizePumpStatus(payload.pump_status));
              }

              return;
            }

            if (data.type === 'device_status') {
              const payload = data.payload || {};
              const isOnline: boolean = payload.is_online ?? payload.online ?? false;

              setIsControllerStatusKnown(true);
              setDeviceStatus(prev => {
                if (prev.is_online !== isOnline) {
                  if (isOnline) toast.success("Trạm điều khiển đã trực tuyến trở lại.");
                  else toast.error("Trạm điều khiển đã ngắt kết nối mạng.");
                }

                return {
                  ...prev,
                  ...payload,
                  is_online: isOnline,
                  last_seen: new Date().toISOString()
                };
              });

              if (!isOnline) {
                setFsmState("Offline");
              }
              return;
            }

            if (data.type === 'alert') {
              const alert = data.payload;

              if (alert.title === 'Trạng thái Trạm Điều Khiển') {
                const isOnline = alert.level === 'success';
                setDeviceStatus({ is_online: isOnline, last_seen: new Date().toISOString() });
                setIsControllerStatusKnown(true);
                if (isOnline) {
                  toast.success("Trạm điều khiển đã trực tuyến.");
                } else {
                  setFsmState("Offline");
                  setSensorData(prev => prev ? { ...prev, pump_status: {} as any } : prev);
                  toast.error("Trạm điều khiển đã mất kết nối mạng.");
                }
                return;
              }

              if (alert.title === 'Trạng thái Mạch Cảm Biến') {
                const onlineStatus = alert.level === 'success';
                setIsSensorOnline(onlineStatus);
                if (!onlineStatus) {
                  toast.error("Hộp cảm biến bồn chứa đã ngắt kết nối.");
                  setSensorData(prev => prev ? { ...prev, err_water: true, err_temp: true, err_ph: true, err_ec: true } : prev);
                  if (sensorTimeoutRef.current) clearTimeout(sensorTimeoutRef.current);
                } else {
                  toast.success("Tín hiệu bồn chứa đã trực tuyến.");
                  resetSensorTimeout();
                }
                return;
              }

              setSystemEvents(prev => [alert, ...prev].slice(0, 50));
              if (alert.level === 'critical' || alert.level === 'warning') {
                toast.error(`${alert.title}\n${alert.message}`, { id: 'sys-alert', duration: 4000 });
              } else if (alert.level === 'success') {
                toast.success(`✅ ${alert.title}\n${alert.message}`, { id: 'sys-success', duration: 3000 });
              } return;
            }

            if (data.type === 'sensor_update') {
              const incomingPayload = data.payload.data || data.payload;
              const incomingPumpStatus = incomingPayload?.pump_status
                ? normalizePumpStatus(incomingPayload.pump_status)
                : null;

              setSensorData(prev => {
                if (!prev) {
                  return {
                    ...incomingPayload,
                    pump_status: incomingPumpStatus || normalizePumpStatus(incomingPayload?.pump_status)
                  };
                }
                return {
                  ...prev,
                  pump_status: incomingPumpStatus || prev.pump_status,
                  temp: incomingPayload.temp !== undefined ? incomingPayload.temp : prev.temp,
                  ec: incomingPayload.ec !== undefined ? incomingPayload.ec : prev.ec,
                  ph: incomingPayload.ph !== undefined ? incomingPayload.ph : prev.ph,
                  water_level: incomingPayload.water_level !== undefined ? incomingPayload.water_level : prev.water_level,
                  err_water: incomingPayload.err_water !== undefined ? incomingPayload.err_water : prev.err_water,
                  err_temp: incomingPayload.err_temp !== undefined ? incomingPayload.err_temp : prev.err_temp,
                  err_ph: incomingPayload.err_ph !== undefined ? incomingPayload.err_ph : prev.err_ph,
                  err_ec: incomingPayload.err_ec !== undefined ? incomingPayload.err_ec : prev.err_ec,
                  is_continuous: incomingPayload.is_continuous !== undefined ? incomingPayload.is_continuous : prev.is_continuous,
                  rssi: incomingPayload.rssi !== undefined ? incomingPayload.rssi : prev.rssi,
                  free_heap: incomingPayload.free_heap !== undefined ? incomingPayload.free_heap : prev.free_heap,
                  uptime: incomingPayload.uptime !== undefined ? incomingPayload.uptime : prev.uptime,
                  ph_voltage_mv: incomingPayload.ph_voltage_mv !== undefined ? incomingPayload.ph_voltage_mv : prev.ph_voltage_mv,
                };
              });

              if (incomingPumpStatus) savePumpStatusToStore(incomingPumpStatus);

              setIsSensorOnline(true);
              resetSensorTimeout();
              return;
            }

            if (data.type === 'controller_status') {
              const payload = data.payload || {};
              const healthState = payload.fsm_state_display ?? payload.fsm_state ?? payload.current_phase;
              if (healthState) {
                setFsmState(phaseToString(healthState) || 'Monitoring');
              }
              if (payload.budgets) {
                setDeviceStatus(prev => ({ ...prev, budgets: payload.budgets }));
              }
              if (payload.pump_status) {
                applyPumpStatus(normalizePumpStatus(payload.pump_status));
              }
              if (payload.online !== undefined || payload.is_online !== undefined) {
                const isOnline = payload.is_online ?? payload.online;
                setDeviceStatus(prev => ({ ...prev, is_online: Boolean(isOnline), last_seen: new Date().toISOString() }));
                setIsControllerStatusKnown(true);
              }
              return;
            }

            if (data.type === 'device_health' || data.type === 'health_snapshot') {
              const healthData = data.payload;

              setControllerHealth({
                rssi: healthData.rssi,
                free_heap: healthData.free_heap,
                uptime: healthData.uptime_sec,
                health_score_percent: healthData.health_score_percent,
                fsm_state_display: healthData.fsm_state_display,
                log_drop_count: healthData.log_drop_count,
                kalman_confidence: healthData.kalman_confidence || null,
                matrix_update_count: healthData.matrix_update_count,
                matrix_is_warm: healthData.matrix_is_warm,
                hestia: healthData.hestia || null,
                diagnostics: healthData.diagnostics || null
              });

              const healthState = healthData.fsm_state_display ?? healthData.fsm_state;
              if (healthState) {
                setFsmState(typeof healthState === 'object' ? JSON.stringify(healthState) : String(healthState));
              }

              if (healthData.budgets && Object.keys(healthData.budgets).length > 0) {
                setDeviceStatus(prev => ({ ...prev, budgets: healthData.budgets }));
              }

              if (healthData.pump_status) {
                applyPumpStatus(normalizePumpStatus(healthData.pump_status));
              }

              setDeviceStatus(prev => !prev.is_online ? { is_online: true, last_seen: new Date().toISOString() } : prev);
              setIsControllerStatusKnown(true);
              return;
            }

          } catch (err) {
            console.error("Lỗi xử lý luồng WS Message:", err);
          }
        };

        ws.onclose = () => {
          debugLog('🔴 [GlobalContext] Mất kết nối WebSocket. Đang tự động cấu hình lại...');
          // setDeviceStatus({ is_online: false, last_seen: '' });
          // setIsControllerStatusKnown(true);
          setIsSensorOnline(false);
          clearInterval(pingInterval);
          if (sensorTimeoutRef.current) clearTimeout(sensorTimeoutRef.current);

          reconnectTimeout = setTimeout(() => { connectWs(); }, 5000);
        };

        ws.onerror = (_err) => ws.close();
      };

      connectWs();
    };

    setupConnection();

    return () => {
      clearInterval(pingInterval);
      clearTimeout(reconnectTimeout);
      if (sensorTimeoutRef.current) clearTimeout(sensorTimeoutRef.current);
      if (ws) {
        ws.onclose = null;
        ws.close();
      }
    };
  }, [deviceId, settings, resetSensorTimeout, applyPumpStatus, refreshDeviceSnapshot]);

  return (
    <DeviceContext.Provider value={{
      deviceId, sensorData, deviceStatus, isControllerStatusKnown, controllerHealth, fsmState, isLoading,
      friendlyState, computedHealth, // 🌟 Khai thông mạch ống dữ liệu hướng người dùng cuối lên các trang UI
      settings, systemEvents, isSensorOnline, isMissingConfig,
      pwmPreferences, savePwmPreference, refreshSettings, refreshDeviceSnapshot
    }}>
      {children}
    </DeviceContext.Provider>
  );
};

export const useDeviceContext = () => {
  const context = useContext(DeviceContext);
  if (context === undefined) throw new Error('useDeviceContext must be used within a DeviceProvider');
  return context;
};
