import React from 'react';

export type FaultGuide = {
  code: string;
  short: string;
  reason: string;
  action: string;
  recovery: string;
};

export const FAULT_GUIDES: Record<string, FaultGuide> = {
  MAX_HOURLY_DOSE_EC: {
    code: 'MAX_HOURLY_DOSE_EC',
    short: 'Đã đạt giới hạn châm EC theo giờ.',
    reason: 'Đã châm phân EC đủ giới hạn trong 1 giờ.',
    action: 'Chờ hết cửa sổ 1 giờ hoặc bấm Reset lỗi.',
    recovery: 'Tự phục hồi khi hết cửa sổ rate-limit 1 giờ.'
  },
  MAX_HOURLY_DOSE_PH: {
    code: 'MAX_HOURLY_DOSE_PH',
    short: 'Đã đạt giới hạn châm pH theo giờ.',
    reason: 'Đã châm pH đủ giới hạn trong 1 giờ.',
    action: 'Kiểm tra cảm biến pH, sau đó chờ hoặc Reset lỗi.',
    recovery: 'Tự phục hồi khi hết cửa sổ rate-limit 1 giờ.'
  },
  EC_DOSING_FAILED: {
    code: 'EC_DOSING_FAILED', short: 'Châm EC thất bại sau 3 lần thử.',
    reason: 'Bơm chạy 3 lần nhưng EC không tăng.',
    action: 'Kiểm tra bình A/B còn dung dịch và đường ống có tắc không.',
    recovery: 'Không tự phục hồi, cần xử lý nguyên nhân rồi Reset.'
  },
  PH_DOSING_FAILED: {
    code: 'PH_DOSING_FAILED', short: 'Châm pH thất bại sau 3 lần thử.',
    reason: 'Bơm pH chạy 3 lần nhưng pH không đổi.',
    action: 'Kiểm tra bình pH Up/Down và đầu bơm.',
    recovery: 'Không tự phục hồi, cần xử lý nguyên nhân rồi Reset.'
  },
  WATER_REFILL_FAILED: {
    code: 'WATER_REFILL_FAILED', short: 'Cấp nước thất bại sau 3 lần thử.',
    reason: 'Bơm vào 3 lần nhưng mực nước không tăng.',
    action: 'Kiểm tra phao, nguồn nước, bơm và van.',
    recovery: 'Không tự phục hồi, cần xử lý nguyên nhân rồi Reset.'
  },
  TOO_MANY_REFILLS: {
    code: 'TOO_MANY_REFILLS', short: 'Cấp nước quá nhiều lần trong 1 giờ.',
    reason: 'Hệ thống đã cấp nước quá ngưỡng số lần cho phép.',
    action: 'Kiểm tra rò rỉ nước và cảm biến siêu âm.',
    recovery: 'Tự phục hồi khi hết cửa sổ rate-limit 1 giờ.'
  },
  TOO_MANY_DRAINS: {
    code: 'TOO_MANY_DRAINS', short: 'Xả nước quá nhiều lần trong 1 giờ.',
    reason: 'Hệ thống đã xả nước quá ngưỡng số lần cho phép.',
    action: 'Kiểm tra cảm biến mực nước và logic điều khiển xả.',
    recovery: 'Tự phục hồi khi hết cửa sổ rate-limit 1 giờ.'
  },
  EC_OUT_OF_BOUNDS: {
    code: 'EC_OUT_OF_BOUNDS', short: 'EC vượt ngưỡng an toàn cứng.',
    reason: 'Giá trị EC vượt giới hạn an toàn cài đặt.',
    action: 'Kiểm tra cảm biến và khả năng nhiễu tín hiệu.',
    recovery: 'Tự phục hồi khi giá trị quay về ngưỡng an toàn.'
  },
  PH_OUT_OF_BOUNDS: {
    code: 'PH_OUT_OF_BOUNDS', short: 'pH vượt ngưỡng an toàn cứng.',
    reason: 'Giá trị pH vượt giới hạn an toàn cài đặt.',
    action: 'Kiểm tra cảm biến và khả năng nhiễu tín hiệu.',
    recovery: 'Tự phục hồi khi giá trị quay về ngưỡng an toàn.'
  }
};

export const getFaultGuide = (code?: string): FaultGuide | null => {
  if (!code) return null;
  return FAULT_GUIDES[code] ?? null;
};

export const FaultExplanation: React.FC<{ code: string; onClose: () => void }> = ({ code, onClose }) => {
  const guide = getFaultGuide(code);
  if (!guide) return null;

  return (
    <>
      <button className="fixed inset-0 bg-emerald-950/40 z-40" onClick={onClose} aria-label="close" />
      <div className="fixed inset-x-0 bottom-0 z-50 rounded-t-2xl border border-red-200 bg-white p-5 shadow-2xl">
        <h4 className="text-sm font-semibold text-red-700">{guide.code}</h4>
        <p className="text-xs text-emerald-800/75 mt-1">{guide.short}</p>
        <div className="mt-4 space-y-3 text-sm">
          <div><p className="text-emerald-700/75 text-xs">Nguyên nhân</p><p className="text-emerald-950">{guide.reason}</p></div>
          <div><p className="text-emerald-700/75 text-xs">Hành động ngay</p><p className="text-emerald-950">{guide.action}</p></div>
          <div><p className="text-emerald-700/75 text-xs">Tự phục hồi</p><p className="text-emerald-950">{guide.recovery}</p></div>
        </div>
      </div>
    </>
  );
};
