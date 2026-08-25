import React from 'react';
import { History, CheckCircle2, Leaf, Calendar } from 'lucide-react';
import { CropSeason } from '../../types/models';
import { StateView } from '../ui/StateView';

interface SeasonHistoryListProps {
  seasons: CropSeason[];
}

export const SeasonHistoryList: React.FC<SeasonHistoryListProps> = ({ seasons }) => {
  return (
    <div className="ui-card space-y-4">
      <div className="border-b border-emerald-100 pb-3">
        <h2 className="text-sm font-semibold text-emerald-950 flex items-center gap-2">
          <History size={18} className="text-emerald-800/80" />
          Lịch sử các mùa vụ
        </h2>
      </div>

      <div className="space-y-3">
        {seasons.length === 0 ? (
          <div className="p-4">
            <StateView icon={History} title="Chưa có lịch sử mùa vụ" className="border-none bg-transparent" />
          </div>
        ) : (
          seasons.map((season) => {
            const isCompleted = season.status !== 'active';
            const leftBorder = isCompleted ? 'border-l-4 border-l-emerald-500' : 'border-l-4 border-l-amber-400';
            return (
              <div key={season.id} className={`ui-card ui-card-hover p-4 ${leftBorder}`}>
                <div className="flex justify-between items-start mb-2">
                  <h3 className="font-bold text-emerald-950 text-sm">{season.name}</h3>
                  {season.status === 'active' ? (
                    <span className="px-2 py-0.5 bg-emerald-100 text-emerald-700 border border-emerald-200 rounded-full text-[10px] font-bold flex items-center gap-1.5">
                      <span className="w-1.5 h-1.5 rounded-full bg-emerald-500 animate-pulse"></span>
                      Đang chạy
                    </span>
                  ) : (
                    <span className="px-2 py-0.5 bg-emerald-50 text-emerald-800 border border-emerald-200 rounded-full text-[10px] font-bold flex items-center gap-1.5">
                      <CheckCircle2 size={10} /> Đã hoàn thành
                    </span>
                  )}
                </div>
                <div className="flex flex-col sm:flex-row sm:items-center gap-2 sm:gap-4 text-xs font-medium text-emerald-700/75">
                  <span className="flex items-center gap-1.5">
                    <Leaf size={14} className="text-emerald-800/80" />
                    {season.plant_type || 'Chưa cập nhật'}
                  </span>
                  <span className="flex items-center gap-1.5">
                    <Calendar size={14} className="text-emerald-800/80" />
                    {new Date(season.start_time).toLocaleDateString('vi-VN')}
                    {season.end_time ? ` → ${new Date(season.end_time).toLocaleDateString('vi-VN')}` : ' → Nay'}
                  </span>
                </div>
              </div>
            );
          })
        )}
      </div>
    </div>
  );
};
