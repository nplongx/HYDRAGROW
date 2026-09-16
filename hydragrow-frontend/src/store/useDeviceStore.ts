import { create } from 'zustand';
import { getItem, setItem } from '../platform/storage';

interface DeviceStoreState {
  // Explicitly local UI preference; not device-domain truth.
  pwmPreferences: Record<string, number>;

  setPwmPreferences: (prefs: Record<string, number>) => void;
  savePwmPreference: (pumpId: string, pwm: number) => void;
}

const PWM_PREFS_STORE_KEY = 'pump_pwm_prefs';

export const useDeviceStore = create<DeviceStoreState>((set, get) => ({
  pwmPreferences: {},

  setPwmPreferences: (pwmPreferences) => set({ pwmPreferences }),

  savePwmPreference: (pumpId: string, pwm: number) => {
    const updated = { ...get().pwmPreferences, [pumpId]: pwm };
    set({ pwmPreferences: updated });
    void setItem(PWM_PREFS_STORE_KEY, updated).catch(() => {});
  },
}));

void getItem<Record<string, number>>(PWM_PREFS_STORE_KEY)
  .then((stored) => {
    if (stored) useDeviceStore.setState({ pwmPreferences: stored });
  })
  .catch(() => {});

if (typeof window !== 'undefined') {
  (window as any).useDeviceStore = useDeviceStore;
}
