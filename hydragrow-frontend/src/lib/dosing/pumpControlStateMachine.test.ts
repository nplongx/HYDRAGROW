// hydragrow-frontend/src/lib/dosing/pumpControlStateMachine.test.ts
import { describe, expect, it } from 'vitest';
import { derivePumpControlState, type PumpControlInputs } from './pumpControlStateMachine';

const base: PumpControlInputs = {
  currentStatus: false,
  isAutoMode: false,
  isEmergency: false,
  lockedByPumpId: undefined,
};

describe('derivePumpControlState', () => {
  it('idle: tắt, không auto, không emergency, không interlock', () => {
    expect(derivePumpControlState(base)).toEqual({
      state: 'idle',
      reason: null,
    });
  });

  it('running: đang bật bất kể các cờ khác', () => {
    expect(derivePumpControlState({ ...base, currentStatus: true })).toEqual({
      state: 'running',
      reason: null,
    });
  });

  it('running thắng cả khi isAutoMode true (bơm đang chạy do tự động)', () => {
    expect(
      derivePumpControlState({ ...base, currentStatus: true, isAutoMode: true }),
    ).toEqual({ state: 'running', reason: null });
  });

  it('locked: isAutoMode khi đang tắt', () => {
    expect(derivePumpControlState({ ...base, isAutoMode: true })).toEqual({
      state: 'locked',
      reason: 'auto_mode',
    });
  });

  it('locked: emergency khi đang tắt', () => {
    expect(derivePumpControlState({ ...base, isEmergency: true })).toEqual({
      state: 'locked',
      reason: 'emergency',
    });
  });

  it('locked: interlock (bị khoá bởi bơm xung khắc) khi đang tắt', () => {
    expect(
      derivePumpControlState({ ...base, lockedByPumpId: 'PH_UP' }),
    ).toEqual({ state: 'locked', reason: 'interlock' });
  });

  it('thứ tự ưu tiên reason khi tắt: auto_mode > emergency > interlock', () => {
    expect(
      derivePumpControlState({
        ...base,
        isAutoMode: true,
        isEmergency: true,
        lockedByPumpId: 'PH_UP',
      }),
    ).toEqual({ state: 'locked', reason: 'auto_mode' });
  });
});
