import { describe, it, expect, beforeEach, vi } from "vitest";
import { useDeviceStore } from "./useDeviceStore";
import { setItem } from "../platform/storage";

vi.mock("../platform/storage", () => ({
  getItem: vi.fn(() => Promise.resolve(null)),
  setItem: vi.fn(() => Promise.resolve()),
}));

describe("useDeviceStore", () => {
  beforeEach(() => {
    useDeviceStore.setState({ pwmPreferences: {} });
    vi.clearAllMocks();
  });

  it("contains only identity compatibility and local UI preference state", () => {
    const state = useDeviceStore.getState();
    expect(state.pwmPreferences).toEqual({});
    expect("sensorData" in state).toBe(false);
    expect("settings" in state).toBe(false);
    expect("systemEvents" in state).toBe(false);
  });

  it("saves PWM preference to local storage", () => {
    useDeviceStore.setState({ pwmPreferences: { pump1: 50 } });
    useDeviceStore.getState().savePwmPreference("pump2", 75);

    expect(useDeviceStore.getState().pwmPreferences).toEqual({
      pump1: 50,
      pump2: 75,
    });
    expect(setItem).toHaveBeenCalledWith("pump_pwm_prefs", {
      pump1: 50,
      pump2: 75,
    });
  });
});
