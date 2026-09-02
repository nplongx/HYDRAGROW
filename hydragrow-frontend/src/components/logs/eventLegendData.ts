export interface LegendEntry {
  label: string;
  swatchClassName: string;
  description: string;
}

export const EVENT_LEGEND_ENTRIES: LegendEntry[] = [
  { label: 'Nghiêm trọng', swatchClassName: 'bg-rose-500', description: 'Sự cố cần xử lý ngay (Danger)' },
  { label: 'Cảnh báo', swatchClassName: 'bg-amber-500', description: 'Cần chú ý, gần ngưỡng (Warning)' },
  { label: 'Châm vi chất', swatchClassName: 'bg-cyan-400', description: 'Sự kiện châm EC/pH (Dosing)' },
  { label: 'Nước', swatchClassName: 'bg-sky-400', description: 'Cấp/xả nước (Water)' },
  { label: 'Hiệu chuẩn', swatchClassName: 'bg-purple-400', description: 'Calibration debug, cập nhật EMA' },
  { label: 'Người dùng / hệ thống', swatchClassName: 'bg-emerald-500', description: 'Thao tác thủ công, khởi động, kết nối (Success/Primary)' },
  { label: 'Kỹ thuật đã gộp', swatchClassName: 'log-neutral-dot', description: 'FSM transition / calibration debug / sensor reading lặp lại, gộp 1 dòng ở chế độ Quan trọng' },
];
