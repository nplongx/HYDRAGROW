import * as $option from "../gleam_stdlib/gleam/option.mjs";
import { Some, Option$None$const } from "../gleam_stdlib/gleam/option.mjs";
import * as $should from "../gleeunit/gleeunit/should.mjs";
import * as $fsm from "./fsm.mjs";

export function extract_fault_code_test() {
  let _pipe = $fsm.extract_fault_code("SystemFault:MAX_HOURLY_DOSE_EC");
  $should.equal(_pipe, new Some("MAX_HOURLY_DOSE_EC"));
  let _pipe$1 = $fsm.extract_fault_code("Fault:EC_DOSING_FAILED");
  $should.equal(_pipe$1, new Some("EC_DOSING_FAILED"));
  let _pipe$2 = $fsm.extract_fault_code("{\"Fault\":\"PH_DOSING_FAILED\"}");
  $should.equal(_pipe$2, new Some("PH_DOSING_FAILED"));
  let _pipe$3 = $fsm.extract_fault_code("Monitoring");
  return $should.equal(_pipe$3, Option$None$const);
}

export function compute_health_test() {
  let health = $fsm.compute_health(true, new Some(95));
  let _pipe = health.score;
  $should.equal(_pipe, 95);
  let _pipe$1 = health.label;
  $should.equal(_pipe$1, "Hoàn hảo");
  let offline_health = $fsm.compute_health(false, new Some(100));
  let _pipe$2 = offline_health.score;
  $should.equal(_pipe$2, 0);
  let _pipe$3 = offline_health.label;
  return $should.equal(_pipe$3, "Mất kết nối");
}

export function friendly_state_test() {
  let state = $fsm.friendly_state("MimoDosing", true);
  let _pipe = state.label;
  $should.equal(_pipe, "Đang bổ sung vi chất");
  let _pipe$1 = state.tone;
  return $should.equal(_pipe$1, "mist");
}
