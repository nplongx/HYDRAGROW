import React from 'react';
import { History, CheckCircle2, Leaf, Calendar, ChevronRight, Trash2 } from 'lucide-react';
import { CropSeason } from '../../types/models';
import { StateView } from '../ui/StateView';

interface SeasonHistoryListProps {
  seasons: CropSeason[];
  onSelect?: (season: CropSeason) => void;
  onDelete?: (season: CropSeason) => void;
}

const formatDays = (season: CropSeason): string => {
  const start = new Date(season.start_time).getTime();
  const end = season.end_time ? new Date(season.end_time).getTime() : Date.now();
  const days = Math.round((end - start) / 86400000);
  return `${Math.max(0, days)} ngày`;
};

export const SeasonHistoryList: React.FC<SeasonHistoryListProps> = ({ seasons, onSelect, onDelete }) => {
  return (
    <div className="bg-white border border-line rounded-xl overflow-hidden shadow-sm">
      <div className="p-4 md:p-5 border-b border-line bg-surface-muted">
        <h2 className="text-sm font-semibold text-primary-deep flex items-center gap-2">
          <History size={18} className="text-primary/70" />
          Lịch sử các mùa vụ
        </h2>
      </div>

      <div className="divide-y divide-line">
        {seasons.length === 0 ? (
          <div className="p-8">
            <StateView icon={History} title="Chưa có lịch sử mùa vụ" className="border-none bg-transparent" />
          </div>
        ) : (
          seasons.map((season) => (
            <button
              key={season.id}
              type="button"
              onClick={() => onSelect?.(season)}
              className={`w-full text-left p-4 md:p-5 hover:bg-surface-muted transition-colors flex items-center gap-3 ${
                onSelect ? 'cursor-pointer' : 'cursor-default'
              }`}
            >
              <div className="flex-1 min-w-0">
                <div className="flex justify-between items-start mb-2">
                  <h3 className="font-medium text-primary-deep truncate">{season.name}</h3>
                  {season.status === 'active' ? (
                    <span className="px-2 py-0.5 bg-pill text-status border border-pill rounded text-[10px] font-medium flex items-center gap-1.5 shrink-0">
                      <span className="w-1 h-1 rounded-full bg-status animate-pulse"></span>
                      Đang chạy
                    </span>
                  ) : (
                    <span className="px-2 py-0.5 bg-surface-muted text-faint border border-line rounded text-[10px] font-medium flex items-center gap-1.5 shrink-0">
                      <CheckCircle2 size={10} /> Đã hoàn thành
                    </span>
                  )}
                </div>
                <div className="flex flex-col sm:flex-row sm:items-center gap-2 sm:gap-4 text-xs font-medium text-text-muted">
                  <span className="flex items-center gap-1.5">
                    <Leaf size={14} className="text-primary/60" />
                    {season.plant_type || 'Chưa cập nhật'}
                  </span>
                  <span className="flex items-center gap-1.5">
                    <Calendar size={14} className="text-primary/60" />
                    {new Date(season.start_time).toLocaleDateString('vi-VN')}
                    {season.end_time ? ` → ${new Date(season.end_time).toLocaleDateString('vi-VN')}` : ' → Nay'}
                    <span className="text-faint">· {formatDays(season)}</span>
                  </span>
                </div>
              </div>
              <div className="flex items-center gap-2 shrink-0">
                {onDelete && season.status !== 'active' && (
                  <button
                    type="button"
                    aria-label={`Xoá mùa vụ ${season.name}`}
                    onClick={(e) => {
                      e.stopPropagation();
                      if (window.confirm(`Xoá mùa vụ "${season.name}" và toàn bộ ảnh của mùa vụ này?`)) {
                        onDelete(season);
                      }
                    }}
                    className="p-2 text-faint hover:text-red-600 hover:bg-red-50 rounded-lg transition-colors"
                  >
                    <Trash2 size={15} />
                  </button>
                )}
                {onSelect && <ChevronRight size={16} className="text-line" />}
              </div>
            </button>
          ))
        )}
      </div>
    </div>
  );
};
