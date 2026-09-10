import React, { useState } from 'react';
import { ShieldCheck, Check, X, Tag } from 'lucide-react';

export interface ScanConfirmOverlayProps {
  deviceId: string;
  initialLabel?: string;
  onConfirm: (deviceId: string, label: string) => void | Promise<void>;
  onCancel: () => void;
  isSubmitting?: boolean;
}

export function deriveConfirmationCode(deviceId: string): string {
  let hash = 0;
  for (let i = 0; i < deviceId.length; i++) {
    hash = (hash << 5) - hash + deviceId.charCodeAt(i);
    hash |= 0;
  }
  const abs = Math.abs(hash);
  const num = (abs % 90 + 10).toString();
  const chars = 'ABCDEFGHJKLMNPQRSTUVWXYZ';
  const c1 = chars[(abs >> 3) % chars.length];
  const c2 = chars[(abs >> 7) % chars.length];
  return `${num} • ${c1}${c2}`;
}

export const ScanConfirmOverlay: React.FC<ScanConfirmOverlayProps> = ({
  deviceId,
  initialLabel = '',
  onConfirm,
  onCancel,
  isSubmitting = false,
}) => {
  const [label, setLabel] = useState(initialLabel);
  const confirmCode = deriveConfirmationCode(deviceId);

  const handleConfirm = () => {
    onConfirm(deviceId, label.trim());
  };

  return (
    <div
      className="fixed inset-0 z-50 bg-black/50 backdrop-blur-sm flex items-center justify-center p-4 animate-in fade-in"
      data-testid="scan-confirm-overlay"
    >
      <div className="ui-card max-w-md w-full p-6 space-y-6 shadow-2xl border border-line animate-in zoom-in-95">
        <div className="text-center space-y-2">
          <div className="w-14 h-14 rounded-2xl bg-pill text-status flex items-center justify-center mx-auto mb-3">
            <ShieldCheck size={32} />
          </div>
          <h2 className="text-xl font-bold text-primary-deep">Xác nhận ghép nối thiết bị</h2>
          <p className="text-xs text-text-muted max-w-xs mx-auto">
            Kiểm tra mã bảo mật đối chiếu hiển thị trên tem nhãn phần cứng của trạm thuỷ canh
          </p>
        </div>

        {/* Device ID and Confirmation Code */}
        <div className="p-4 rounded-2xl bg-surface-muted border border-line text-center space-y-3">
          <div>
            <span className="text-[11px] font-medium text-text-muted uppercase tracking-wider">
              Mã thiết bị (Device ID)
            </span>
            <div className="text-sm font-mono font-bold text-primary-deep" data-testid="confirm-device-id">
              {deviceId}
            </div>
          </div>

          <div className="pt-2 border-t border-line/60">
            <span className="text-[11px] font-medium text-text-muted uppercase tracking-wider">
              Mã đối chiếu bảo mật
            </span>
            <div
              className="text-3xl font-extrabold font-mono tracking-widest text-primary mt-1"
              data-testid="derived-confirm-code"
            >
              {confirmCode}
            </div>
            <p className="text-[11px] text-text-muted mt-1">
              So khớp với mã 4 ký tự in trên màn hình LCD hoặc tem vỏ trạm
            </p>
          </div>
        </div>

        {/* Optional Label input */}
        <div className="space-y-1.5">
          <label className="flex items-center gap-1.5 text-xs font-semibold text-primary-deep">
            <Tag size={14} className="text-primary" /> Đặt tên / Vị trí trạm (tuỳ chọn)
          </label>
          <input
            type="text"
            value={label}
            onChange={(e) => setLabel(e.target.value)}
            placeholder="Ví dụ: Giàn rau ban công tầng 2"
            className="w-full px-3 py-2 text-sm rounded-xl border border-line bg-white focus:outline-none focus:border-primary"
          />
        </div>

        {/* Action buttons */}
        <div className="grid grid-cols-2 gap-3 pt-2">
          <button
            type="button"
            onClick={onCancel}
            disabled={isSubmitting}
            className="ui-btn-md border border-line text-rose-600 bg-white hover:bg-rose-50 flex items-center justify-center gap-1.5"
          >
            <X size={16} /> Mã không khớp
          </button>
          <button
            type="button"
            onClick={handleConfirm}
            disabled={isSubmitting}
            className="ui-btn-primary flex items-center justify-center gap-1.5"
          >
            <Check size={16} /> {isSubmitting ? 'Đang ghép...' : 'Mã khớp & Ghép'}
          </button>
        </div>
      </div>
    </div>
  );
};
