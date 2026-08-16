import * as $should from "../gleeunit/gleeunit/should.mjs";
import * as $dashboard from "./dashboard.mjs";

export function eval_sensor_status_test() {
  let status = $dashboard.eval_sensor_status_safe(false, "1.5", "0.5", "3.0");
  let _pipe = status.label;
  $should.equal(_pipe, "Ổn định");
  let _pipe$1 = status.tone;
  $should.equal(_pipe$1, "good");
  let high_status = $dashboard.eval_sensor_status_safe(
    false,
    "4.0",
    "0.5",
    "3.0",
  );
  let _pipe$2 = high_status.label;
  $should.equal(_pipe$2, "Cao");
  let _pipe$3 = high_status.tone;
  $should.equal(_pipe$3, "warn");
  let err_status = $dashboard.eval_sensor_status_safe(true, "1.5", "0.5", "3.0");
  let _pipe$4 = err_status.label;
  $should.equal(_pipe$4, "Cần kiểm tra");
  let _pipe$5 = err_status.tone;
  return $should.equal(_pipe$5, "danger");
}

export function calc_hourly_dose_str_test() {
  let now = 10000000.0;
  let ts_recent = "9900000.0";
  let ts_old = "5000000.0";
  let events_str = ((ts_recent + ",10.0,5.0;") + ts_old) + ",20.0,10.0";
  let result = $dashboard.calc_hourly_dose_str(events_str, now);
  let _pipe = result.ec_ml;
  $should.equal(_pipe, 10.0);
  let _pipe$1 = result.ph_ml;
  return $should.equal(_pipe$1, 5.0);
}
