import { PartyPopper, Calendar, Image as ImageIcon, ArrowRight } from 'lucide-react';

interface SeasonCompletionSummaryProps {
  seasonName: string;
  totalDaysGrown: number;
  photoCount: number;
  onClose: () => void;
}

/**
 * Màn "peak" khi kết thúc 1 mùa vụ — peak-end-rule: "the final moment of
 * a session shapes overall impression more than most of what preceded
 * it". Trước khi có component này, kết thúc mùa vụ chuyển thẳng về
 * CreateSeasonForm trống — đúng phản-pattern skill mô tả ("never end on
 * an administrative or transitional screen").
 */
export const SeasonCompletionSummary = ({ seasonName, totalDaysGrown, photoCount, onClose }: SeasonCompletionSummaryProps) => (
  <div
    className="fixed inset-0 z-50 bg-black/50 backdrop-blur-sm flex items-center justify-center p-4 animate-in fade-in"
    data-testid="season-completion-summary"
  >
    <div className="ui-card max-w-md w-full p-6 space-y-6 text-center shadow-2xl border border-line animate-in zoom-in-95">
      <div className="w-14 h-14 rounded-2xl bg-pill text-status flex items-center justify-center mx-auto">
        <PartyPopper size={32} />
      </div>
      <div className="space-y-1">
        <h2 className="text-xl font-bold text-primary-deep">Đã hoàn thành mùa vụ!</h2>
        <p className="text-base font-bold text-primary">{seasonName}</p>
      </div>

      <div className="grid grid-cols-2 gap-3">
        <div className="p-3 rounded-xl bg-surface-muted border border-line space-y-1">
          <Calendar size={18} className="text-primary mx-auto" />
          <p className="text-lg font-black text-primary-deep">{totalDaysGrown} ngày</p>
          <p className="text-[10px] text-text-muted uppercase tracking-wide">Thời gian canh tác</p>
        </div>
        <div className="p-3 rounded-xl bg-surface-muted border border-line space-y-1">
          <ImageIcon size={18} className="text-primary mx-auto" />
          <p className="text-lg font-black text-primary-deep">{photoCount} ảnh</p>
          <p className="text-[10px] text-text-muted uppercase tracking-wide">Nhật ký hình ảnh</p>
        </div>
      </div>

      <button
        type="button"
        onClick={onClose}
        className="w-full flex items-center justify-center gap-2 py-2.5 bg-primary hover:bg-primary-deep text-white rounded-lg font-bold text-sm transition-colors"
      >
        Bắt đầu mùa vụ tiếp theo <ArrowRight size={16} />
      </button>
    </div>
  </div>
);
