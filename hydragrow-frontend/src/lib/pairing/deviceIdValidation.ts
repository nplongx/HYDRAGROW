// hydragrow-frontend/src/lib/pairing/deviceIdValidation.ts

/**
 * Validate định dạng Device ID nhập tay (form-design skill: "Inline
 * validation: validate on blur" + "Error messages: explain what went
 * wrong and how to fix it"). Không validate xem thiết bị có tồn tại hay
 * đã bị người khác claim hay chưa — đó là lỗi phía server, hiển thị
 * riêng qua formError sau khi gọi API (xem Task 14).
 */
const VALID_DEVICE_ID = /^[a-zA-Z0-9_-]+$/;

export function validateDeviceId(raw: string): string | null {
  const trimmed = raw.trim();
  if (!trimmed) return 'Vui lòng nhập Device ID.';
  if (!VALID_DEVICE_ID.test(trimmed)) {
    return 'Device ID chỉ gồm chữ, số, dấu gạch dưới hoặc gạch ngang, không có khoảng trắng.';
  }
  return null;
}
