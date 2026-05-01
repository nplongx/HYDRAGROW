import React from 'react';

export const FsmStatusBadge: React.FC<{ state?: string }> = ({ state }) => {
  const rawState = state || 'Monitoring';

  const renderBadge = (
    tone: 'default' | 'warn' | 'danger' | 'success' | 'info' | 'mist',
    content: string
  ) => {
    const toneClass =
      tone === 'danger' ? 'bg-red-500/10 border-red-500/30 text-red-400'
        : tone === 'warn' ? 'bg-amber-500/10 border-amber-500/30 text-amber-400'
          : tone === 'success' ? 'bg-emerald-500/10 border-emerald-500/30 text-emerald-400'
            : tone === 'info' ? 'bg-blue-500/10 border-blue-500/30 text-blue-400'
              : tone === 'mist' ? 'bg-cyan-500/10 border-cyan-500/30 text-cyan-400'
                : 'bg-slate-800 border-slate-700 text-slate-300';

    return (
      <span className={`px-2.5 py-0.5 rounded-md text-xs font-medium border ${toneClass}`}>
        {content}
      </span>
    );
  };

  if (rawState.startsWith('SystemFault:')) return renderBadge('danger', `Lỗi: ${rawState.replace('SystemFault:', '')}`);
  if (rawState.startsWith('EmergencyStop:')) return renderBadge('danger', `Ngắt khẩn cấp: ${rawState.replace('EmergencyStop:', '')}`);
  if (rawState.startsWith('Cooldown:')) return renderBadge('warn', 'Đang làm mát');
  if (rawState.startsWith('SensorCalibration:')) return renderBadge('info', `Calib: ${rawState.replace('SensorCalibration:', '')}`);

  switch (rawState) {
    case 'SystemBooting': return renderBadge('info', 'Đang khởi động...');
    case 'ManualMode': return renderBadge('warn', 'Chế độ thủ công');
    case 'Monitoring': return renderBadge('default', 'Đang giám sát');
    case 'DosingCycleComplete': return renderBadge('success', 'Hoàn tất chu trình');
    case 'EmergencyStop': return renderBadge('danger', 'Dừng khẩn cấp');
    case 'Disconnected':
    case 'Offline': return renderBadge('danger', 'Mất kết nối');
    case 'WaterRefilling': return renderBadge('info', 'Đang cấp nước');
    case 'WaterDraining': return renderBadge('info', 'Đang xả nước');
    case 'StartingOsakaPump': return renderBadge('default', 'Khởi động máy trộn');
    case 'DosingPumpA': return renderBadge('default', 'Đang châm Phân A');
    case 'WaitingBetweenDose': return renderBadge('warn', 'Chờ hòa tan A → B');
    case 'DosingPumpB': return renderBadge('default', 'Đang châm Phân B');
    case 'DosingPH': return renderBadge('default', 'Đang chỉnh pH');
    case 'ActiveMixing': return renderBadge('info', 'Đang sục trộn');
    case 'Stabilizing': return renderBadge('warn', 'Chờ ổn định');
    case 'Misting': return renderBadge('mist', 'Đang phun sương');
    case 'enter_calibration': return renderBadge('info', 'Vào chế độ Calib');
    case 'exit_calibration': return renderBadge('success', 'Thoát Calib');
    default: return renderBadge('default', rawState);
  }
};
