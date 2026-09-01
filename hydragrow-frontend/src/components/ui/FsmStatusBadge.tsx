import React, { useMemo, useState } from 'react';
import { FaultExplanation, getFaultGuide } from './FaultExplanation';

// File: components/ui/FsmStatusBadge.tsx

export const extractFaultCode = (state?: string): string | null => {
  if (!state) return null;
  
  if (state.startsWith('SystemFault:')) return state.replace('SystemFault:', '').trim();
  if (state.startsWith('Fault:')) return state.replace('Fault:', '').trim();
  
  // 🟢 Bổ sung: Bắt trường hợp chuỗi JSON thô {"Fault":"PhDosingFailed"}
  if (state.startsWith('{')) {
    try {
      const parsed = JSON.parse(state);
      if (parsed.Fault) return String(parsed.Fault).trim();
    } catch {
      // Ignore JSON parse error if state is not JSON
    }
  }
  return null;
};

export const FsmStatusBadge: React.FC<{ state?: string }> = ({ state }) => {
  const [showFaultSheet, setShowFaultSheet] = useState(false);
  const rawState = state || 'Monitoring';
  const faultCode = extractFaultCode(rawState);
  const faultGuide = useMemo(() => getFaultGuide(faultCode || undefined), [faultCode]);

  const renderBadge = (tone: 'default' | 'warn' | 'danger' | 'success' | 'info' | 'mist', content: string) => {
    const toneClass =
      tone === 'danger' ? 'bg-red-50 border-red-200 text-red-700'
        : tone === 'warn' ? 'bg-amber-50 border-amber-200 text-amber-800'
          : tone === 'success' ? 'bg-emerald-50 border-emerald-200 text-emerald-700'
            : tone === 'info' ? 'bg-sky-50 border-sky-200 text-sky-700'
              : tone === 'mist' ? 'bg-cyan-50 border-cyan-200 text-cyan-700'
                : 'bg-emerald-50 border-emerald-200 text-emerald-800';

    const baseClass = `px-2.5 py-0.5 rounded-md text-xs font-medium border ${toneClass}`;
    if (faultCode) {
      return <button className={`${baseClass} hover:opacity-90`} onClick={() => setShowFaultSheet(true)}>{content}</button>;
    }
    return <span className={baseClass}>{content}</span>;
  };

  if (faultCode) return <>{renderBadge('danger', `Lỗi: ${faultCode}`)}{showFaultSheet && faultGuide && <FaultExplanation code={faultCode} onClose={() => setShowFaultSheet(false)} />}</>;
  if (rawState.startsWith('EmergencyStop:')) return renderBadge('danger', `Ngắt khẩn cấp: ${rawState.replace('EmergencyStop:', '')}`);
  if (rawState.startsWith('Cooldown:')) return renderBadge('warn', 'Pha khóa bảo vệ (Cooldown)');
  if (rawState.startsWith('SensorCalibration:')) return renderBadge('info', `Calib: ${rawState.replace('SensorCalibration:', '')}`);

  switch (rawState) {
    case 'SystemBooting':
    case 'Booting': return renderBadge('info', 'Đang khởi động...');
    case 'ManualMode': return renderBadge('warn', 'Chế độ thủ công');
    case 'Monitoring': return renderBadge('default', 'Đang giám sát');
    case 'DosingCycleComplete': return renderBadge('success', 'Hoàn tất chu trình');
    case 'EmergencyStop': return renderBadge('danger', 'Dừng khẩn cấp');
    case 'Disconnected':
    case 'Offline': return renderBadge('danger', 'Mất kết nối');
    case 'WaterRefilling': return renderBadge('info', 'Đang cấp nước');
    case 'WaterDraining': return renderBadge('info', 'Đang xả nước');
    case 'StartingOsakaPump': return renderBadge('default', 'Khởi động máy trộn');

    case 'MimoDosing': return renderBadge('mist', 'Đang châm MIMO (EC/pH)');

    case 'ActiveMixing': return renderBadge('info', 'Đang sục trộn khuấy động');
    case 'Stabilizing': return renderBadge('warn', 'Chờ ổn định cảm biến');
    case 'Misting': return renderBadge('mist', 'Đang phun sương');
    case 'enter_calibration': return renderBadge('info', 'Vào chế độ Calib');
    case 'exit_calibration': return renderBadge('success', 'Thoát Calib');
    default: return renderBadge('default', rawState);
  }
};
