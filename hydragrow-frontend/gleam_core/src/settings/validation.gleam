import gleam/float
import gleam/int
import gleam/list

pub type ValidationError {
  InvalidPwm(field: String, message: String)
  InvalidCapacity(field: String, message: String)
}

pub fn validate_dosing_config(
  dosing_pwm_str: String,
  dosing_min_pwm_str: String,
  pump_a_str: String,
  pump_b_str: String,
  pump_ph_up_str: String,
  pump_ph_down_str: String,
) -> List(ValidationError) {
  let errors = []

  let errors = case int.parse(dosing_pwm_str) {
    Ok(val) if val >= 1 && val <= 100 -> errors
    _ -> [
      InvalidPwm("dosing_pwm_percent", "phải nằm trong khoảng 1-100"),
      ..errors
    ]
  }

  let errors = case int.parse(dosing_min_pwm_str) {
    Ok(val) if val >= 0 && val <= 100 -> errors
    _ -> [
      InvalidPwm("dosing_min_pwm_percent", "phải nằm trong khoảng 0-100"),
      ..errors
    ]
  }

  let validate_capacity = fn(acc_errors, field: String, val_str: String) {
    case float.parse(val_str) {
      Ok(val) if val >. 0.0 -> acc_errors
      _ -> [InvalidCapacity(field, "phải lớn hơn 0"), ..acc_errors]
    }
  }

  errors
  |> validate_capacity("pump_a_capacity_ml_per_sec", pump_a_str)
  |> validate_capacity("pump_b_capacity_ml_per_sec", pump_b_str)
  |> validate_capacity("pump_ph_up_capacity_ml_per_sec", pump_ph_up_str)
  |> validate_capacity("pump_ph_down_capacity_ml_per_sec", pump_ph_down_str)
}
