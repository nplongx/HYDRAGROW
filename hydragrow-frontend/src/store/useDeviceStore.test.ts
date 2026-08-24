import { describe, it, expect, beforeEach, vi } from 'vitest';
import { useDeviceStore } from './useDeviceStore';
import { setItem } from '../platform/storage';

// Mock the platform storage so tests don't try to use real localStorage/Tauri storage
vi.mock('../platform/storage', () => ({
  setItem: vi.fn(() => Promise.resolve()),
  getItem: vi.fn(() => Promise.resolve(null)),
}));

describe('useDeviceStore', () => {
  beforeEach(() => {
    // Reset the store state before each test
    useDeviceStore.setState({
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
    });
    vi.clearAllMocks();
  });

  it('should initialize with default values', () => {
    const state = useDeviceStore.getState();
    expect(state.deviceId).toBeNull();
    expect(state.settings).toBeNull();
    expect(state.isMissingConfig).toBe(false);
    expect(state.sensorData).toBeNull();
    expect(state.deviceStatus).toEqual({ is_online: false, last_seen: '' });
    expect(state.isControllerStatusKnown).toBe(false);
    expect(state.controllerHealth).toBeNull();
    expect(state.fsmState).toBe('Offline');
    expect(state.systemEvents).toEqual([]);
    expect(state.isLoading).toBe(true);
    expect(state.isSensorOnline).toBe(false);
    expect(state.pwmPreferences).toEqual({});
    expect(state.tankAlert).toBeNull();
  });

  it('should set device ID', () => {
    useDeviceStore.getState().setDeviceId('device-123');
    expect(useDeviceStore.getState().deviceId).toBe('device-123');
  });

  it('should set settings', () => {
    const mockSettings = { someSetting: true } as any;
    useDeviceStore.getState().setSettings(mockSettings);
    expect(useDeviceStore.getState().settings).toBe(mockSettings);
  });

  it('should update sensor data with a value', () => {
    const mockData = { temperature: 25 } as any;
    useDeviceStore.getState().setSensorData(mockData);
    expect(useDeviceStore.getState().sensorData).toBe(mockData);
  });

  it('should update sensor data with an updater function', () => {
    const mockInitialData = { temperature: 20 } as any;
    useDeviceStore.setState({ sensorData: mockInitialData });

    useDeviceStore.getState().setSensorData((prev: any) => ({ ...prev, humidity: 60 }));

    expect(useDeviceStore.getState().sensorData).toEqual({ temperature: 20, humidity: 60 });
  });

  it('should save PWM preference to state and storage', () => {
    useDeviceStore.setState({ pwmPreferences: { pump1: 50 } });

    useDeviceStore.getState().savePwmPreference('pump2', 75);

    expect(useDeviceStore.getState().pwmPreferences).toEqual({ pump1: 50, pump2: 75 });
    expect(setItem).toHaveBeenCalledWith('pump_pwm_prefs', { pump1: 50, pump2: 75 });
  });

  it('should set system events with an updater function', () => {
    const sampleEvent1 = { id: 1, device_id: 'd1', level: 'info' as const, category: 'system' as const, message: 'Event 1', timestamp_ms: 100 };
    const sampleEvent2 = { id: 2, device_id: 'd1', level: 'info' as const, category: 'system' as const, message: 'Event 2', timestamp_ms: 200 };

    useDeviceStore.setState({ systemEvents: [sampleEvent1] });

    useDeviceStore.getState().setSystemEvents((prev) => [...prev, sampleEvent2]);

    expect(useDeviceStore.getState().systemEvents).toEqual([
      sampleEvent1,
      sampleEvent2
    ]);
  });

  it('should set simple properties correctly', () => {
    const state = useDeviceStore.getState();

    state.setIsMissingConfig(true);
    expect(useDeviceStore.getState().isMissingConfig).toBe(true);

    state.setIsControllerStatusKnown(true);
    expect(useDeviceStore.getState().isControllerStatusKnown).toBe(true);

    const healthSnapshot = {
      device_id: 'd1',
      free_heap: 200000,
      uptime_sec: 3600,
      rssi: -60,
      health_score_percent: 100,
      fsm_state_display: 'Monitoring',
      log_drop_count: 0,
      firmware_version: 'v1.0.0',
      matrix_update_count: 10,
      matrix_is_warm: true,
      timestamp_ms: 1000,
    };
    state.setControllerHealth(healthSnapshot);
    expect(useDeviceStore.getState().controllerHealth).toEqual(healthSnapshot);

    state.setFsmState('Active');
    expect(useDeviceStore.getState().fsmState).toBe('Active');

    state.setIsLoading(false);
    expect(useDeviceStore.getState().isLoading).toBe(false);

    state.setIsSensorOnline(true);
    expect(useDeviceStore.getState().isSensorOnline).toBe(true);

    const mockAlert = { message: 'Alert!' } as any;
    state.setTankAlert(mockAlert);
    expect(useDeviceStore.getState().tankAlert).toBe(mockAlert);
  });
});
