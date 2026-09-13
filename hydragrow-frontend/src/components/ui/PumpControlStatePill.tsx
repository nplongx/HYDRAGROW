// hydragrow-frontend/src/components/ui/PumpControlStatePill.tsx
import React from 'react';
import type { PumpControlState, PumpLockReason } from '../../lib/dosing/pumpControlStateMachine';
import { LOCK_REASON_COPY } from '../../lib/dosing/pumpControlStateMachine';

interface PumpControlStatePillProps {
  state: PumpControlState['state'];
  reason: PumpLockReason | null;
  className?: string;
}

const META: Record<PumpControlState['state'], { label: string; classes: string; dot: string }> = {
  idle: { label: 'Sẵn sàng', classes: 'bg-surface-muted text-faint', dot: 'bg-faint' },
  running: { label: 'Đang chạy', classes: 'bg-success-bg text-status', dot: 'bg-status' },
  locked: { label: 'Đã khoá', classes: 'bg-warning-bg text-warn-deep', dot: 'bg-warn-deep' },
};

export const PumpControlStatePill: React.FC<PumpControlStatePillProps> = ({ state, reason, className }) => {
  const meta = META[state];
  const title = reason ? LOCK_REASON_COPY[reason] : undefined;
  return (
    <span
      title={title}
      className={`inline-flex items-center gap-1.5 rounded-full border border-transparent px-2.5 py-0.5 text-[10px] font-bold ${meta.classes} ${className ?? ''}`}
    >
      <span aria-hidden="true" className={`h-1.5 w-1.5 rounded-full ${meta.dot}`} />
      {meta.label}
    </span>
  );
};
