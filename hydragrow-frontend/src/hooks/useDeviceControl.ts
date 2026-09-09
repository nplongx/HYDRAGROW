import { useState, useCallback } from "react";
import { useDeviceStore } from "../store/useDeviceStore";
import toast from "react-hot-toast";
import { httpFetch } from "../platform/http";
import { isTauriRuntime } from "../platform/settings";
import { invoke } from "@tauri-apps/api/core";

export const INTERLOCK_PAIRS: Record<string, string> = {
  WATER_PUMP_IN: "WATER_PUMP_OUT",
  WATER_PUMP_OUT: "WATER_PUMP_IN",
  PH_UP: "PH_DOWN",
  PH_DOWN: "PH_UP",
};

const INTERLOCK_LABELS: Record<string, string> = {
  WATER_PUMP_IN: "Van cấp nước",
  WATER_PUMP_OUT: "Bơm xả thoát",
  PH_UP: "Bơm pH Up",
  PH_DOWN: "Bơm pH Down",
};

const isPumpRunning = (
  pumps: Record<string, boolean> | undefined,
  pumpId: string,
): boolean => {
  switch (pumpId) {
    case "WATER_PUMP_IN":
      return Boolean(pumps?.water_pump_in);
    case "WATER_PUMP_OUT":
      return Boolean(pumps?.water_pump_out);
    case "PH_UP":
      return Boolean(pumps?.ph_up);
    case "PH_DOWN":
      return Boolean(pumps?.ph_down);
    default:
      return false;
  }
};

export const ensureInterlock = async (
  pumpId: string,
  action: string,
): Promise<string | null> => {
  const partnerId = INTERLOCK_PAIRS[pumpId];
  if (action !== "on" || !partnerId) return null;

  const pumps = useDeviceStore.getState().sensorData?.pump_status;
  if (isPumpRunning(pumps as Record<string, boolean> | undefined, partnerId)) {
    return `⛔ XUNG ĐỘT AN TOÀN: Không thể bật ${INTERLOCK_LABELS[pumpId]} khi ${INTERLOCK_LABELS[partnerId]} đang chạy.`;
  }

  if (isTauriRuntime()) {
    try {
      await invoke("check_valve_safety", { targetPump: pumpId, isOn: true });
    } catch (error) {
      return String(error);
    }
  }
  return null;
};

const isDangerousCommand = (pumpId: string, action: string, pwm?: number) => {
  const dosingPumps = ["A", "PUMP_A", "B", "PUMP_B", "PH_UP", "PH_DOWN"];
  return (
    action === "force_on" ||
    action === "reset_fault" ||
    action === "set_pwm" ||
    typeof pwm === "number" ||
    dosingPumps.includes(pumpId.toUpperCase())
  );
};

export const useDeviceControl = (deviceId: string) => {
  const settings = useDeviceStore((state) => state.settings);

  const [isProcessing, setIsProcessing] = useState(false);
  const [processingPumpIds, setProcessingPumpIds] = useState<
    Record<string, boolean>
  >({});
  const [commandStatus, setCommandStatus] = useState<Record<string, string>>(
    {},
  );

  const cooldownPump = useCallback((pumpId: string, status: string) => {
    setProcessingPumpIds((prev) => ({ ...prev, [pumpId]: true }));
    setCommandStatus((prev) => ({ ...prev, [pumpId]: status }));
    setTimeout(() => {
      setProcessingPumpIds((prev) => ({ ...prev, [pumpId]: false }));
    }, 1500);
  }, []);

  const sendCommand = useCallback(
    async (
      pumpId: string,
      action: string,
      duration_sec?: number,
      pwm?: number,
      forceConfirmed = false,
    ) => {
      if (!deviceId || !settings?.backend_url) {
        toast.error("Chưa cấu hình máy chủ!");
        return false;
      }
      const interlockError = await ensureInterlock(pumpId, action);
      if (interlockError) {
        setCommandStatus((prev) => ({ ...prev, [pumpId]: "safety_blocked" }));
        toast.error(interlockError);
        return false;
      }
      const dangerous = isDangerousCommand(pumpId, action, pwm);
      if (dangerous && !forceConfirmed) {
        const confirmed = window.confirm(
          `Lệnh nguy hiểm: ${action} cho ${pumpId}. Xác nhận thực thi?`,
        );
        if (!confirmed) return false;
      }
      const isConfirmed = dangerous || forceConfirmed;
      setIsProcessing(true);
      cooldownPump(pumpId, "sending");
      try {
        const payload = {
          target: "all",
          action: action,
          params: {
            pump_id: pumpId,
            duration_sec: duration_sec || null,
            pwm: pwm ?? null,
          },
          command_metadata: {
            action,
            pump_id: pumpId,
            duration_sec: duration_sec ?? null,
            pwm: pwm ?? null,
            dangerous,
          },
        };
        const res = await httpFetch(
          `${settings.backend_url}/api/devices/${deviceId}/control`,
          {
            method: "POST",
            headers: {
              "Content-Type": "application/json",
              "X-API-Key": settings.api_key || "",
              ...(isConfirmed ? { "X-User-Confirmed": "true" } : {}),
            },
            body: JSON.stringify(payload),
          },
        );
        if (res.ok) {
          setCommandStatus((prev) => ({ ...prev, [pumpId]: "accepted" }));
          toast.success(`Đã gửi lệnh: ${action} -> ${pumpId}`);
          return true;
        } else {
          const rejectedStatus =
            res.status === 429 ? "rate_limited" : `HTTP ${res.status}`;
          setCommandStatus((prev) => ({ ...prev, [pumpId]: rejectedStatus }));
          toast.error(`Từ chối: ${rejectedStatus}`);
          return false;
        }
      } catch {
        setCommandStatus((prev) => ({ ...prev, [pumpId]: "network_error" }));
        toast.error("Lỗi mạng khi gửi lệnh!");
        return false;
      } finally {
        setIsProcessing(false);
      }
    },
    [deviceId, settings, cooldownPump],
  );

  const togglePump = (pumpId: string, action: "on" | "off") =>
    sendCommand(pumpId, action);
  const forceOn = (pumpId: string, durationSec: number, pwmValue?: number) =>
    sendCommand(pumpId, "force_on", durationSec, pwmValue);
  const setPwm = (pumpId: string, pwmValue: number, durationSec?: number) =>
    sendCommand(pumpId, "set_pwm", durationSec, pwmValue);
  const resetFault = () => sendCommand("ALL", "reset_fault");
  const emergencyStop = () => sendCommand("ALL", "emergency_stop", undefined, undefined, true);

  return {
    isProcessing,
    processingPumpIds,
    commandStatus,
    togglePump,
    forceOn,
    setPwm,
    resetFault,
    emergencyStop,
  };
};
