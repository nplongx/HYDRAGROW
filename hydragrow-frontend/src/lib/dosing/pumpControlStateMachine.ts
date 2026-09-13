// hydragrow-frontend/src/lib/dosing/pumpControlStateMachine.ts

/**
 * State machine thuần cho 1 card điều khiển bơm dosing (AdvancedDeviceControl).
 *
 * States: 'idle' | 'running' | 'locked' — hiển thị trực tiếp bằng
 * PumpControlStatePill (xem pumpControlStatePill.tsx).
 *
 * 'running' luôn thắng mọi lý do khoá: nếu bơm đang thực sự bật (do lệnh
 * thủ công trước đó, hoặc do MIMO tự động bật), khoá chỉ áp dụng cho lệnh
 * BẬT tiếp theo — không có ý nghĩa hiển thị "đã khoá" trong khi bơm đang
 * chạy thật. Đây là lý do currentStatus được kiểm tra trước isAutoMode
 * trong handleToggle() gốc (dòng 93-102 của AdvancedDeviceControl.tsx) —
 * state machine này chỉ tường minh hoá logic đã đúng, không đổi hành vi.
 */
export type PumpLockReason = 'auto_mode' | 'emergency' | 'interlock';

export interface PumpControlInputs {
  currentStatus: boolean;
  isAutoMode: boolean;
  isEmergency: boolean;
  lockedByPumpId?: string;
}

export interface PumpControlState {
  state: 'idle' | 'running' | 'locked';
  reason: PumpLockReason | null;
}

export function derivePumpControlState(inputs: PumpControlInputs): PumpControlState {
  if (inputs.currentStatus) {
    return { state: 'running', reason: null };
  }
  if (inputs.isAutoMode) {
    return { state: 'locked', reason: 'auto_mode' };
  }
  if (inputs.isEmergency) {
    return { state: 'locked', reason: 'emergency' };
  }
  if (inputs.lockedByPumpId) {
    return { state: 'locked', reason: 'interlock' };
  }
  return { state: 'idle', reason: null };
}

/** Copy hiển thị cho từng lý do khoá — dùng chung giữa pill (tooltip) và banner. */
export const LOCK_REASON_COPY: Record<PumpLockReason, string> = {
  auto_mode: 'Tự động (MIMO) đang quản lý bơm này',
  emergency: 'Đang ngắt do sự cố an toàn',
  interlock: 'Đã khoá vì thiết bị xung khắc đang chạy — tránh trung hoà lẫn nhau',
};
