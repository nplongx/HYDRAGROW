import { createContext, useContext, useState, useEffect, useCallback, ReactNode, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { fetch } from '@tauri-apps/plugin-http';
import { SensorData, StatusPayload, PumpStatus } from '../types/models';
import toast from 'react-hot-toast';
import { Store } from '@tauri-apps/plugin-store';

interface DeviceContextType {
  deviceId: string | null;
  settings: any;
  sensorData: SensorData | null;
  deviceStatus: StatusPayload;
  isControllerStatusKnown: boolean;
  controllerHealth: any;
  fsmState: string;
  isLoading: boolean;
  updatePumpStatusOptimistically: (stateKey: string, isNowOn: boolean) => void;
  systemEvents: any[];
  isSensorOnline: boolean;
  // 🟢 THÊM MỚI: Quản lý PWM Preferences
  pwmPreferences: Record<string, number>;
  savePwmPreference: (pumpId: string, pwm: number) => void;
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

const normalizePumpStatus = (rawPumpStatus: any = {}): PumpStatus => {
  if (!rawPumpStatus || typeof rawPumpStatus !== 'object') return defaultPumpStatus;

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

  const normalized = { ...defaultPumpStatus };
  const booleanKeys: Array<keyof PumpStatus> = [
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
    if (booleanKeys.includes(normalizedKey as keyof PumpStatus)) {
      (normalized as unknown as Record<string, boolean>)[normalizedKey] = Boolean(value);
    }
  });

  return normalized;
};

const PUMP_STATUS_STORE_KEY = 'last_pump_status';
const PWM_PREFS_STORE_KEY = 'pump_pwm_prefs'; // 🟢 THÊM MỚI: Key lưu PWM

const savePumpStatusToStore = async (pumpStatus: PumpStatus) => {
  try {
    const store = await Store.load('device-state.json');
    await store.set(PUMP_STATUS_STORE_KEY, pumpStatus);
    await store.save();
  } catch (e) { /* bỏ qua */ }
};

const loadPumpStatusFromStore = async (): Promise<PumpStatus | null> => {
  try {
    const store = await Store.load('device-state.json');
    const val = await store.get<PumpStatus>(PUMP_STATUS_STORE_KEY);
    return val ?? null;
  } catch (e) { return null; }
};

// 🟢 THÊM MỚI: Hàm load PWM từ ổ cứng
const loadPwmPrefsFromStore = async (): Promise<Record<string, number> | null> => {
  try {
    const store = await Store.load('device-state.json');
    const val = await store.get<Record<string, number>>(PWM_PREFS_STORE_KEY);
    return val ?? null;
  } catch (e) { return null; }
};

export const DeviceProvider = ({ children }: { children: ReactNode }) => {
  const [deviceId, setDeviceId] = useState<string | null>(null);
  const [settings, setSettings] = useState<any>(null);

  const [sensorData, setSensorData] = useState<SensorData | null>(null);
  const [controllerHealth, setControllerHealth] = useState<any>(null);

  const [deviceStatus, setDeviceStatus] = useState<StatusPayload>({ is_online: false, last_seen: '' });
  const [isControllerStatusKnown, setIsControllerStatusKnown] = useState(false);
  const [fsmState, setFsmState] = useState<string>("Offline");
  const [systemEvents, setSystemEvents] = useState<any[]>([]);
  const [isLoading, setIsLoading] = useState(true);

  const [isSensorOnline, setIsSensorOnline] = useState<boolean>(false);

  // 🟢 THÊM MỚI: State chứa danh sách PWM của các bơm
  const [pwmPreferences, setPwmPreferences] = useState<Record<string, number>>({});

  const sensorTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const resetSensorTimeout = useCallback(() => {
    if (sensorTimeoutRef.current) clearTimeout(sensorTimeoutRef.current);

    sensorTimeoutRef.current = setTimeout(() => {
      setIsSensorOnline(false);
      setSensorData(prev => prev ? { ...prev, err_water: true, err_temp: true, err_ec: true, err_ph: true } : prev);
      toast.error("Mất kết nối mạch cảm biến. Vui lòng thử lại. Nếu vẫn lỗi, hãy kiểm tra nguồn và mạng.");
    }, 65000);
  }, []);

  useEffect(() => {
    const loadSettings = async () => {
      try {
        const s: any = await invoke('load_settings').catch(() => null);
        if (s && s.device_id && s.backend_url) {
          setSettings(s);
          setDeviceId(s.device_id);
        } else {
          setIsLoading(false);
        }
      } catch (error) {
        console.error("Lỗi load settings:", error);
        setIsLoading(false);
      }
    };
    loadSettings();
  }, []);

  useEffect(() => {
    if (!deviceId || !settings) return;

    let ws: WebSocket;
    let pingInterval: ReturnType<typeof setTimeout>;
    let reconnectTimeout: ReturnType<typeof setTimeout>;

    const setupConnection = async () => {
      setIsLoading(true);

      try {
        const url = `${settings.backend_url}/api/devices/${deviceId}/sensors/latest`;
        const response = await fetch(url, {
          method: 'GET',
          headers: { 'Content-Type': 'application/json', 'X-API-Key': settings.api_key }
        });
        if (response.ok) {
          const resData = await response.json();
          const initialData = resData.data || resData;

          const cachedPumpStatus = await loadPumpStatusFromStore();

          // 🟢 THÊM MỚI: Đọc PWM từ App Storage khi vừa khởi động
          const cachedPwmPrefs = await loadPwmPrefsFromStore();
          if (cachedPwmPrefs) setPwmPreferences(cachedPwmPrefs);

          setSensorData({
            ...initialData,
            pump_status: cachedPumpStatus
              ? cachedPumpStatus
              : normalizePumpStatus(initialData?.pump_status)
          });
        }
      } catch (err) { /* empty */ }
      setIsLoading(false);

      try {
        const res = await fetch(`${settings.backend_url}/api/devices/${deviceId}/events`, {
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
        const wsUrl = `${cleanBaseUrl.replace(/^http/, 'ws')}/api/devices/${deviceId}/ws?api_key=${settings.api_key}`;
        ws = new WebSocket(wsUrl);

        ws.onopen = () => {
          console.log('🟢 [GlobalContext] Đã kết nối tới Server WebSocket');
          setIsControllerStatusKnown(false);
          resetSensorTimeout();

          fetch(`${settings.backend_url}/api/devices/${deviceId}/control/sync`, {
            method: 'POST',
            headers: { 'X-API-Key': settings.api_key }
          }).catch(() => console.log("Lỗi gửi lệnh Sync ban đầu"));

          pingInterval = setInterval(() => {
            if (ws.readyState === WebSocket.OPEN) ws.send('ping');
          }, 25000);
        };

        ws.onmessage = (event) => {
          try {
            const data = JSON.parse(event.data);

            if (data.type === 'device_status') {
              const isOnline: boolean = data.payload.is_online ?? false;
              setIsControllerStatusKnown(true);
              setDeviceStatus(prev => {
                if (prev.is_online !== isOnline) {
                  if (isOnline) toast.success("Trạm điều khiển đã trực tuyến trở lại.");
                  else toast.error("Trạm điều khiển đã ngắt kết nối. Vui lòng kiểm tra mạng rồi thử lại.");
                }
                return { is_online: isOnline, last_seen: new Date().toISOString() };
              });

              if (!isOnline) {
                setFsmState("Offline");
                setSensorData(prev => prev ? { ...prev, pump_status: {} as any } : prev);
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
                  toast.success("Trạm điều khiển đã trực tuyến trở lại.");
                } else {
                  setFsmState("Offline");
                  setSensorData(prev => prev ? { ...prev, pump_status: {} as any } : prev);
                  toast.error("Trạm điều khiển đã ngắt kết nối. Vui lòng kiểm tra mạng rồi thử lại.");
                }
                return;
              }

              if (alert.title === 'Trạng thái Mạch Cảm Biến') {
                const onlineStatus = alert.level === 'success';
                setIsSensorOnline(onlineStatus);
                if (!onlineStatus) {
                  toast.error("Mạch cảm biến đã mất kết nối. Vui lòng kiểm tra nguồn cảm biến và mạng.");
                  setSensorData(prev => prev ? { ...prev, err_water: true, err_temp: true, err_ph: true, err_ec: true } : prev);
                  if (sensorTimeoutRef.current) clearTimeout(sensorTimeoutRef.current);
                } else {
                  toast.success("Mạch cảm biến đã trực tuyến.");
                  resetSensorTimeout();
                }
                return;
              }

              if (alert.level === 'FSM_UPDATE') {
                setFsmState(alert.message);
                return;
              }

              setSystemEvents(prev => [alert, ...prev].slice(0, 50));
              switch (alert.level) {
                case 'critical': toast.error(`🚨 ${alert.title}\n${alert.message}`, { duration: 10000 }); break;
                case 'warning': toast.error(`⚠️ ${alert.title}\n${alert.message}`, { duration: 6000 }); break;
                case 'success': toast.success(`✅ ${alert.title}\n${alert.message}`, { duration: 5000 }); break;
                default: toast(`ℹ️ ${alert.title}`, { duration: 4000 }); break;
              }
              return;
            }

            if (data.type === 'sensor_update') {
              const incomingPayload = data.payload.data || data.payload;

              setSensorData(prev => {
                if (!prev) return incomingPayload;
                return {
                  ...prev,
                  pump_status: incomingPayload.pump_status !== undefined
                    ? normalizePumpStatus(incomingPayload.pump_status)
                    : prev.pump_status,
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

              setIsSensorOnline(true);
              resetSensorTimeout();
              return;
            }

            if (data.type === 'device_health') {
              const healthData = data.payload;

              setControllerHealth({
                rssi: healthData.rssi,
                free_heap: healthData.free_heap,
                uptime: healthData.uptime_sec
              });

              const confirmedPumpStatus = normalizePumpStatus(healthData.pump_status);
              savePumpStatusToStore(confirmedPumpStatus);

              // 🟢 THÊM MỚI: Cập nhật lại PWM vào App nếu ESP32 có gửi kèm
              if (healthData.pump_status) {
                const raw = healthData.pump_status;
                if (raw.pump_a_pwm !== undefined && raw.pump_a_pwm > 0) savePwmPreference('PUMP_A', raw.pump_a_pwm);
                if (raw.pump_b_pwm !== undefined && raw.pump_b_pwm > 0) savePwmPreference('PUMP_B', raw.pump_b_pwm);
                if (raw.ph_up_pwm !== undefined && raw.ph_up_pwm > 0) savePwmPreference('PH_UP', raw.ph_up_pwm);
                if (raw.ph_down_pwm !== undefined && raw.ph_down_pwm > 0) savePwmPreference('PH_DOWN', raw.ph_down_pwm);
                if (raw.osaka_pwm !== undefined && raw.osaka_pwm > 0) savePwmPreference('OSAKA', raw.osaka_pwm);
              }

              setSensorData(prev => {
                if (!prev) return prev;
                return { ...prev, pump_status: confirmedPumpStatus };
              });

              setDeviceStatus(prev => !prev.is_online ? { is_online: true, last_seen: new Date().toISOString() } : prev);
              setIsControllerStatusKnown(true);
              return;
            }

          } catch (err) {
            console.error("Lỗi parse WS Message:", err);
          }
        };

        ws.onclose = () => {
          console.log('🔴 [GlobalContext] Mất kết nối WebSocket. Đang thử kết nối lại...');
          setDeviceStatus({ is_online: false, last_seen: '' });
          setIsControllerStatusKnown(true);
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
  }, [deviceId, settings, resetSensorTimeout]);

  const updatePumpStatusOptimistically = useCallback((stateKey: string, isNowOn: boolean) => {
    setSensorData(prevData => {
      if (!prevData) return prevData;
      const newPumpStatus = {
        ...prevData.pump_status,
        [stateKey]: isNowOn
      };

      savePumpStatusToStore(newPumpStatus as PumpStatus);

      return { ...prevData, pump_status: newPumpStatus };
    });
  }, []);

  // 🟢 THÊM MỚI: Cài đặt logic cho hàm savePwmPreference
  const savePwmPreference = useCallback(async (pumpId: string, pwm: number) => {
    setPwmPreferences(prev => {
      const updated = { ...prev, [pumpId]: pwm };
      Store.load('device-state.json').then(store => {
        store.set(PWM_PREFS_STORE_KEY, updated);
        store.save();
      }).catch(() => { });
      return updated;
    });
  }, []);

  return (
    <DeviceContext.Provider value={{
      deviceId, sensorData, deviceStatus, isControllerStatusKnown, controllerHealth, fsmState, isLoading,
      updatePumpStatusOptimistically, settings, systemEvents, isSensorOnline,
      // 🟢 THÊM MỚI: Export các biến/hàm này ra để ControlPanel.tsx có thể xài được
      pwmPreferences, savePwmPreference
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
