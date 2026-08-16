import * as $float from "../gleam_stdlib/gleam/float.mjs";
import * as $int from "../gleam_stdlib/gleam/int.mjs";
import * as $list from "../gleam_stdlib/gleam/list.mjs";
import * as $string from "../gleam_stdlib/gleam/string.mjs";
import { Ok, Error, Empty as $Empty, CustomType as $CustomType, divideFloat } from "./gleam.mjs";

export class SensorEvaluation extends $CustomType {
  constructor(label, tone) {
    super();
    this.label = label;
    this.tone = tone;
  }
}
export const SensorEvaluation$SensorEvaluation = (label, tone) =>
  new SensorEvaluation(label, tone);
export const SensorEvaluation$isSensorEvaluation = (value) =>
  value instanceof SensorEvaluation;
export const SensorEvaluation$SensorEvaluation$label = (value) => value.label;
export const SensorEvaluation$SensorEvaluation$0 = (value) => value.label;
export const SensorEvaluation$SensorEvaluation$tone = (value) => value.tone;
export const SensorEvaluation$SensorEvaluation$1 = (value) => value.tone;

export class HourlyDose extends $CustomType {
  constructor(ec_ml, ph_ml) {
    super();
    this.ec_ml = ec_ml;
    this.ph_ml = ph_ml;
  }
}
export const HourlyDose$HourlyDose = (ec_ml, ph_ml) =>
  new HourlyDose(ec_ml, ph_ml);
export const HourlyDose$isHourlyDose = (value) => value instanceof HourlyDose;
export const HourlyDose$HourlyDose$ec_ml = (value) => value.ec_ml;
export const HourlyDose$HourlyDose$0 = (value) => value.ec_ml;
export const HourlyDose$HourlyDose$ph_ml = (value) => value.ph_ml;
export const HourlyDose$HourlyDose$1 = (value) => value.ph_ml;

export function eval_sensor_status(
  has_error,
  value,
  has_min,
  min_val,
  has_max,
  max_val
) {
  if (has_error) {
    return new SensorEvaluation("Cần kiểm tra", "danger");
  } else {
    let is_low = has_min && (value < min_val);
    let is_high = has_max && (value > max_val);
    if (is_low) {
      return new SensorEvaluation("Thấp", "warn");
    } else if (is_high) {
      return new SensorEvaluation("Cao", "warn");
    } else {
      return new SensorEvaluation("Ổn định", "good");
    }
  }
}

export function eval_sensor_status_safe(has_error, value_str, min_str, max_str) {
  let $ = $float.parse(value_str);
  if ($ instanceof Ok) {
    let val = $[0];
    let _block;
    let $2 = $float.parse(min_str);
    if ($2 instanceof Ok) {
      let m = $2[0];
      _block = [true, m];
    } else {
      _block = [false, 0.0];
    }
    let $1 = _block;
    let has_min = $1[0];
    let min_val = $1[1];
    let _block$1;
    let $4 = $float.parse(max_str);
    if ($4 instanceof Ok) {
      let m = $4[0];
      _block$1 = [true, m];
    } else {
      _block$1 = [false, 0.0];
    }
    let $3 = _block$1;
    let has_max = $3[0];
    let max_val = $3[1];
    return eval_sensor_status(
      has_error,
      val,
      has_min,
      min_val,
      has_max,
      max_val,
    );
  } else {
    return new SensorEvaluation("Cần kiểm tra", "danger");
  }
}

export function calc_budget_percent(used, limit) {
  let $ = limit <= 0.0;
  if ($) {
    return 0;
  } else {
    let pct = $float.round((divideFloat(used, limit)) * 100.0);
    return $int.min(100, $int.max(0, pct));
  }
}

export function calc_budget_percent_safe(used_num, limit_num) {
  let _block;
  let $ = used_num < 0.0;
  if ($) {
    _block = 0.0;
  } else {
    _block = used_num;
  }
  let used = _block;
  let _block$1;
  let $1 = limit_num <= 0.0;
  if ($1) {
    _block$1 = 300.0;
  } else {
    _block$1 = limit_num;
  }
  let limit = _block$1;
  return calc_budget_percent(used, limit);
}

/**
 * Nhận chuỗi định dạng "ts1,ec1,ph1;ts2,ec2,ph2" từ JS
 */
export function calc_hourly_dose_str(events_str, now_ms) {
  let one_hour_ms = 3600000.0;
  let _pipe = events_str;
  let _pipe$1 = $string.split(_pipe, ";");
  let _pipe$2 = $list.filter_map(
    _pipe$1,
    (item_str) => {
      let $ = $string.split(item_str, ",");
      if ($ instanceof $Empty) {
        return new Error(undefined);
      } else {
        let $1 = $.tail;
        if ($1 instanceof $Empty) {
          return new Error(undefined);
        } else {
          let $2 = $1.tail;
          if ($2 instanceof $Empty) {
            return new Error(undefined);
          } else {
            let $3 = $2.tail;
            if ($3 instanceof $Empty) {
              let ts_str = $.head;
              let ec_str = $1.head;
              let ph_str = $2.head;
              let $4 = $float.parse(ts_str);
              let $5 = $float.parse(ec_str);
              let $6 = $float.parse(ph_str);
              if ($4 instanceof Ok && $5 instanceof Ok && $6 instanceof Ok) {
                let ts = $4[0];
                let ec = $5[0];
                let ph = $6[0];
                return new Ok([ts, ec, ph]);
              } else {
                return new Error(undefined);
              }
            } else {
              return new Error(undefined);
            }
          }
        }
      }
    },
  );
  let _pipe$3 = $list.filter(
    _pipe$2,
    (item) => {
      let ts = item[0];
      let diff = now_ms - ts;
      return (diff <= one_hour_ms) && (diff >= 0.0);
    },
  );
  return $list.fold(
    _pipe$3,
    new HourlyDose(0.0, 0.0),
    (acc, item) => {
      let ec = item[1];
      let ph = item[2];
      return new HourlyDose(acc.ec_ml + ec, acc.ph_ml + ph);
    },
  );
}
