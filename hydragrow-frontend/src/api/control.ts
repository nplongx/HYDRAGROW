import { apiGet, apiPost } from '../lib/apiClient';

export type CommandLifecycle =
  | 'REQUESTED'
  | 'SENT'
  | 'ACKNOWLEDGED'
  | 'CONFIRMED'
  | 'REJECTED'
  | 'FAILED'
  | 'TIMEOUT'
  | 'UNKNOWN';

export interface CommandRecord {
  command_id: string;
  device_id: string;
  action: string;
  pump_id: string | null;
  requested_state: boolean | null;
  requested_pwm: number | null;
  lifecycle: CommandLifecycle;
}

export interface ControlCommandRequest {
  target: 'all';
  action: string;
  params: {
    pump_id: string;
    duration_sec: number | null;
    pwm: number | null;
  };
  command_metadata: {
    action: string;
    pump_id: string;
    duration_sec: number | null;
    pwm: number | null;
    dangerous: boolean;
  };
}

export function buildControlCommandRequest(
  action: string,
  pumpId: string,
  durationSec: number | undefined,
  pwm: number | undefined,
  dangerous: boolean,
): ControlCommandRequest {
  return {
    target: 'all',
    action,
    params: { pump_id: pumpId, duration_sec: durationSec ?? null, pwm: pwm ?? null },
    command_metadata: { action, pump_id: pumpId, duration_sec: durationSec ?? null, pwm: pwm ?? null, dangerous },
  };
}

export interface PrivilegedTokenResponse {
  token: string;
  expires_at: number;
}

export interface ControlCommandResponse {
  status: string;
  command_id: string;
  lifecycle: CommandLifecycle;
  device_id: string;
  target: string;
  action: string;
  pump: string;
  duration_sec: number | null;
  pwm: number | null;
  published_at: number;
}

export const controlApi = {
  listCommands: (deviceId: string, signal?: AbortSignal) =>
    apiGet<{ data?: CommandRecord[] }>(`/devices/${encodeURIComponent(deviceId)}/control/commands`, { signal })
      .then((response) => response.data ?? []),
  issuePrivilegedToken: (deviceId: string) =>
    apiPost<PrivilegedTokenResponse, { action_class: 'dangerous_control' }>(
      `/devices/${encodeURIComponent(deviceId)}/control/privileged-token`,
      { action_class: 'dangerous_control' },
      { 'X-User-Confirmed': 'true' },
    ),
  send: (
    deviceId: string,
    payload: ControlCommandRequest,
    confirmed: boolean,
    privilegedToken?: string,
  ) =>
    apiPost<ControlCommandResponse, ControlCommandRequest>(
      `/devices/${encodeURIComponent(deviceId)}/control`,
      payload,
      {
        ...(confirmed ? { 'X-User-Confirmed': 'true' } : {}),
        ...(privilegedToken ? { 'X-Privileged-Token': privilegedToken } : {}),
      },
    ),
};
