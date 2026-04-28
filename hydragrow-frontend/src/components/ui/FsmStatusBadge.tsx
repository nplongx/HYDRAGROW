import React from 'react';

export const FsmStatusBadge: React.FC<{ state?: string }> = ({ state }) => {
  const rawState = state || 'Monitoring';

  const renderBadge = (tone: 'default' | 'warn' | 'danger', content: string) => {
    const toneClass = tone === 'danger'
      ? 'bg-rose-500/10 border-rose-500/40 text-rose-300'
      : tone === 'warn'
        ? 'bg-amber-500/10 border-amber-500/40 text-amber-200'
        : 'bg-slate-800 border-slate-700 text-slate-200';

    return (
      <span className={`px-3 py-1 rounded-full text-xs font-semibold border ${toneClass}`}>
        {content}
      </span>
    );
  };

  if (rawState.startsWith('SystemFault:')) {
    const reason = rawState.replace('SystemFault:', '');
    return renderBadge('danger', `Lỗi: ${reason}`);
  }

  if (rawState.startsWith('EmergencyStop:')) {
    const reason = rawState.replace('EmergencyStop:', '');
    return renderBadge('danger', `Dừng khẩn cấp: ${reason}`);
  }

  switch (rawState) {
    case 'Monitoring': return renderBadge('default', 'Đang giám sát');
    case 'EmergencyStop': return renderBadge('danger', 'Dừng khẩn cấp');
    case 'Disconnected':
    case 'Offline':
      return renderBadge('danger', 'Mất kết nối');
    case 'Stabilizing':
    case 'WaitingBetweenDose':
      return renderBadge('warn', rawState === 'Stabilizing' ? 'Chờ ổn định' : 'Chờ hòa tan');
    case 'WaterRefilling': return renderBadge('default', 'Đang cấp nước');
    case 'WaterDraining': return renderBadge('default', 'Đang xả nước');
    case 'DosingPumpA': return renderBadge('default', 'Đang châm phân A');
    case 'DosingPumpB': return renderBadge('default', 'Đang châm phân B');
    case 'DosingPH': return renderBadge('default', 'Đang chỉnh pH');
    case 'StartingOsakaPump': return renderBadge('default', 'Khởi động bơm');
    case 'ActiveMixing': return renderBadge('default', 'Đang sục trộn');
    case 'enter_calibration': return renderBadge('default', 'Đang hiệu chỉnh cảm biến');
    case 'exit_calibration': return renderBadge('default', 'Đã hiệu chỉnh cảm biến');

    default: return renderBadge('default', rawState);
  }
};
