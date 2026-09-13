// hydragrow-frontend/src/lib/logs/eventCategoryTheme.ts

/**
 * Nguồn sự thật duy nhất cho màu phân loại sự kiện nhật ký (EC dosing / pH
 * dosing / nước / cảnh báo). Trước khi có module này, HealthSummaryBar,
 * EventLogCard, CycleEventCard, MetadataRenderers, FsmStatusBadge mỗi nơi
 * tự chọn 1 hue Tailwind rời rạc cho cùng khái niệm — file .ts thuần, nằm
 * ngoài phạm vi quét của hardcodedColors.test.ts (chỉ liệt kê *.tsx), lý
 * do kiến trúc giống hệt src/lib/dosing/pumpVisualTheme.ts (Track A).
 */
export type EventCategoryThemeKey = 'ecDosing' | 'phDosing' | 'water' | 'warning' | 'automation' | 'device';

export const EVENT_CATEGORY_THEME: Record<EventCategoryThemeKey, { icon: string; badge: string }> = {
  ecDosing: { icon: 'text-fuchsia-700', badge: 'text-fuchsia-700 bg-fuchsia-50 border-fuchsia-200' },
  phDosing: { icon: 'text-violet-700', badge: 'text-violet-700 bg-violet-50 border-violet-200' },
  water: { icon: 'text-sky-700', badge: 'text-sky-700 bg-sky-50 border-sky-200' },
  warning: { icon: 'text-warn-deep', badge: 'text-warn-deep bg-warning-bg border-warn-deep/25' },
  automation: { icon: 'text-primary', badge: 'text-primary bg-pill border-primary/25' },
  device: { icon: 'text-cyan-700', badge: 'text-cyan-700 bg-cyan-50 border-cyan-200' },
};
