#pragma once

// Đảm bảo esp_ota_mark_app_valid_cancel_rollback() chỉ được gọi đúng 1 lần
// mỗi lần boot, và chỉ sau khi thiết bị đã chứng minh hoạt động tốt (kết nối
// MQTT thành công). Port 1:1 từ
// hydragrow-controller-core/src/core/ota_health.rs::OtaValidationGate — cùng
// tên, cùng hành vi, khác ngôn ngữ.
class OtaValidationGate {
public:
    OtaValidationGate() : marked_(false) {}

    // Gọi mỗi khi thiết bị đạt trạng thái "known healthy" (vd: mỗi lần
    // reconnect() MQTT thành công). Trả về true ĐÚNG 1 LẦN DUY NHẤT — caller
    // chỉ nên gọi esp_ota_mark_app_valid_cancel_rollback() khi hàm này trả
    // về true.
    bool markIfNeeded() {
        if (marked_) return false;
        marked_ = true;
        return true;
    }

private:
    bool marked_;
};