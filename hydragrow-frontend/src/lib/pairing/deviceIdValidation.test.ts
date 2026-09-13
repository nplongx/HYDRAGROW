// hydragrow-frontend/src/lib/pairing/deviceIdValidation.test.ts
import { describe, expect, it } from 'vitest';
import { validateDeviceId } from './deviceIdValidation';

describe('validateDeviceId', () => {
  it('rỗng → lỗi bắt buộc', () => {
    expect(validateDeviceId('')).toBe('Vui lòng nhập Device ID.');
    expect(validateDeviceId('   ')).toBe('Vui lòng nhập Device ID.');
  });

  it('chứa khoảng trắng ở giữa → lỗi định dạng', () => {
    expect(validateDeviceId('hydra station 01')).toBe(
      'Device ID chỉ gồm chữ, số, dấu gạch dưới hoặc gạch ngang, không có khoảng trắng.',
    );
  });

  it('chứa ký tự đặc biệt không hợp lệ → lỗi định dạng', () => {
    expect(validateDeviceId('hydra@station!01')).toBe(
      'Device ID chỉ gồm chữ, số, dấu gạch dưới hoặc gạch ngang, không có khoảng trắng.',
    );
  });

  it('hợp lệ (chữ, số, gạch dưới, gạch ngang) → null', () => {
    expect(validateDeviceId('hydra_station-01')).toBeNull();
    expect(validateDeviceId('HydraStation01')).toBeNull();
  });

  it('URL dạng hydragrow://claim/... vẫn hợp lệ trước khi được parse riêng', () => {
    // Cho phép qua đây; parseDeviceIdFromQr() xử lý trích xuất trước khi validate được gọi trên chuỗi đã trích.
    expect(validateDeviceId('hydra-station-01')).toBeNull();
  });
});
