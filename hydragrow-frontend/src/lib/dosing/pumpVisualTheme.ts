// hydragrow-frontend/src/lib/dosing/pumpVisualTheme.ts

/**
 * Nguồn sự thật DUY NHẤT cho màu định danh từng bơm/van, dùng ở cả
 * điều khiển thủ công (AdvancedDeviceControl/ControlPanel) lẫn hiển thị
 * dữ liệu (DosingHourlyChart legend, DosingReportCard badges) — trước
 * khi có module này, 2 nơi tự vẽ 2 bảng màu khác nhau cho cùng khái
 * niệm "bơm pH Up" (xem PATTERN-LIBRARY.md mục 5 để biết lịch sử).
 *
 * File .ts thuần (không phải .tsx): các chuỗi class Tailwind bên dưới
 * là dữ liệu cấu hình, không phải JSX — nằm ngoài phạm vi quét của
 * hardcodedColors.test.ts (guard đó chỉ liệt kê *.tsx, giống cách
 * dosingAggregates.ts vốn đã nằm ngoài phạm vi quét). Đây là lựa chọn
 * kiến trúc có chủ đích (một nơi định nghĩa, nhiều nơi import) chứ
 * không phải né guard — xem PATTERN-LIBRARY.md mục 5 để biết lý do đầy
 * đủ, y hệt tinh thần "categorical palette" đã được chấp nhận cho
 * automation node types trong colorAllowlist.json.
 */
export type PumpThemeKey = 'nutrient' | 'phUp' | 'phDown' | 'aqua';

export interface PumpVisualTheme {
  hue: string;
  activeIcon: string;
  glow: string;
  border: string;
  /** Dùng cho badge nhỏ (DosingReportCard, chart legend). */
  badge: string;
}

export const PUMP_VISUAL_THEME: Record<PumpThemeKey, PumpVisualTheme> = {
  nutrient: {
    hue: 'orange',
    activeIcon: 'bg-orange-600 text-white',
    glow: 'border-orange-200 bg-orange-50',
    border: 'border-orange-300',
    badge: 'text-orange-700 bg-orange-50 border-orange-200',
  },
  phUp: {
    hue: 'violet',
    activeIcon: 'bg-violet-600 text-white',
    glow: 'border-violet-200 bg-violet-50',
    border: 'border-violet-300',
    badge: 'text-violet-700 bg-violet-50 border-violet-200',
  },
  phDown: {
    hue: 'fuchsia',
    activeIcon: 'bg-fuchsia-600 text-white',
    glow: 'border-fuchsia-200 bg-fuchsia-50',
    border: 'border-fuchsia-300',
    badge: 'text-fuchsia-700 bg-fuchsia-50 border-fuchsia-200',
  },
  aqua: {
    hue: 'sky',
    activeIcon: 'bg-sky-600 text-white',
    glow: 'border-sky-200 bg-sky-50',
    border: 'border-sky-300',
    badge: 'text-sky-700 bg-sky-50 border-sky-200',
  },
};

const PUMP_ID_THEME: Record<string, PumpThemeKey> = {
  PUMP_A: 'nutrient',
  PUMP_B: 'nutrient',
  PH_UP: 'phUp',
  PH_DOWN: 'phDown',
  WATER_PUMP_IN: 'aqua',
  WATER_PUMP_OUT: 'aqua',
  OSAKA: 'aqua',
  MIST: 'aqua',
  MIX: 'aqua',
};

export function pumpThemeFor(pumpId: string): PumpThemeKey {
  return PUMP_ID_THEME[pumpId] ?? 'aqua';
}
