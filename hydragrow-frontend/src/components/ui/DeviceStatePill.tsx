import React from 'react';

export type DeviceState = 'online' | 'offline' | 'warning' | 'dosing' | 'auto' | 'manual';

interface DeviceStatePillProps {
  state: DeviceState | string;
  label?: string;
  className?: string;
}

const META: Record<DeviceState, { label: string; classes: string; dot: string }> = {
  online: { label: 'Trực tuyến', classes: 'bg-success-bg text-status', dot: 'bg-status' },
  offline: { label: 'Ngoại tuyến', classes: 'bg-surface-muted text-faint', dot: 'bg-faint' },
  warning: { label: 'Cảnh báo', classes: 'bg-warning-bg text-warn-deep', dot: 'bg-warn-deep' },
  dosing: { label: 'Đang châm', classes: 'bg-info-bg text-info-fg', dot: 'bg-info-fg' },
  auto: { label: 'Tự động', classes: 'bg-pill text-primary', dot: 'bg-primary' },
  manual: { label: 'Thủ công', classes: 'bg-surface-muted text-text-muted', dot: 'bg-text-muted' },
};

export const DeviceStatePill: React.FC<DeviceStatePillProps> = ({ state, label, className }) => {
  const meta = META[state as DeviceState] ?? META.manual;
  return (
    <span
      className={`inline-flex items-center gap-1.5 rounded-full border border-transparent px-2.5 py-0.5 text-[10px] font-bold ${meta.classes} ${className ?? ''}`}
    >
      <span aria-hidden="true" className={`h-1.5 w-1.5 rounded-full ${meta.dot}`} />
      {label ?? meta.label}
    </span>
  );
};