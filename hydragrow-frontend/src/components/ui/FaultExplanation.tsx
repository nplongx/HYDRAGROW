import React from 'react';
import { AlertTriangle, X, Wrench, Info } from 'lucide-react';
import { get_fault_guide } from '../../../gleam_core/build/dev/javascript/gleam_core/faults.mjs';

export const getFaultGuide = (code?: string) => {
  if (!code) return null;
  const guide = get_fault_guide(code);
  // Unwrap Gleam Option (Some [0] / None)
  if (guide && guide[0]) {
    return guide[0];
  }
  return null;
};

interface FaultExplanationProps {
  code: string;
  onClose: () => void;
}

export const FaultExplanation: React.FC<FaultExplanationProps> = ({ code, onClose }) => {
  const guide = getFaultGuide(code);

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-emerald-950/40 backdrop-blur-sm animate-in fade-in duration-200">
      <div className="bg-white border border-red-200 rounded-2xl max-w-lg w-full p-6 shadow-2xl space-y-4 animate-in zoom-in-95 duration-200">
        <div className="flex items-start justify-between">
          <div className="flex items-center gap-3">
            <div className="p-2.5 bg-red-100 text-red-700 rounded-xl">
              <AlertTriangle size={20} />
            </div>
            <div>
              <h3 className="text-base font-bold text-emerald-950">Chi tiết sự cố</h3>
              <p className="text-xs font-mono font-semibold text-red-600 mt-0.5">Mã lỗi: {code}</p>
            </div>
          </div>
          <button
            onClick={onClose}
            className="p-1.5 rounded-lg text-emerald-800/70 hover:text-emerald-950 hover:bg-emerald-50 transition-colors"
          >
            <X size={18} />
          </button>
        </div>

        {guide ? (
          <div className="space-y-3 pt-2">
            {guide.short && (
              <div className="p-3 bg-red-50 border border-red-100 rounded-xl">
                <p className="text-xs text-red-900 leading-relaxed font-medium">{guide.short}</p>
              </div>
            )}
            {guide.cause && (
              <div className="space-y-1">
                <span className="text-[11px] font-bold uppercase tracking-wider text-emerald-800 flex items-center gap-1.5">
                  <Info size={13} className="text-emerald-700" /> Nguyên nhân khả dĩ
                </span>
                <p className="text-xs text-emerald-900 bg-emerald-50/60 border border-emerald-100 p-2.5 rounded-lg leading-relaxed">
                  {guide.cause}
                </p>
              </div>
            )}
            {guide.action && (
              <div className="space-y-1">
                <span className="text-[11px] font-bold uppercase tracking-wider text-emerald-800 flex items-center gap-1.5">
                  <Wrench size={13} className="text-emerald-700" /> Hướng dẫn khắc phục
                </span>
                <p className="text-xs font-semibold text-emerald-900 bg-emerald-100/60 border border-emerald-200 p-2.5 rounded-lg leading-relaxed">
                  {guide.action}
                </p>
              </div>
            )}
          </div>
        ) : (
          <div className="py-4 text-center text-xs text-emerald-700/75">
            Chưa có tài liệu hướng dẫn cụ thể cho mã lỗi này.
          </div>
        )}

        <div className="pt-2 flex justify-end">
          <button
            onClick={onClose}
            className="px-4 py-2 bg-emerald-100 hover:bg-emerald-200 text-emerald-900 text-xs font-bold rounded-xl transition-colors"
          >
            Đóng
          </button>
        </div>
      </div>
    </div>
  );
};
