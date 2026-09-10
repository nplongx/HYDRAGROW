import { AlertOctagon, X } from 'lucide-react';
import { pumpLabels } from '../../lib/pumpLabels';

interface EmergencyStopConfirmDialogProps {
  open: boolean;
  runningPumps: Record<string, boolean>;
  onCancel: () => void;
  onConfirm: () => void;
  isSubmitting: boolean;
}

export const EmergencyStopConfirmDialog = ({
  open,
  runningPumps,
  onCancel,
  onConfirm,
  isSubmitting,
}: EmergencyStopConfirmDialogProps) => {
  if (!open) return null;

  const runningKeys = Object.entries(runningPumps)
    .filter(([, isOn]) => isOn)
    .map(([key]) => key);

  return (
    <div className="fixed inset-0 z-[60] bg-[#0F1F14]/40 backdrop-blur-sm flex items-end sm:items-center justify-center p-4">
      <div className="bg-white rounded-3xl shadow-2xl w-full max-w-sm p-6 space-y-4">
        <div className="flex items-start gap-3">
          <div className="p-2.5 bg-red-50 border border-red-200 rounded-2xl text-red-600 shrink-0">
            <AlertOctagon size={22} />
          </div>
          <div>
            <h3 className="text-base font-bold text-red-700">Dừng khẩn cấp toàn hệ thống</h3>
            <p className="text-xs text-text-muted mt-1">
              Toàn bộ bơm và van đang chạy sẽ tắt ngay lập tức. Bạn cần bấm "Khôi phục" ở màn Vận hành để chạy lại sau đó.
            </p>
          </div>
        </div>

        <div className="space-y-1.5">
          <p className="text-[10px] font-bold uppercase tracking-wider text-faint">Thiết bị đang chạy sẽ bị tắt</p>
          {runningKeys.length === 0 ? (
            <p className="text-xs text-text-muted italic">Không có thiết bị nào đang chạy.</p>
          ) : (
            <div className="flex flex-wrap gap-1.5">
              {runningKeys.map((key) => (
                <span key={key} className="px-2.5 py-1 rounded-full text-xs font-semibold bg-red-50 text-red-700 border border-red-200">
                  {pumpLabels[key] || key}
                </span>
              ))}
            </div>
          )}
        </div>

        <div className="flex gap-2 pt-2">
          <button
            onClick={onCancel}
            disabled={isSubmitting}
            className="flex-1 flex items-center justify-center gap-1.5 px-4 py-2.5 rounded-xl border border-line text-primary-deep text-sm font-bold hover:bg-soft transition-all disabled:opacity-50 cursor-pointer"
          >
            <X size={14} /> Huỷ
          </button>
          <button
            onClick={onConfirm}
            disabled={isSubmitting}
            className="flex-1 px-4 py-2.5 rounded-xl bg-red-600 hover:bg-red-700 text-white text-sm font-bold transition-all disabled:opacity-50 cursor-pointer"
          >
            {isSubmitting ? 'Đang dừng…' : 'Xác nhận dừng khẩn cấp'}
          </button>
        </div>
      </div>
    </div>
  );
};
