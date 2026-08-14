import React from 'react';
import { History, CheckCircle2, Leaf, Calendar } from 'lucide-react';
import { CropSeason } from '../../types/models';
import { StateView } from '../ui/StateView';

interface SeasonHistoryListProps {
  seasons: CropSeason[];
}

export const SeasonHistoryList: React.FC<SeasonHistoryListProps> = ({ seasons }) => {
  return (
    <div className="bg-white border border-emerald-100 rounded-xl overflow-hidden shadow-sm">
      <div className="p-4 md:p-5 border-b border-emerald-100 bg-emerald-50/70">
        <h2 className="text-sm font-semibold text-emerald-950 flex items-center gap-2">
          <History size={18} className="text-emerald-800/80" />
          Lịch sử các mùa vụ
        </h2>
      </div>

      <div className="divide-y divide-emerald-100">
        {seasons.length === 0 ? (
          <div className="p-8">
            <StateView icon={History} title="Chưa có lịch sử mùa vụ" className="border-none bg-transparent" />
          </div>
        ) : (
          seasons.map((season) => (
            <div key={season.id} className="p-4 md:p-5 hover:bg-emerald-50/40 transition-colors">
              <div className="flex justify-between items-start mb-2">
                <h3 className="font-medium text-emerald-950">{season.name}</h3>
                {season.status === 'active' ? (
                  <span className="px-2 py-0.5 bg-emerald-500/10 text-emerald-700 border border-emerald-500/20 rounded text-[10px] font-medium flex items-center gap-1.5">
                    <span className="w-1 h-1 rounded-full bg-emerald-500 animate-pulse"></span>
                    Đang chạy
                  </span>
                ) : (
                  <span className="px-2 py-0.5 bg-emerald-100 text-emerald-800 border border-emerald-200 rounded text-[10px] font-medium flex items-center gap-1.5">
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
          ))
        )}
      </div>
    </div>
  );
};
