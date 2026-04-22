import { createContext, useContext, useState, useEffect, useCallback, ReactNode, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { fetch } from '@tauri-apps/plugin-http';
import { SensorData, StatusPayload } from '../types/models';
import toast from 'react-hot-toast';

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
}

const DeviceContext = createContext<DeviceContextType | undefined>(undefined);

export const DeviceProvider = ({ children }: { children: ReactNode }) => {
  const [deviceId, setDeviceId] = useState<string | null>(null);
  const [settings, setSettings] = useState<any>(null);

  const [sensorData, setSensorData] = useState<SensorData | null>(null);
  const [controllerHealth, setControllerHealth] = useState<any>(null);

  const [deviceStatus, setDeviceStatus] = useState<StatusPayload>({ is_online: false, last_seen: '' });
  const [isControllerStatusKnown, setIsControllerStatusKnown] = useState(false);
  const [isInitialConnection, setIsInitialConnection] = useState(true);
  const [fsmState, setFsmState] = useState<string>("Offline");
  const [systemEvents, setSystemEvents] = useState<any[]>([]);
  const [isLoading, setIsLoading] = useState(true);

  const [isSensorOnline, setIsSensorOnline] = useState<boolean>(false);

  const sensorTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // ─── Sensor timeout (65s) ─────────────────────────────────────────────────
  const resetSensorTimeout = useCallback(() => {
    if (sensorTimeoutRef.current) clearTimeout(sensorTimeoutRef.current);

    sensorTimeoutRef.current = setTimeout(() => {
      setIsSensorOnline(false);
      setSensorData(prev => prev ? { ...prev, err_water: true, err_temp: true, err_ec: true, err_ph: true } : prev);
      toast.error("Mất kết nối với Mạch Cảm Biến!");
    }, 65000);
  }, []);

  // ─── Load settings on mount ───────────────────────────────────────────────
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

  // ─── Main WebSocket + HTTP setup ──────────────────────────────────────────
  useEffect(() => {
    if (!deviceId || !settings) return;

    let ws: WebSocket;
    let pingInterval: ReturnType<typeof setTimeout>;
    let reconnectTimeout: ReturnType<typeof setTimeout>;

    const setupConnection = async () => {
      setIsLoading(true);

      // Initial HTTP fetch for latest sensor data
      try {
        const url = `${settings.backend_url}/api/devices/${deviceId}/sensors/latest`;
        const response = await fetch(url, {
          method: 'GET',
          headers: { 'Content-Type': 'application/json', 'X-API-Key': settings.api_key }
        });
        if (response.ok) {
          const resData = await response.json();
          setSensorData(resData.data || resData);
        }
      } catch (err) { /* empty */ }
      setIsLoading(false);

      // Initial fetch for system events
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
          setIsInitialConnection(true);

          setDeviceStatus(prev => ({ ...prev, is_online: true, last_seen: new Date().toISOString() }));
          setIsControllerStatusKnown(true);
          resetSensorTimeout();

          // 🟢 Ép Controller Node gửi toàn bộ trạng thái mới nhất lập tức!
          fetch(`${settings.backend_url}/api/control/${deviceId}/sync`, {
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

            // ── device_status ────────────────────────────────────────────────
            // Comes from health_sender when MQTT /status topic is received.
            // Payload: { type: "device_status", payload: { is_online, last_seen } }
            if (data.type === 'device_status') {
              const isOnline: boolean = data.payload.is_online ?? false;
              setDeviceStatus({ is_online: isOnline, last_seen: new Date().toISOString() });

              if (isOnline) {
                setIsInitialConnection(false);
                setIsControllerStatusKnown(true);
                if (!isInitialConnection) {
                  toast.success("Trạm Điều Khiển đã trực tuyến trở lại!");
                }
              } else {
                // Only mark as known offline if not initial connection
                if (!isInitialConnection) {
                  setIsControllerStatusKnown(true);
                  setFsmState("Offline");
                  setSensorData(prev => prev ? { ...prev, pump_status: {} as any } : prev);
                  toast.error("Đã ngắt kết nối Trạm Điều Khiển (LWT)!");
                }
              }
              return;
            }

            // ── alert messages ───────────────────────────────────────────────
            if (data.type === 'alert') {
              const alert = data.payload;

              // Legacy path: device_status sent via alert_sender
              // Backend sends title "Trạng thái Trạm Điều Khiển" for controller LWT
              if (alert.title === 'Trạng thái Trạm Điều Khiển') {
                const isOnline = alert.level === 'success';
                setDeviceStatus({ is_online: isOnline, last_seen: new Date().toISOString() });
                setIsControllerStatusKnown(true);
                if (isOnline) {
                  toast.success("Trạm Điều Khiển đã trực tuyến trở lại!");
                } else {
                  setFsmState("Offline");
                  setSensorData(prev => prev ? { ...prev, pump_status: {} as any } : prev);
                  toast.error("Đã ngắt kết nối Trạm Điều Khiển (LWT)!");
                }
                return;
              }

              // Sensor node LWT
              if (alert.title === 'Trạng thái Mạch Cảm Biến') {
                const onlineStatus = alert.level === 'success';
                setIsSensorOnline(onlineStatus);
                if (!onlineStatus) {
                  toast.error("Mạch Cảm Biến đã mất kết nối!");
                  setSensorData(prev => prev ? { ...prev, err_water: true, err_temp: true, err_ph: true, err_ec: true } : prev);
                  if (sensorTimeoutRef.current) clearTimeout(sensorTimeoutRef.current);
                } else {
                  toast.success("Mạch Cảm Biến đã trực tuyến!");
                  resetSensorTimeout();
                }
                return;
              }

              // FSM state update
              if (alert.level === 'FSM_UPDATE') {
                setFsmState(alert.message);
                return;
              }

              // Regular alert — add to event log
              setSystemEvents(prev => [alert, ...prev].slice(0, 50));
              switch (alert.level) {
                case 'critical': toast.error(`🚨 ${alert.title}\n${alert.message}`, { duration: 10000 }); break;
                case 'warning': toast.error(`⚠️ ${alert.title}\n${alert.message}`, { duration: 6000 }); break;
                case 'success': toast.success(`✅ ${alert.title}\n${alert.message}`, { duration: 5000 }); break;
                default: toast(`ℹ️ ${alert.title}`, { duration: 4000 }); break;
              }
              return;
            }

            // ── sensor_update ────────────────────────────────────────────────
            if (data.type === 'sensor_update') {
              const incomingPayload = data.payload.data || data.payload;

              setSensorData(prev => {
                if (!prev) return incomingPayload;
                return {
                  ...prev,
                  temp_value: incomingPayload.temp_value !== undefined ? incomingPayload.temp_value : prev.temp_value,
                  ec_value: incomingPayload.ec_value !== undefined ? incomingPayload.ec_value : prev.ec_value,
                  ph_value: incomingPayload.ph_value !== undefined ? incomingPayload.ph_value : prev.ph_value,
                  water_level: incomingPayload.water_level !== undefined ? incomingPayload.water_level : prev.water_level,
                  err_water: incomingPayload.err_water !== undefined ? incomingPayload.err_water : prev.err_water,
                  err_temp: incomingPayload.err_temp !== undefined ? incomingPayload.err_temp : prev.err_temp,
                  err_ph: incomingPayload.err_ph !== undefined ? incomingPayload.err_ph : prev.err_ph,
                  err_ec: incomingPayload.err_ec !== undefined ? incomingPayload.err_ec : prev.err_ec,
                  is_continuous: incomingPayload.is_continuous !== undefined ? incomingPayload.is_continuous : prev.is_continuous,
                  rssi: incomingPayload.rssi !== undefined ? incomingPayload.rssi : prev.rssi,
                  free_heap: incomingPayload.free_heap !== undefined ? incomingPayload.free_heap : prev.free_heap,
                  uptime: incomingPayload.uptime !== undefined ? incomingPayload.uptime : prev.uptime,
                };
              });

              setIsSensorOnline(true);
              resetSensorTimeout();
              return;
            }

            // ── device_health ────────────────────────────────────────────────
            // Sent by backend when it receives MQTT on AGITECH/+/controller/status
            if (data.type === 'device_health') {
              const healthData = data.payload;

              setControllerHealth({
                rssi: healthData.rssi,
                free_heap: healthData.free_heap,
                uptime: healthData.uptime_sec
              });

              setSensorData(prev => {
                if (!prev) return prev;
                return { ...prev, pump_status: healthData.pump_status };
              });

              // Mark controller online
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
      return {
        ...prevData,
        pump_status: {
          ...prevData.pump_status,
          [stateKey]: isNowOn
        }
      };
    });
  }, []);

  return (
    <DeviceContext.Provider value={{
      deviceId, sensorData, deviceStatus, isControllerStatusKnown, controllerHealth, fsmState, isLoading,
      updatePumpStatusOptimistically, settings, systemEvents, isSensorOnline
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
