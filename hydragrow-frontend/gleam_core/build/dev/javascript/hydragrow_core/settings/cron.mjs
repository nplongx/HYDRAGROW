import * as $int from "../../gleam_stdlib/gleam/int.mjs";
import * as $list from "../../gleam_stdlib/gleam/list.mjs";
import * as $result from "../../gleam_stdlib/gleam/result.mjs";
import * as $string from "../../gleam_stdlib/gleam/string.mjs";
import {
  Ok,
  Error,
  Empty as $Empty,
  List$Empty$const as $List$Empty$const,
  CustomType as $CustomType,
} from "../gleam.mjs";

export class CronSchedule extends $CustomType {
  constructor(minute, hour, days_of_week, is_every_day) {
    super();
    this.minute = minute;
    this.hour = hour;
    this.days_of_week = days_of_week;
    this.is_every_day = is_every_day;
  }
}
export const CronSchedule$CronSchedule = (minute, hour, days_of_week, is_every_day) =>
  new CronSchedule(minute, hour, days_of_week, is_every_day);
export const CronSchedule$isCronSchedule = (value) =>
  value instanceof CronSchedule;
export const CronSchedule$CronSchedule$minute = (value) => value.minute;
export const CronSchedule$CronSchedule$0 = (value) => value.minute;
export const CronSchedule$CronSchedule$hour = (value) => value.hour;
export const CronSchedule$CronSchedule$1 = (value) => value.hour;
export const CronSchedule$CronSchedule$days_of_week = (value) =>
  value.days_of_week;
export const CronSchedule$CronSchedule$2 = (value) => value.days_of_week;
export const CronSchedule$CronSchedule$is_every_day = (value) =>
  value.is_every_day;
export const CronSchedule$CronSchedule$3 = (value) => value.is_every_day;

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
                        _block$1 = [true, $List$Empty$const];
                      } else {
                        _block$1 = [false, $string.split(dow_str, ",")];
                      }
                      let $5 = _block$1;
                      let is_every_day = $5[0];
                      let days = $5[1];
                      return new Ok(
                        new CronSchedule(minute, hour, days, is_every_day),
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

export function to_cron_string(schedule) {
  let _block;
  let $ = schedule.is_every_day;
  let $1 = schedule.days_of_week;
  if ($) {
    _block = "*";
  } else if ($1 instanceof $Empty) {
    _block = "*";
  } else {
    let days = $1;
    _block = $string.join(days, ",");
  }
  let dow_part = _block;
  return (((("0 " + $int.to_string(schedule.minute)) + " ") + $int.to_string(
    schedule.hour,
  )) + " * * ") + dow_part;
}
