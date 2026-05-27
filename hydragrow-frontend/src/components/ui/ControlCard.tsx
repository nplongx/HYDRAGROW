import { Lock } from 'lucide-react';
import React from 'react';
import { Switch } from './Switch';
import { toast } from 'react-hot-toast';

interface ControlCardProps {
  title: string;
  icon: React.ElementType;
  colorClass: string; // Tailwind text color class (e.g., text-blue-500)
  borderClass: string; // Tailwind border color class (e.g., border-blue-500)
  isOn: boolean;
  pumpId: string;
  lockedMessage?: string;
  supportsPwm?: boolean;
  currentPwm?: number;
  isOnline: boolean;
  isProcessing: boolean;
  onToggle: (id: string, action: 'on' | 'off', isLocked: boolean, pwm?: number, title?: string) => void;
  onPwmChange?: (id: string, val: number) => void;
  onPwmCommit?: (id: string, val: number, title: string) => void;
}

export const ControlCard: React.FC<ControlCardProps> = ({
  title, icon: Icon, colorClass, borderClass, isOn, lockedMessage, pumpId,
  supportsPwm = false, currentPwm = 100, isOnline, isProcessing,
  onToggle, onPwmChange, onPwmCommit
}) => {
  const isLocked = !!lockedMessage && !isOn;

  const handleClick = (e: React.MouseEvent) => {
    if ((e.target as HTMLElement).tagName === 'INPUT') return;
    if (!isOnline) { toast.error("Thiết bị Offline!"); return; }
    if (isProcessing) return;
    if (isLocked) { toast.error(lockedMessage); return; }

    const pwmToPass = supportsPwm ? currentPwm : undefined;
    onToggle(pumpId, isOn ? 'off' : 'on', isLocked, pwmToPass, title);
  };

  return (
    <div
      onClick={handleClick}
      className={`relative bg-white rounded-xl p-4 flex flex-col transition-colors duration-200 select-none border shadow-sm shadow-emerald-950/5
        ${isOn ? `border-${borderClass.split('-')[1]}-500 bg-emerald-50` : 'border-emerald-100 hover:border-emerald-300'}
        ${isLocked ? 'cursor-not-allowed opacity-60' : 'cursor-pointer'}
      `}
    >
      <div className="flex items-center justify-between w-full">
        <div className="flex items-center gap-3 overflow-hidden">
          <div className={`p-2 rounded-lg transition-colors ${isOn ? `bg-${colorClass.split('-')[1]}-500/10 ${colorClass}` : 'bg-emerald-50 text-emerald-700/75'}`}>
            <Icon size={20} />
          </div>
          <div className="flex flex-col min-w-0">
            <span className={`text-sm font-semibold truncate ${isOn ? 'text-emerald-950' : 'text-emerald-900'}`}>
              {title}
            </span>
            {supportsPwm && isOn ? (
              <span className={`text-xs font-medium mt-0.5 ${colorClass}`}>Công suất: {currentPwm}%</span>
            ) : (
              <span className="text-xs font-medium text-emerald-700/75 mt-0.5">{isOn ? 'Đang chạy' : 'Đã tắt'}</span>
            )}
          </div>
        </div>

        <div className="shrink-0 ml-2">
          {isLocked ? (
            <div className="h-6 w-11 flex items-center justify-center bg-emerald-100 rounded-full border border-emerald-200">
              <Lock size={12} className="text-emerald-700/75" />
            </div>
          ) : (
            <Switch isOn={isOn} disabled={isProcessing || !isOnline} />
          )}
        </div>
      </div>

      {isLocked && (
        <div className="mt-3 text-xs text-red-700 font-medium flex items-center gap-1.5 bg-red-50 p-2 rounded-lg border border-red-200">
          <Lock size={12} className="shrink-0" /> {lockedMessage}
        </div>
      )}

      {supportsPwm && !isLocked && onPwmChange && onPwmCommit && (
        <div className={`transition-all duration-300 ease-in-out overflow-hidden ${isOn ? 'max-h-20 opacity-100 mt-4' : 'max-h-0 opacity-0 mt-0'}`}>
          <div className="px-1">
            <input
              type="range" min="10" max="100" step="5"
              value={currentPwm}
              onChange={(e) => onPwmChange(pumpId, parseInt(e.target.value))}
              onMouseUp={() => onPwmCommit(pumpId, currentPwm, title)}
              onTouchEnd={() => onPwmCommit(pumpId, currentPwm, title)}
              className="w-full h-1.5 bg-emerald-100 rounded-lg appearance-none cursor-pointer accent-emerald-600"
            />
            <div className="flex justify-between text-[10px] text-emerald-700/75 font-medium mt-2">
              <span>Yếu</span><span>Vừa</span><span>Mạnh</span>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
