import { create } from 'zustand';
import { SensorData, StatusPayload, AppSettings, TankAlert, UnifiedSystemLog } from '../types/models';
import { setItem } from '../platform/storage';

export interface ControllerHealth {
  device_id: string;
  free_heap: number;
  uptime_sec: number;
  rssi: number;
  health_score_percent: number;
  fsm_state_display: string;
  log_drop_count: number;
  firmware_version: string;
  diagnostics?: unknown;
}

interface DeviceState {
  // --- STATES ---
  deviceId: string | null;
  settings: AppSettings | null;
  isMissingConfig: boolean;
  sensorData: SensorData | null;
  deviceStatus: StatusPayload;
  isControllerStatusKnown: boolean;
  controllerHealth: ControllerHealth | null;
  fsmState: string;
  systemEvents: UnifiedSystemLog[];
  isLoading: boolean;
  isSensorOnline: boolean;
  pwmPreferences: Record<string, number>;
  tankAlert: TankAlert | null; // <-- Thêm state
  setTankAlert: (tankAlert: TankAlert | null) => void;

  // --- ACTIONS ---
  setDeviceId: (id: string | null) => void;
  setSettings: (settings: AppSettings | null) => void;
  setIsMissingConfig: (missing: boolean) => void;
  setSensorData: (data: SensorData | null | ((prev: SensorData | null) => SensorData | null)) => void;
  setDeviceStatus: (status: StatusPayload | ((prev: StatusPayload) => StatusPayload)) => void;
  setIsControllerStatusKnown: (known: boolean) => void;
  setControllerHealth: (health: ControllerHealth | null) => void;
  setFsmState: (state: string) => void;
  setSystemEvents: (events: UnifiedSystemLog[] | ((prev: UnifiedSystemLog[]) => UnifiedSystemLog[])) => void;
  setIsLoading: (loading: boolean) => void;
  setIsSensorOnline: (online: boolean) => void;
  setPwmPreferences: (prefs: Record<string, number>) => void;
  savePwmPreference: (pumpId: string, pwm: number) => void;
}

const PWM_PREFS_STORE_KEY = 'pump_pwm_prefs';

export const useDeviceStore = create<DeviceState>((set, get) => ({
  deviceId: null,
  settings: null,
  isMissingConfig: false,
  sensorData: null,
  deviceStatus: { is_online: false, last_seen: '' },
  isControllerStatusKnown: false,
  controllerHealth: null,
  fsmState: 'Offline',
  systemEvents: [],
  isLoading: true,
  isSensorOnline: false,
  pwmPreferences: {},
  tankAlert: null,
  setTankAlert: (tankAlert) => set({ tankAlert }),

  setDeviceId: (deviceId) => set({ deviceId }),
  setSettings: (settings) => set({ settings }),
  setIsMissingConfig: (isMissingConfig) => set({ isMissingConfig }),
  setSensorData: (updater) =>
    set((state) => ({
      sensorData: typeof updater === 'function' ? updater(state.sensorData) : updater,
    })),
  setDeviceStatus: (updater) =>
    set((state) => ({
      deviceStatus: typeof updater === 'function' ? updater(state.deviceStatus) : updater,
    })),
  setIsControllerStatusKnown: (isControllerStatusKnown) => set({ isControllerStatusKnown }),
  setControllerHealth: (controllerHealth) => set({ controllerHealth }),
  setFsmState: (fsmState) => set({ fsmState }),
  setSystemEvents: (updater) =>
    set((state) => ({
      systemEvents: typeof updater === 'function' ? updater(state.systemEvents) : updater,
    })),
  setIsLoading: (isLoading) => set({ isLoading }),
  setIsSensorOnline: (isSensorOnline) => set({ isSensorOnline }),
  setPwmPreferences: (pwmPreferences) => set({ pwmPreferences }),

  savePwmPreference: (pumpId: string, pwm: number) => {
    const updated = { ...get().pwmPreferences, [pumpId]: pwm };
    set({ pwmPreferences: updated });
    setItem(PWM_PREFS_STORE_KEY, updated).catch(() => {});
  },
}));
