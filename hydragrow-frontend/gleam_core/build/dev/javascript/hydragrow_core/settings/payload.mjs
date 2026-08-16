import * as $json from "../../gleam_json/gleam/json.mjs";
import * as $float from "../../gleam_stdlib/gleam/float.mjs";
import * as $int from "../../gleam_stdlib/gleam/int.mjs";
import * as $option from "../../gleam_stdlib/gleam/option.mjs";
import { None, Some } from "../../gleam_stdlib/gleam/option.mjs";
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
  ec_target_str,
  ec_tolerance_str,
  ph_target_str,
  ph_tolerance_str,
  delay_a_b_str,
  ts
) {
  return $json.object(
    toList([
      ["device_id", $json.string(dev_id)],
      ["control_mode", $json.string(control_mode)],
      ["is_enabled", $json.bool(is_enabled)],
      ["ec_target", $json.float(parse_float_or(ec_target_str, 1.5))],
      ["ec_tolerance", $json.float(parse_float_or(ec_tolerance_str, 0.05))],
      ["ph_target", $json.float(parse_float_or(ph_target_str, 6.0))],
      ["ph_tolerance", $json.float(parse_float_or(ph_tolerance_str, 0.5))],
      ["delay_between_a_and_b_sec", $json.int(parse_int_or(delay_a_b_str, 10))],
      ["last_updated", $json.string(ts)],
    ]),
  );
}

function encode_water_config(
  dev_id,
  tank_height_str,
  water_target_str,
  auto_refill,
  cron_str,
  ts
) {
  return $json.object(
    toList([
      ["device_id", $json.string(dev_id)],
      ["tank_height", $json.float(parse_float_or(tank_height_str, 50.0))],
      [
        "water_level_target",
        $json.float(parse_float_or(water_target_str, 80.0)),
      ],
      ["auto_refill_enabled", $json.bool(auto_refill)],
      ["water_change_cron", $json.string(cron_str)],
      ["last_updated", $json.string(ts)],
    ]),
  );
}

function encode_dosing_calibration(
  dev_id,
  pump_a_cap_str,
  pump_b_cap_str,
  dosing_pwm_str,
  ts
) {
  return $json.object(
    toList([
      ["device_id", $json.string(dev_id)],
      [
        "pump_a_capacity_ml_per_sec",
        $json.float(parse_float_or(pump_a_cap_str, 1.2)),
      ],
      [
        "pump_b_capacity_ml_per_sec",
        $json.float(parse_float_or(pump_b_cap_str, 1.2)),
      ],
      ["dosing_pwm_percent", $json.int(parse_int_or(dosing_pwm_str, 50))],
      ["last_calibrated", $json.string(ts)],
    ]),
  );
}

export function build_unified_payload_json(
  dev_id,
  control_mode,
  is_enabled,
  ec_target,
  ec_tolerance,
  ph_target,
  ph_tolerance,
  delay_a_b,
  tank_height,
  water_target,
  auto_refill,
  cron,
  pump_a_cap,
  pump_b_cap,
  dosing_pwm,
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
          water_target,
          auto_refill,
          cron,
          timestamp,
        ),
      ],
      [
        "dosing_calibration",
        encode_dosing_calibration(
          dev_id,
          pump_a_cap,
          pump_b_cap,
          dosing_pwm,
          timestamp,
        ),
      ],
    ]),
  );
  return $json.to_string(_pipe);
}
