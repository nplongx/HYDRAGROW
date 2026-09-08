import { describe, expect, it } from 'vitest';
import registry from './device-config-keys.json';

interface ConfigKeyDef {
  key: string;
  valueType: 'float' | 'integer';
  min: number;
  max: number;
  unit: string;
  step: number;
  label: string;
  sourceGroup: string;
  defaultVal: number;
}

describe('device-config-keys.json', () => {
  it('lists exactly the keys DeviceConfig (backend) actually supports', () => {
    const keys = (registry as ConfigKeyDef[]).map((d) => d.key).sort();
    expect(keys).toEqual([
      'delay_between_a_and_b_sec',
      'ec_target',
      'ec_tolerance',
      'ph_target',
      'ph_tolerance',
    ]);
  });

  it('every entry has min < max and defaultVal within [min, max]', () => {
    for (const def of registry as ConfigKeyDef[]) {
      expect(def.min).toBeLessThan(def.max);
      expect(def.defaultVal).toBeGreaterThanOrEqual(def.min);
      expect(def.defaultVal).toBeLessThanOrEqual(def.max);
    }
  });

  it('has no duplicate keys', () => {
    const keys = (registry as ConfigKeyDef[]).map((d) => d.key);
    expect(new Set(keys).size).toBe(keys.length);
  });
});
