import * as $option from "../gleam_stdlib/gleam/option.mjs";
import { None, Some } from "../gleam_stdlib/gleam/option.mjs";
import * as $should from "../gleeunit/gleeunit/should.mjs";
import { Empty as $Empty, List$Empty$const as $List$Empty$const, makeError } from "./gleam.mjs";
import * as $calibration from "./settings/calibration.mjs";
import * as $cron from "./settings/cron.mjs";
import * as $validation from "./settings/validation.mjs";

const FILEPATH = "test/settings_test.gleam";

export function validate_dosing_config_test() {
  let _pipe = $validation.validate_dosing_config(
    "50",
    "20",
    "1.2",
    "1.2",
    "1.2",
    "1.2",
  );
  $should.equal(_pipe, $List$Empty$const);
  let errors = $validation.validate_dosing_config(
    "150",
    "20",
    "1.2",
    "1.2",
    "1.2",
    "1.2",
  );
  if (errors instanceof $Empty) {
    throw makeError(
      "panic",
      FILEPATH,
      "settings_test",
      17,
      "validate_dosing_config_test",
      "Kỳ vọng có lỗi validation PWM",
      {}
    )
  } else {
    let err = errors.head;
    let _pipe$1 = err.field;
    return $should.equal(_pipe$1, "dosing_pwm_percent");
  }
}

export function parse_cron_safe_test() {
  let schedule = $cron.parse_cron_safe("0 30 14 * * MON,WED");
  let _pipe = schedule.minute;
  $should.equal(_pipe, 30);
  let _pipe$1 = schedule.hour;
  $should.equal(_pipe$1, 14);
  let _pipe$2 = schedule.is_every_day;
  $should.equal(_pipe$2, false);
  let _pipe$3 = schedule.days_str;
  $should.equal(_pipe$3, "MON,WED");
  let fallback = $cron.parse_cron_safe("invalid_cron_string");
  let _pipe$4 = fallback.hour;
  $should.equal(_pipe$4, 8);
  let _pipe$5 = fallback.minute;
  $should.equal(_pipe$5, 0);
  let _pipe$6 = fallback.is_every_day;
  return $should.equal(_pipe$6, true);
}

export function calculate_calibration_summary_test() {
  let summary = $calibration.calculate_summary(new Some(2.5), new Some(1.5), 80);
  let _pipe = summary.reliability;
  return $should.equal(_pipe, 95);
}
