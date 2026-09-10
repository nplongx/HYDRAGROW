import React from 'react';
import { History, CheckCircle2, Leaf, Calendar } from 'lucide-react';
import { CropSeason } from '../../types/models';
import { StateView } from '../ui/StateView';

interface SeasonHistoryListProps {
  seasons: CropSeason[];
}

export const SeasonHistoryList: React.FC<SeasonHistoryListProps> = ({ seasons }) => {
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
            <div key={season.id} className="p-4 md:p-5 hover:bg-surface-muted transition-colors">
              <div className="flex justify-between items-start mb-2">
                <h3 className="font-medium text-primary-deep">{season.name}</h3>
                {season.status === 'active' ? (
                  <span className="px-2 py-0.5 bg-pill text-status border border-pill rounded text-[10px] font-medium flex items-center gap-1.5">
                    <span className="w-1 h-1 rounded-full bg-status animate-pulse"></span>
                    Đang chạy
                  </span>
                ) : (
                  <span className="px-2 py-0.5 bg-surface-muted text-faint border border-line rounded text-[10px] font-medium flex items-center gap-1.5">
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
                </span>
              </div>
            </div>
          ))
        )}
      </div>
    </div>
  );
};
