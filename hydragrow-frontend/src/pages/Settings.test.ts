import { describe, it, expect } from 'vitest';
import { build_full_unified_payload_json } from '../../gleam_core/build/dev/javascript/gleam_core/settings/payload.mjs';

function buildHeaders(
  apiKey: string,
  extraHeaders?: Record<string, string>
): Record<string, string> {
  return {
    'Content-Type': 'application/json',
    'X-API-Key': apiKey,
    ...extraHeaders,
  };
}

describe('buildHeaders', () => {
  it('merges X-User-Confirmed into headers', () => {
    const headers = buildHeaders('key123', { 'X-User-Confirmed': 'true' });
    expect(headers['X-User-Confirmed']).toBe('true');
  });

  it('does not include X-User-Confirmed when not provided', () => {
    const headers = buildHeaders('key123');
    expect(headers['X-User-Confirmed']).toBeUndefined();
  });
});

describe('build_full_unified_payload_json', () => {
  it('encodes per-pump step ratios and pump min PWMs correctly into JSON', () => {
    const jsonStr = build_full_unified_payload_json(
      'dev-test',
      'auto',
      true,
      false,
      '1.6',
      '0.05',
      '6.2',
      '0.2',
      '10',
      '50',
      '20',
      '80',
      '90',
      '5',
      true,
      true,
      false,
      '5.0',
      false,
      '0 0 7 * * SUN',
      '10.0',
      '10000',
      '180000',
      '30',
      '15000',
      '60000',
      '0.5',
      '3.0',
      '4.0',
      '8.0',
      '0.5',
      '0.3',
      '50',
      '200',
      '60',
      '10',
      '3',
      '3',
      '120',
      '120',
      '15',
      '35',
      '0.05',
      '0.1',
      '0.5',
      '0.1',
      '0.2',
      '0.2',
      '5',
      '5',
      '0.4',
      '0.1',
      '0.35', // ec_a_step_ratio
      '0.45', // ec_b_step_ratio
      '0.15', // ph_up_step_ratio
      '0.25', // ph_down_step_ratio
      '1.2',  // pump_a_cap
      '1.3',  // pump_b_cap
      '1.1',  // pump_ph_up_cap
      '1.4',  // pump_ph_down_cap
      '50',   // dosing_pwm
      '60',
      '100',
      '20',   // dosing_min_pwm
      '25',   // pump_a_min_pwm
      '30',   // pump_b_min_pwm
      '15',   // pump_ph_up_min_pwm
      '18',   // pump_ph_down_min_pwm
      '500',
      '500',
      '1.0',
      '20',
      '3000',
      '3600',
      '300',
      '2.5',
      '1.428',
      '880.0',
      '0.0',
      '0.0',
      '0.02',
      '5000',
      '15',
      true,
      true,
      true,
      true,
      '2026-08-31T00:00:00.000Z'
    );

    const parsed = JSON.parse(jsonStr);
    expect(parsed.dosing_calibration.ec_a_step_ratio).toBe(0.35);
    expect(parsed.dosing_calibration.ec_b_step_ratio).toBe(0.45);
    expect(parsed.dosing_calibration.ph_up_step_ratio).toBe(0.15);
    expect(parsed.dosing_calibration.ph_down_step_ratio).toBe(0.25);
    expect(parsed.dosing_calibration.pump_a_min_pwm_percent).toBe(25);
    expect(parsed.dosing_calibration.pump_b_min_pwm_percent).toBe(30);
    expect(parsed.dosing_calibration.pump_ph_up_min_pwm_percent).toBe(15);
    expect(parsed.dosing_calibration.pump_ph_down_min_pwm_percent).toBe(18);
    expect(parsed.dosing_calibration.pump_a_capacity_ml_per_sec).toBe(1.2);
    expect(parsed.dosing_calibration.pump_b_capacity_ml_per_sec).toBe(1.3);
  });
});
