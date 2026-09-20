import React from 'react';
import { Droplets, Pause, Bell, Sprout, Zap } from 'lucide-react';

interface QuickActionBarProps {
  onWaterNow: () => void;
  onDose: () => void;
  onPausePumps: () => void;
  onViewAlerts: () => void;
  pumpsPaused?: boolean;
  commandStatus?: string;
}

export const QuickActionBar: React.FC<QuickActionBarProps> = ({
  onWaterNow,
  onDose,
  onPausePumps,
  onViewAlerts,
  pumpsPaused = false,
  commandStatus,
}) => (
  <div className="space-y-3">
    <h3 className="farm-section-title">
      <Zap size={14} aria-hidden />
      <span>Thao tác nhanh</span>
    </h3>
    <div className="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-4 gap-3">
      <button type="button" onClick={onWaterNow} className="ui-btn-outline w-full" disabled={['REQUESTED', 'SENT', 'ACKNOWLEDGED'].includes(commandStatus ?? '')}>
        <Sprout size={15} className="text-status" />
        {commandStatus === 'REQUESTED' ? 'Đang yêu cầu tưới…' : commandStatus === 'SENT' || commandStatus === 'ACKNOWLEDGED' ? 'Đang chờ xác nhận…' : 'Tưới ngay'}
      </button>
      <button type="button" onClick={onDose} className="ui-btn-primary flex items-center justify-center gap-2">
        <Droplets size={15} />
        Châm dinh dưỡng
      </button>
      <button type="button" onClick={onPausePumps} className="ui-btn-outline w-full">
        <Pause size={15} />
        {pumpsPaused ? 'Tiếp tục bơm' : 'Tạm dừng bơm'}
      </button>
      <button type="button" onClick={onViewAlerts} className="ui-btn-outline w-full">
        <Bell size={15} />
        Xem cảnh báo
      </button>
      {commandStatus && commandStatus !== 'CONFIRMED' && commandStatus !== 'IDLE' && (
        <p className="sm:col-span-2 xl:col-span-4 text-xs text-text-muted" role="status">
          Lệnh tưới: {commandStatus === 'REQUESTED' ? 'đã yêu cầu' : commandStatus === 'SENT' || commandStatus === 'ACKNOWLEDGED' ? 'đã gửi, đang chờ xác nhận vật lý' : commandStatus === 'TIMEOUT' ? 'hết thời gian xác nhận' : commandStatus === 'FAILED' || commandStatus === 'REJECTED' ? 'không thành công' : commandStatus.toLowerCase()}
        </p>
      )}
    </div>
  </div>
);
