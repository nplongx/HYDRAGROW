import { useState, useCallback, useEffect } from "react";
import { useQuery } from "@tanstack/react-query";
import { useStationContext } from "../contexts/StationContext";
import toast from "react-hot-toast";
import { isTauriRuntime } from "../platform/settings";
import { invoke } from "@tauri-apps/api/core";
import { buildControlCommandRequest, controlApi } from "../api/control";
import { useDeviceTelemetry } from "./useDeviceTelemetry";
import type { PumpStatus } from "../types/models";
import { queryKeys } from "../api/queryKeys";

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

const readPumpState = (
  pumps: Partial<PumpStatus> | undefined,
  pumpId: string,
): boolean | undefined => {
  if (!pumps) return undefined;
  switch (pumpId) {
    case "WATER_PUMP_IN":
      return pumps.water_pump_in;
    case "WATER_PUMP_OUT":
      return pumps.water_pump_out;
    case "PH_UP":
      return pumps.ph_up;
    case "PH_DOWN":
      return pumps.ph_down;
    default:
      return undefined;
  }
};

export const ensureInterlock = async (
  pumpId: string,
  action: string,
  pumpsOverride?: Partial<PumpStatus>,
): Promise<string | null> => {
  const partnerId = INTERLOCK_PAIRS[pumpId];
  if (action !== "on" || !partnerId) return null;

  const pumps = pumpsOverride;
  const partnerState = readPumpState(pumps, partnerId);
  if (partnerState === undefined) {
    return `⛔ KHÔNG XÁC ĐỊNH TRẠNG THÁI AN TOÀN: Không thể bật ${INTERLOCK_LABELS[pumpId]} khi trạng thái ${INTERLOCK_LABELS[partnerId]} chưa được xác nhận.`;
  }
  if (partnerState) {
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
  const { selectedDeviceId } = useStationContext();
  const activeDeviceId = selectedDeviceId;
  void deviceId; // Transitional callers may still pass the store mirror; StationContext is authoritative.
  const { data: telemetry } = useDeviceTelemetry(activeDeviceId);

  const [isProcessing, setIsProcessing] = useState(false);
  const [processingPumpIds, setProcessingPumpIds] = useState<
    Record<string, boolean>
  >({});
  const [commandStatus, setCommandStatus] = useState<Record<string, string>>(
    {},
  );
  const [commandIds, setCommandIds] = useState<Record<string, string>>({});

  // All control components share one React Query cache. Previously every
  // useDeviceControl() instance fetched the same command history independently.
  const commandHistoryQuery = useQuery({
    queryKey: activeDeviceId
      ? queryKeys.controlCommands(activeDeviceId)
      : ["control-commands", null],
    queryFn: ({ signal }) => controlApi.listCommands(activeDeviceId!, signal),
    enabled: Boolean(activeDeviceId),
    staleTime: 5_000,
    refetchOnWindowFocus: false,
  });

  useEffect(() => {
    setCommandIds({});
    setCommandStatus({});
    setProcessingPumpIds({});
    const records = commandHistoryQuery.data;
    if (!records) return;
    const ids: Record<string, string> = {};
    const statuses: Record<string, string> = {};
    for (const record of records) {
      if (record.pump_id && !ids[record.pump_id]) {
        ids[record.pump_id] = record.command_id;
        statuses[record.pump_id] = record.lifecycle;
      }
    }
    setCommandIds(ids);
    setCommandStatus(statuses);
  }, [activeDeviceId, commandHistoryQuery.data]);

  useEffect(() => {
    const onLifecycle = (event: Event) => {
      const detail = (event as CustomEvent).detail;
      if (!detail || detail.device_id !== activeDeviceId || !detail.command_id)
        return;
      const pumpId = Object.keys(commandIds).find(
        (key) => commandIds[key] === detail.command_id,
      );
      if (!pumpId) return;
      const lifecycle = String(detail.lifecycle || "").toUpperCase();
      setCommandStatus((prev) => ({ ...prev, [pumpId]: lifecycle }));
      if (
        ["CONFIRMED", "REJECTED", "FAILED", "TIMEOUT", "UNKNOWN"].includes(
          lifecycle,
        )
      ) {
        setProcessingPumpIds((prev) => ({ ...prev, [pumpId]: false }));
      }
    };
    window.addEventListener("hydragrow:command-lifecycle", onLifecycle);
    return () =>
      window.removeEventListener("hydragrow:command-lifecycle", onLifecycle);
  }, [activeDeviceId, commandIds]);

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
      if (!activeDeviceId) {
        toast.error("Chưa cấu hình máy chủ!");
        return false;
      }
      const interlockError = await ensureInterlock(
        pumpId,
        action,
        telemetry?.actuator?.pump_status,
      );
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
      cooldownPump(pumpId, "REQUESTED");
      try {
        const payload = buildControlCommandRequest(
          action,
          pumpId,
          duration_sec,
          pwm,
          dangerous,
        );
         const privilegedToken = dangerous
           ? (await controlApi.issuePrivilegedToken(activeDeviceId)).token
           : undefined;
        const body = await controlApi.send(
          activeDeviceId,
          payload,
          isConfirmed,
           privilegedToken,
        );
        const commandId = body.command_id;
        if (commandId) {
          setCommandIds((prev) => ({ ...prev, [pumpId]: commandId }));
        }
        setCommandStatus((prev) => ({ ...prev, [pumpId]: "SENT" }));
        toast.success(`Đã gửi lệnh: ${action} -> ${pumpId}`);
        return true;
      } catch (error) {
        const status = (error as Error & { status?: number })?.status;
        const lifecycle = status === 429 ? "REJECTED" : "UNKNOWN";
        setCommandStatus((prev) => ({ ...prev, [pumpId]: lifecycle }));
        toast.error(
          status ? `Từ chối: ${lifecycle}` : "Lỗi mạng khi gửi lệnh!",
        );
        return false;
      } finally {
        setIsProcessing(false);
      }
    },
    [activeDeviceId, telemetry, cooldownPump],
  );

  const togglePump = (pumpId: string, action: "on" | "off") =>
    sendCommand(pumpId, action);
  const forceOn = (pumpId: string, durationSec: number, pwmValue?: number) =>
    sendCommand(pumpId, "force_on", durationSec, pwmValue);
  const setPwm = (pumpId: string, pwmValue: number, durationSec?: number) =>
    sendCommand(pumpId, "set_pwm", durationSec, pwmValue);
  const resetFault = () => sendCommand("ALL", "reset_fault");
  const emergencyStop = () =>
    sendCommand("ALL", "emergency_stop", undefined, undefined, true);

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
