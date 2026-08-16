import * as $json from "../../gleam_json/gleam/json.mjs";
import * as $float from "../../gleam_stdlib/gleam/float.mjs";
import * as $int from "../../gleam_stdlib/gleam/int.mjs";
import { Ok, toList } from "../gleam.mjs";

export function parse_float_or(val, fallback) {
  let $ = $float.parse(val);
  if ($ instanceof Ok) {
    let v = $[0];
    return v;
  } else {
    let $1 = $int.parse(val);
    if ($1 instanceof Ok) {
      let v = $1[0];
      return $int.to_float(v);
    } else {
      return fallback;
    }
  }
}

export function parse_int_or(val, fallback) {
  let $ = $int.parse(val);
  if ($ instanceof Ok) {
    let v = $[0];
    return v;
  } else {
    let $1 = $float.parse(val);
    if ($1 instanceof Ok) {
      let v = $1[0];
      return $float.round(v);
    } else {
      return fallback;
    }
  }
}

function encode_device_config(
  dev_id,
  control_mode,
  is_enabled,
  ec_target,
  ec_tolerance,
  ph_target,
  ph_tolerance,
  delay_a_b,
  ts
) {
  return $json.object(
    toList([
      ["device_id", $json.string(dev_id)],
      ["control_mode", $json.string(control_mode)],
      ["is_enabled", $json.bool(is_enabled)],
      ["ec_target", $json.float(parse_float_or(ec_target, 1.5))],
      ["ec_tolerance", $json.float(parse_float_or(ec_tolerance, 0.05))],
      ["ph_target", $json.float(parse_float_or(ph_target, 6.0))],
      ["ph_tolerance", $json.float(parse_float_or(ph_tolerance, 0.5))],
      ["delay_between_a_and_b_sec", $json.int(parse_int_or(delay_a_b, 10))],
      ["last_updated", $json.string(ts)],
    ]),
  );
}

function encode_water_config(
  dev_id,
  tank_height,
  water_min,
  water_target,
  water_max,
  water_tolerance,
  auto_refill,
  auto_drain,
  cron_str,
  misting_on,
  misting_off,
  ts
) {
  return $json.object(
    toList([
      ["device_id", $json.string(dev_id)],
      ["tank_height", $json.float(parse_float_or(tank_height, 50.0))],
      ["water_level_min", $json.float(parse_float_or(water_min, 20.0))],
      ["water_level_target", $json.float(parse_float_or(water_target, 80.0))],
      ["water_level_max", $json.float(parse_float_or(water_max, 90.0))],
      ["water_level_drain", $json.float(5.0)],
      [
        "water_level_tolerance",
        $json.float(parse_float_or(water_tolerance, 5.0)),
      ],
      ["auto_refill_enabled", $json.bool(auto_refill)],
      ["auto_drain_overflow", $json.bool(auto_drain)],
      ["auto_dilute_enabled", $json.bool(false)],
      ["dilute_drain_amount_cm", $json.float(5.0)],
      ["scheduled_water_change_enabled", $json.bool(false)],
      ["water_change_cron", $json.string(cron_str)],
      ["scheduled_drain_amount_cm", $json.float(10.0)],
      ["misting_on_duration_ms", $json.int(parse_int_or(misting_on, 10_000))],
      ["misting_off_duration_ms", $json.int(parse_int_or(misting_off, 180_000))],
      ["misting_temp_threshold", $json.float(30.0)],
      ["high_temp_misting_on_duration_ms", $json.int(15_000)],
      ["high_temp_misting_off_duration_ms", $json.int(60_000)],
      ["last_updated", $json.string(ts)],
    ]),
  );
}

function encode_safety_config(
  dev_id,
  emergency_shutdown,
  min_ec,
  max_ec,
  min_ph,
  max_ph,
  max_dose_cycle,
  max_dose_hour,
  ts
) {
  return $json.object(
    toList([
      ["device_id", $json.string(dev_id)],
      ["emergency_shutdown", $json.bool(emergency_shutdown)],
      ["min_ec_limit", $json.float(parse_float_or(min_ec, 0.5))],
      ["max_ec_limit", $json.float(parse_float_or(max_ec, 3.0))],
      ["min_ph_limit", $json.float(parse_float_or(min_ph, 4.0))],
      ["max_ph_limit", $json.float(parse_float_or(max_ph, 8.0))],
      ["max_ec_delta", $json.float(0.5)],
      ["max_ph_delta", $json.float(0.3)],
      ["max_dose_per_cycle", $json.float(parse_float_or(max_dose_cycle, 50.0))],
      ["cooldown_sec", $json.int(60)],
      ["max_dose_per_hour", $json.float(parse_float_or(max_dose_hour, 200.0))],
      ["water_level_critical_min", $json.float(10.0)],
      ["max_refill_cycles_per_hour", $json.int(3)],
      ["max_drain_cycles_per_hour", $json.int(3)],
      ["max_refill_duration_sec", $json.int(120)],
      ["max_drain_duration_sec", $json.int(120)],
      ["min_temp_limit", $json.float(15.0)],
      ["max_temp_limit", $json.float(35.0)],
      ["ec_ack_threshold", $json.float(0.05)],
      ["ph_ack_threshold", $json.float(0.1)],
      ["water_ack_threshold", $json.float(0.5)],
      ["last_updated", $json.string(ts)],
    ]),
  );
}

function encode_dosing_calibration(
  dev_id,
  pump_a_cap,
  pump_b_cap,
  pump_ph_up_cap,
  pump_ph_down_cap,
  dosing_pwm,
  ts
) {
  return $json.object(
    toList([
      ["device_id", $json.string(dev_id)],
      ["ec_gain_per_ml", $json.float(0.1)],
      ["ph_shift_up_per_ml", $json.float(0.2)],
      ["ph_shift_down_per_ml", $json.float(0.2)],
      ["active_mixing_sec", $json.int(5)],
      ["sensor_stabilize_sec", $json.int(5)],
      ["ec_step_ratio", $json.float(0.4)],
      ["ph_step_ratio", $json.float(0.1)],
      [
        "pump_a_capacity_ml_per_sec",
        $json.float(parse_float_or(pump_a_cap, 1.2)),
      ],
      [
        "pump_b_capacity_ml_per_sec",
        $json.float(parse_float_or(pump_b_cap, 1.2)),
      ],
      [
        "pump_ph_up_capacity_ml_per_sec",
        $json.float(parse_float_or(pump_ph_up_cap, 1.2)),
      ],
      [
        "pump_ph_down_capacity_ml_per_sec",
        $json.float(parse_float_or(pump_ph_down_cap, 1.2)),
      ],
      ["soft_start_duration", $json.int(3000)],
      ["scheduled_mixing_interval_sec", $json.int(3600)],
      ["scheduled_mixing_duration_sec", $json.int(300)],
      ["dosing_pwm_percent", $json.int(parse_int_or(dosing_pwm, 50))],
      ["osaka_mixing_pwm_percent", $json.int(60)],
      ["osaka_misting_pwm_percent", $json.int(100)],
      ["dosing_min_pwm_percent", $json.int(20)],
      ["pump_a_min_pwm_percent", $json.int(20)],
      ["pump_b_min_pwm_percent", $json.int(20)],
      ["pump_ph_up_min_pwm_percent", $json.int(20)],
      ["pump_ph_down_min_pwm_percent", $json.int(20)],
      ["dosing_pulse_on_ms", $json.int(500)],
      ["dosing_pulse_off_ms", $json.int(500)],
      ["dosing_min_dose_ml", $json.float(1.0)],
      ["dosing_max_pulse_count_per_cycle", $json.int(20)],
      ["last_calibrated", $json.string(ts)],
    ]),
  );
}

function encode_sensor_calibration(dev_id, ph_v7, ph_v4, ts) {
  return $json.object(
    toList([
      ["device_id", $json.string(dev_id)],
      ["ph_v7", $json.float(parse_float_or(ph_v7, 2.5))],
      ["ph_v4", $json.float(parse_float_or(ph_v4, 1.428))],
      ["ph_v10", $json.null$()],
      ["ph_calibration_mode", $json.string("2-point")],
      ["ec_factor", $json.float(880.0)],
      ["ec_offset", $json.float(0.0)],
      ["temp_offset", $json.float(0.0)],
      ["temp_compensation_beta", $json.float(0.02)],
      ["publish_interval", $json.int(5000)],
      ["moving_average_window", $json.int(15)],
      ["enable_ph_sensor", $json.bool(true)],
      ["enable_ec_sensor", $json.bool(true)],
      ["enable_temp_sensor", $json.bool(true)],
      ["enable_water_level_sensor", $json.bool(true)],
      ["last_calibrated", $json.string(ts)],
    ]),
  );
}

export function build_unified_payload_json(
  dev_id,
  control_mode,
  is_enabled,
  emergency_shutdown,
  ec_target,
  ec_tolerance,
  ph_target,
  ph_tolerance,
  delay_a_b,
  tank_height,
  water_min,
  water_target,
  water_max,
  water_tolerance,
  auto_refill,
  auto_drain,
  cron,
  misting_on,
  misting_off,
  min_ec,
  max_ec,
  min_ph,
  max_ph,
  max_dose_cycle,
  max_dose_hour,
  pump_a_cap,
  pump_b_cap,
  pump_ph_up_cap,
  pump_ph_down_cap,
  dosing_pwm,
  ph_v7,
  ph_v4,
  timestamp
) {
  let _pipe = $json.object(
    toList([
      [
        "device_config",
        encode_device_config(
          dev_id,
          control_mode,
          is_enabled,
          ec_target,
          ec_tolerance,
          ph_target,
          ph_tolerance,
          delay_a_b,
          timestamp,
        ),
      ],
      [
        "water_config",
        encode_water_config(
          dev_id,
          tank_height,
          water_min,
          water_target,
          water_max,
          water_tolerance,
          auto_refill,
          auto_drain,
          cron,
          misting_on,
          misting_off,
          timestamp,
        ),
      ],
      [
        "safety_config",
        encode_safety_config(
          dev_id,
          emergency_shutdown,
          min_ec,
          max_ec,
          min_ph,
          max_ph,
          max_dose_cycle,
          max_dose_hour,
          timestamp,
        ),
      ],
      [
        "dosing_calibration",
        encode_dosing_calibration(
          dev_id,
          pump_a_cap,
          pump_b_cap,
          pump_ph_up_cap,
          pump_ph_down_cap,
          dosing_pwm,
          timestamp,
        ),
      ],
      [
        "sensor_calibration",
        encode_sensor_calibration(dev_id, ph_v7, ph_v4, timestamp),
      ],
    ]),
  );
  return $json.to_string(_pipe);
}
