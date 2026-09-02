import React from 'react';

interface DosingSummaryCardProps {
  totalCount: number;
  lastDosedAt: number | null;
}

export const DosingSummaryCard: React.FC<DosingSummaryCardProps> = ({ totalCount, lastDosedAt }) => {
  const lastDosedLabel = lastDosedAt
    ? new Date(lastDosedAt).toLocaleTimeString('vi-VN', { hour: '2-digit', minute: '2-digit' })
    : null;
  return (
    <div className="ui-card space-y-1.5">
      <h4 className="text-sm font-bold text-emerald-800">Châm dinh dưỡng hôm nay</h4>
      {lastDosedLabel ? (
        <p className="text-[13px] text-emerald-700/75">{totalCount} lần · lần cuối {lastDosedLabel}</p>
      ) : (
        <p className="text-[13px] text-emerald-700/75">Chưa ghi nhận lần châm nào hôm nay</p>
      )}
    </div>
  );
};
