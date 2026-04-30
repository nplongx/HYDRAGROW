import React from 'react';

export const FsmStatusBadge: React.FC<{ state?: string }> = ({ state }) => {
  const rawState = state || 'Monitoring';

  const renderBadge = (
    tone: 'default' | 'warn' | 'danger' | 'success' | 'info' | 'mist',
    content: string
  ) => {
    const toneClass =
      tone === 'danger'
        ? 'bg-rose-500/10 border-rose-500/40 text-rose-300'
        : tone === 'warn'
          ? 'bg-amber-500/10 border-amber-500/40 text-amber-200'
          : tone === 'success'
            ? 'bg-emerald-500/10 border-emerald-500/40 text-emerald-300'
            : tone === 'info'
              ? 'bg-indigo-500/10 border-indigo-500/40 text-indigo-300'
              : tone === 'mist'
                ? 'bg-sky-500/10 border-sky-500/40 text-sky-300'
                : 'bg-slate-800 border-slate-700 text-slate-200';

    return (
      <span
        className={`px-3 py-1 rounded-full text-xs font-semibold border ${toneClass}`}
      >
        {content}
      </span>
    );
  };

  // --- Trạng thái có prefix động ---
  if (rawState.startsWith('SystemFault:')) {
    const reason = rawState.replace('SystemFault:', '');
    return renderBadge('danger', `Lỗi: ${reason}`);
  }
  if (rawState.startsWith('EmergencyStop:')) {
    const reason = rawState.replace('EmergencyStop:', '');
    return renderBadge('danger', `Dừng khẩn cấp: ${reason}`);
  }
  if (rawState.startsWith('Cooldown:')) {
    return renderBadge('warn', 'Đang làm mát (Cooldown)');
  }
  if (rawState.startsWith('SensorCalibration:')) {
    const step = rawState.replace('SensorCalibration:', '');
    return renderBadge('info', `Hiệu chuẩn: ${step}`);
  }

  // --- Trạng thái cố định ---
  switch (rawState) {
    // Khởi động & Chờ
    case 'SystemBooting':
      return renderBadge('info', 'Đang khởi động...');
    case 'ManualMode':
      return renderBadge('warn', 'Thủ công');
    case 'Monitoring':
      return renderBadge('default', 'Đang giám sát');
    case 'DosingCycleComplete':
      return renderBadge('success', 'Hoàn tất chu trình');

    // Lỗi & Khẩn cấp
    case 'EmergencyStop':
      return renderBadge('danger', 'Dừng khẩn cấp');
    case 'Disconnected':
    case 'Offline':
      return renderBadge('danger', 'Mất kết nối');

    // Nước
    case 'WaterRefilling':
      return renderBadge('info', 'Đang cấp nước');
    case 'WaterDraining':
      return renderBadge('info', 'Đang xả nước');

    // Bơm khởi động & Châm phân
    case 'StartingOsakaPump':
      return renderBadge('default', 'Khởi động bơm trộn');
    case 'DosingPumpA':
      return renderBadge('default', 'Đang châm phân A');
    case 'WaitingBetweenDose':
      return renderBadge('warn', 'Chờ hòa tan A → B');
    case 'DosingPumpB':
      return renderBadge('default', 'Đang châm phân B');
    case 'DosingPH':
      return renderBadge('default', 'Đang chỉnh pH');

    // Trộn & ổn định
    case 'ActiveMixing':
      return renderBadge('info', 'Đang sục trộn');
    case 'Stabilizing':
      return renderBadge('warn', 'Chờ cảm biến ổn định');

    // Phun sương (misting chạy song song FSM — hiển thị theo pump_status)
    case 'Misting':
      return renderBadge('mist', 'Đang phun sương');

    // Hiệu chuẩn (không có suffix)
    case 'SensorCalibration':
      return renderBadge('info', 'Đang hiệu chuẩn cảm biến');
    case 'enter_calibration':
      return renderBadge('info', 'Vào chế độ hiệu chuẩn');
    case 'exit_calibration':
      return renderBadge('success', 'Thoát hiệu chuẩn');

    default:
      return renderBadge('default', rawState);
  }
};
