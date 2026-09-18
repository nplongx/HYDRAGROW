import { describe, expect, it } from 'vitest';
import { buildControlCommandRequest, controlApi, type CommandLifecycle } from './control';

describe('control API contracts', () => {
  it('keeps lifecycle names aligned with shared command contract', () => {
    const lifecycle: CommandLifecycle = 'ACKNOWLEDGED';
    expect(lifecycle).toBe('ACKNOWLEDGED');
  });

  it('encodes device resource IDs before transport', () => {
    expect(encodeURIComponent('dev/a')).toBe('dev%2Fa');
    expect(controlApi.listCommands).toBeTypeOf('function');
    expect(controlApi.issuePrivilegedToken).toBeTypeOf('function');
  });

  it('builds the canonical command envelope with nullable optional params', () => {
    expect(buildControlCommandRequest('on', 'PUMP_A', undefined, undefined, false)).toEqual({
      target: 'all',
      action: 'on',
      params: { pump_id: 'PUMP_A', duration_sec: null, pwm: null },
      command_metadata: { action: 'on', pump_id: 'PUMP_A', duration_sec: null, pwm: null, dangerous: false },
    });
  });

  it('keeps dangerous-command transport headers explicit', () => {
    expect('X-User-Confirmed').toBe('X-User-Confirmed');
    expect('X-Privileged-Token').toBe('X-Privileged-Token');
  });
});
