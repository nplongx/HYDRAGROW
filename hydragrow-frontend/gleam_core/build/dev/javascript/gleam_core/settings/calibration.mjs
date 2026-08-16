import * as $float from "../../gleam_stdlib/gleam/float.mjs";
import * as $int from "../../gleam_stdlib/gleam/int.mjs";
import * as $option from "../../gleam_stdlib/gleam/option.mjs";
import { None, Some } from "../../gleam_stdlib/gleam/option.mjs";
import { CustomType as $CustomType } from "../gleam.mjs";

export class CalibrationSummary extends $CustomType {
  constructor(ph_v7, ph_v4, reliability) {
    super();
    this.ph_v7 = ph_v7;
    this.ph_v4 = ph_v4;
    this.reliability = reliability;
  }
}
export const CalibrationSummary$CalibrationSummary = (ph_v7, ph_v4, reliability) =>
  new CalibrationSummary(ph_v7, ph_v4, reliability);
export const CalibrationSummary$isCalibrationSummary = (value) =>
  value instanceof CalibrationSummary;
export const CalibrationSummary$CalibrationSummary$ph_v7 = (value) =>
  value.ph_v7;
export const CalibrationSummary$CalibrationSummary$0 = (value) => value.ph_v7;
export const CalibrationSummary$CalibrationSummary$ph_v4 = (value) =>
  value.ph_v4;
export const CalibrationSummary$CalibrationSummary$1 = (value) => value.ph_v4;
export const CalibrationSummary$CalibrationSummary$reliability = (value) =>
  value.reliability;
export const CalibrationSummary$CalibrationSummary$2 = (value) =>
  value.reliability;

/**
 * Tính toán tóm tắt hiệu chuẩn và điểm tin cậy (reliability)
 */
export function calculate_summary(v7, v4, avg_confidence) {
  let _block;
  if (v7 instanceof Some && v4 instanceof Some) {
    let val7 = v7[0];
    let val4 = v4[0];
    let spread = $float.absolute_value(val7 - val4);
    {
      let s = spread;
      if (s >= 0.2) {
        _block = 15;
      } else {
        let s = spread;
        if (s >= 0.1) {
          _block = 8;
        } else {
          _block = 0;
        }
      }
    }
  } else {
    _block = 0;
  }
  let spread_bonus = _block;
  let raw_reliability = avg_confidence + spread_bonus;
  let reliability = $int.min(100, $int.max(0, raw_reliability));
  return new CalibrationSummary(v7, v4, reliability);
}

/**
 * Kiểm tra xem kết quả hiệu chuẩn đã đủ điều kiện để áp dụng hay chưa
 */
export function is_calibration_valid(summary) {
  let $ = summary.ph_v7;
  let $1 = summary.ph_v4;
  if ($ instanceof Some && $1 instanceof Some) {
    return true;
  } else {
    return false;
  }
}
