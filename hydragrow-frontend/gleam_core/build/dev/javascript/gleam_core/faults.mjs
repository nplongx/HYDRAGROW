import * as $option from "../gleam_stdlib/gleam/option.mjs";
import { Some, Option$None$const } from "../gleam_stdlib/gleam/option.mjs";
import { CustomType as $CustomType } from "./gleam.mjs";

export class FaultGuide extends $CustomType {
  constructor(code, short, reason, action, recovery) {
    super();
    this.code = code;
    this.short = short;
    this.reason = reason;
    this.action = action;
    this.recovery = recovery;
  }
}
export const FaultGuide$FaultGuide = (code, short, reason, action, recovery) =>
  new FaultGuide(code, short, reason, action, recovery);
export const FaultGuide$isFaultGuide = (value) => value instanceof FaultGuide;
export const FaultGuide$FaultGuide$code = (value) => value.code;
export const FaultGuide$FaultGuide$0 = (value) => value.code;
export const FaultGuide$FaultGuide$short = (value) => value.short;
export const FaultGuide$FaultGuide$1 = (value) => value.short;
export const FaultGuide$FaultGuide$reason = (value) => value.reason;
export const FaultGuide$FaultGuide$2 = (value) => value.reason;
export const FaultGuide$FaultGuide$action = (value) => value.action;
export const FaultGuide$FaultGuide$3 = (value) => value.action;
export const FaultGuide$FaultGuide$recovery = (value) => value.recovery;
export const FaultGuide$FaultGuide$4 = (value) => value.recovery;

export function get_fault_guide(code) {
  if (code === "MAX_HOURLY_DOSE_EC") {
    return new Some(
      new FaultGuide(
        "MAX_HOURLY_DOSE_EC",
        "Đạt giới hạn châm EC theo giờ",
        "Đã châm nhiều phân EC trong vòng 1 giờ qua.",
        "Chờ 1 giờ hoặc thực hiện Reset thủ công.",
        "Tự khôi phục khi hết giới hạn rate-limit.",
      ),
    );
  } else if (code === "MAX_HOURLY_DOSE_PH") {
    return new Some(
      new FaultGuide(
        "MAX_HOURLY_DOSE_PH",
        "Đạt giới hạn châm pH theo giờ",
        "Đã châm nhiều dung dịch pH trong 1 giờ qua.",
        "Kiểm tra cảm biến pH, sau đó nhấn Reset.",
        "Tự khôi phục khi hết giới hạn rate-limit.",
      ),
    );
  } else if (code === "EC_DOSING_FAILED") {
    return new Some(
      new FaultGuide(
        "EC_DOSING_FAILED",
        "Châm EC thất bại sau 3 lần thử",
        "Bơm châm đã chạy nhưng chỉ số EC không tăng.",
        "Kiểm tra bình A/B còn dung dịch không hoặc bơm bị nghẽn.",
        "Khắc phục nguyên nhân vật lý sau đó nhấn Reset.",
      ),
    );
  } else if (code === "PH_DOSING_FAILED") {
    return new Some(
      new FaultGuide(
        "PH_DOSING_FAILED",
        "Châm pH thất bại sau 3 lần thử",
        "Bơm pH đã chạy nhưng chỉ số pH không đổi.",
        "Kiểm tra bình pH Up/Down và van một chiều.",
        "Khắc phục nguyên nhân vật lý sau đó nhấn Reset.",
      ),
    );
  } else if (code === "WATER_REFILL_FAILED") {
    return new Some(
      new FaultGuide(
        "WATER_REFILL_FAILED",
        "Cấp nước thất bại sau 3 lần thử",
        "Bơm cấp nước đã bật nhưng mực nước không tăng.",
        "Kiểm tra phao báo mực nước, nguồn nước cấp và van.",
        "Khắc phục nguồn nước và nhấn Reset.",
      ),
    );
  } else {
    return Option$None$const;
  }
}
