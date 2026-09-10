import React from 'react';
import { Droplets, Pause, Bell, Sprout } from 'lucide-react';

interface QuickActionBarProps {
  onWaterNow: () => void;
  onDose: () => void;
  onPausePumps: () => void;
  onViewAlerts: () => void;
  pumpsPaused?: boolean;
}

export const QuickActionBar: React.FC<QuickActionBarProps> = ({
  onWaterNow,
  onDose,
  onPausePumps,
  onViewAlerts,
  pumpsPaused = false,
}) => (
  <div className="space-y-3">
    <h3 className="farm-section-title">
      <span aria-hidden>⚡</span>
      <span>Thao tác nhanh</span>
    </h3>
    <div className="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-4 gap-3">
      <button type="button" onClick={onWaterNow} className="flex items-center justify-center gap-2 rounded-[14px] border border-line bg-white px-[18px] py-[14px] text-[13px] font-semibold text-primary-deep hover:bg-soft transition-colors">
        <Sprout size={15} className="text-status" />
        Tưới ngay
      </button>
      <button type="button" onClick={onDose} className="ui-btn-primary flex items-center justify-center gap-2">
        <Droplets size={15} />
        Châm dinh dưỡng
      </button>
      <button type="button" onClick={onPausePumps} className="flex items-center justify-center gap-2 rounded-[14px] border border-line bg-white px-[18px] py-[14px] text-[13px] font-semibold text-primary-deep hover:bg-soft transition-colors">
        <Pause size={15} />
        {pumpsPaused ? 'Tiếp tục bơm' : 'Tạm dừng bơm'}
      </button>
      <button type="button" onClick={onViewAlerts} className="flex items-center justify-center gap-2 rounded-[14px] border border-line bg-white px-[18px] py-[14px] text-[13px] font-semibold text-primary-deep hover:bg-soft transition-colors">
        <Bell size={15} />
        Xem cảnh báo
      </button>
    </div>
  </div>
);
