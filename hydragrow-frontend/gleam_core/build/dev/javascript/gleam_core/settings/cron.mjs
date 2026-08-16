import * as $int from "../../gleam_stdlib/gleam/int.mjs";
import * as $list from "../../gleam_stdlib/gleam/list.mjs";
import * as $result from "../../gleam_stdlib/gleam/result.mjs";
import * as $string from "../../gleam_stdlib/gleam/string.mjs";
import { Ok, Error, Empty as $Empty, CustomType as $CustomType } from "../gleam.mjs";

export class CronSchedule extends $CustomType {
  constructor(minute, hour, is_every_day, days_str) {
    super();
    this.minute = minute;
    this.hour = hour;
    this.is_every_day = is_every_day;
    this.days_str = days_str;
  }
}
export const CronSchedule$CronSchedule = (minute, hour, is_every_day, days_str) =>
  new CronSchedule(minute, hour, is_every_day, days_str);
export const CronSchedule$isCronSchedule = (value) =>
  value instanceof CronSchedule;
export const CronSchedule$CronSchedule$minute = (value) => value.minute;
export const CronSchedule$CronSchedule$0 = (value) => value.minute;
export const CronSchedule$CronSchedule$hour = (value) => value.hour;
export const CronSchedule$CronSchedule$1 = (value) => value.hour;
export const CronSchedule$CronSchedule$is_every_day = (value) =>
  value.is_every_day;
export const CronSchedule$CronSchedule$2 = (value) => value.is_every_day;
export const CronSchedule$CronSchedule$days_str = (value) => value.days_str;
export const CronSchedule$CronSchedule$3 = (value) => value.days_str;

/**
 * Parse chuỗi Cron chuẩn
 */
export function parse_cron(cron_str) {
  let _block;
  let _pipe = cron_str;
  let _pipe$1 = $string.trim(_pipe);
  let _pipe$2 = $string.split(_pipe$1, " ");
  _block = $list.filter(_pipe$2, (s) => { return s !== ""; });
  let parts = _block;
  if (parts instanceof $Empty) {
    return new Error(undefined);
  } else {
    let $ = parts.tail;
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
            return new Error(undefined);
          } else {
            let $4 = $3.tail;
            if ($4 instanceof $Empty) {
              return new Error(undefined);
            } else {
              let min_str = $.head;
              let hour_str = $1.head;
              let dow_str = $4.head;
              return $result.try$(
                $int.parse(min_str),
                (minute) => {
                  return $result.try$(
                    $int.parse(hour_str),
                    (hour) => {
                      let _block$1;
                      if (dow_str === "*") {
                        _block$1 = [true, ""];
                      } else {
                        _block$1 = [false, dow_str];
                      }
                      let $5 = _block$1;
                      let is_every_day = $5[0];
                      let days_str = $5[1];
                      return new Ok(
                        new CronSchedule(minute, hour, is_every_day, days_str),
                      );
                    },
                  );
                },
              );
            }
          }
        }
      }
    }
  }
}

/**
 * HÀM DÀNH RIÊNG CHO JS/REACT:
 * Tự động fallback về giá trị mặc định khi gặp lỗi.
 * Trả về Object trực tiếp (không bọc trong Result) giúp React đọc thuộc tính an toàn.
 */
export function parse_cron_safe(cron_str) {
  let $ = parse_cron(cron_str);
  if ($ instanceof Ok) {
    let schedule = $[0];
    return schedule;
  } else {
    return new CronSchedule(0, 8, true, "");
  }
}

/**
 * Convert ngược lại thành chuỗi Cron từ thông số JS
 */
export function to_cron_string(minute, hour, is_every_day, days_str) {
  let _block;
  if (is_every_day) {
    _block = "*";
  } else if (days_str === "") {
    _block = "*";
  } else {
    _block = days_str;
  }
  let dow_part = _block;
  return (((("0 " + $int.to_string(minute)) + " ") + $int.to_string(hour)) + " * * ") + dow_part;
}
