import * as $float from "../../gleam_stdlib/gleam/float.mjs";
import * as $int from "../../gleam_stdlib/gleam/int.mjs";
import * as $list from "../../gleam_stdlib/gleam/list.mjs";
import {
  Ok,
  List$Empty$const as $List$Empty$const,
  prepend as listPrepend,
  CustomType as $CustomType,
} from "../gleam.mjs";

export class InvalidPwm extends $CustomType {
  constructor(field, message) {
    super();
    this.field = field;
    this.message = message;
  }
}
export const ValidationError$InvalidPwm = (field, message) =>
  new InvalidPwm(field, message);
export const ValidationError$isInvalidPwm = (value) =>
  value instanceof InvalidPwm;
export const ValidationError$InvalidPwm$field = (value) => value.field;
export const ValidationError$InvalidPwm$0 = (value) => value.field;
export const ValidationError$InvalidPwm$message = (value) => value.message;
export const ValidationError$InvalidPwm$1 = (value) => value.message;

export class InvalidCapacity extends $CustomType {
  constructor(field, message) {
    super();
    this.field = field;
    this.message = message;
  }
}
export const ValidationError$InvalidCapacity = (field, message) =>
  new InvalidCapacity(field, message);
export const ValidationError$isInvalidCapacity = (value) =>
  value instanceof InvalidCapacity;
export const ValidationError$InvalidCapacity$field = (value) => value.field;
export const ValidationError$InvalidCapacity$0 = (value) => value.field;
export const ValidationError$InvalidCapacity$message = (value) => value.message;
export const ValidationError$InvalidCapacity$1 = (value) => value.message;

export const ValidationError$field = (value) => value.field;
export const ValidationError$message = (value) => value.message;

export function validate_dosing_config(
  dosing_pwm_str,
  dosing_min_pwm_str,
  pump_a_str,
  pump_b_str,
  pump_ph_up_str,
  pump_ph_down_str
) {
  let errors = $List$Empty$const;
  let _block;
  let $ = $int.parse(dosing_pwm_str);
  if ($ instanceof Ok) {
    let val = $[0];
    if ((val >= 1) && (val <= 100)) {
      _block = errors;
    } else {
      _block = listPrepend(
        new InvalidPwm("dosing_pwm_percent", "phải nằm trong khoảng 1-100"),
        errors,
      );
    }
  } else {
    _block = listPrepend(
      new InvalidPwm("dosing_pwm_percent", "phải nằm trong khoảng 1-100"),
      errors,
    );
  }
  let errors$1 = _block;
  let _block$1;
  let $1 = $int.parse(dosing_min_pwm_str);
  if ($1 instanceof Ok) {
    let val = $1[0];
    if ((val >= 0) && (val <= 100)) {
      _block$1 = errors$1;
    } else {
      _block$1 = listPrepend(
        new InvalidPwm("dosing_min_pwm_percent", "phải nằm trong khoảng 0-100"),
        errors$1,
      );
    }
  } else {
    _block$1 = listPrepend(
      new InvalidPwm("dosing_min_pwm_percent", "phải nằm trong khoảng 0-100"),
      errors$1,
    );
  }
  let errors$2 = _block$1;
  let validate_capacity = (acc_errors, field, val_str) => {
    let $2 = $float.parse(val_str);
    if ($2 instanceof Ok) {
      let val = $2[0];
      if (val > 0.0) {
        return acc_errors;
      } else {
        return listPrepend(
          new InvalidCapacity(field, "phải lớn hơn 0"),
          acc_errors,
        );
      }
    } else {
      return listPrepend(
        new InvalidCapacity(field, "phải lớn hơn 0"),
        acc_errors,
      );
    }
  };
  let _pipe = errors$2;
  let _pipe$1 = validate_capacity(
    _pipe,
    "pump_a_capacity_ml_per_sec",
    pump_a_str,
  );
  let _pipe$2 = validate_capacity(
    _pipe$1,
    "pump_b_capacity_ml_per_sec",
    pump_b_str,
  );
  let _pipe$3 = validate_capacity(
    _pipe$2,
    "pump_ph_up_capacity_ml_per_sec",
    pump_ph_up_str,
  );
  return validate_capacity(
    _pipe$3,
    "pump_ph_down_capacity_ml_per_sec",
    pump_ph_down_str,
  );
}
