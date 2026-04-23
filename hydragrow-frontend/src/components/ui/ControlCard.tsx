import { Lock } from 'lucide-react';
import React from 'react';
import { Switch } from './Switch';
import { toast } from 'react-hot-toast'; // Hoặc react-hot-toast tùy thư viện bạn đang dùng

interface ControlCardProps {
  title: string;
  icon: React.ElementType;
  colorClass: string;
  borderClass: string;
  isOn: boolean;
  pumpId: string;
  lockedMessage?: string;
  supportsPwm?: boolean;
  currentPwm?: number;
  isOnline: boolean;
  isProcessing: boolean;
  onToggle: (
    id: string,
    action: 'on' | 'off',
    isLocked: boolean,
    pwm?: number,
    title?: string,
  ) => void;
  onPwmChange?: (id: string, val: number) => void;
  onPwmCommit?: (id: string, val: number, title: string) => void;
}

export const ControlCard: React.FC<ControlCardProps> = ({
  title,
  icon: Icon,
  colorClass,
  borderClass,
  isOn,
  lockedMessage,
  pumpId,
  supportsPwm = false,
  currentPwm = 100,
  isOnline,
  isProcessing,
  onToggle,
  onPwmChange,
  onPwmCommit,
}) => {
  const isLocked = !!lockedMessage && !isOn;

  const handleClick = (e: React.MouseEvent) => {
    if ((e.target as HTMLElement).tagName === 'INPUT') return;
    if (!isOnline) {
      toast.error('Thiết bị Offline!');
      return;
    }
    if (isProcessing) return;
    if (isLocked) {
      toast.error(lockedMessage);
      return;
    }

    const pwmToPass = supportsPwm ? currentPwm : undefined;
    onToggle(pumpId, isOn ? 'off' : 'on', isLocked, pwmToPass, title);
  };

  return (
    <div
      onClick={handleClick}
      className={`relative overflow-hidden bg-slate-900/80 backdrop-blur-md rounded-3xl p-4 flex flex-col transition-all duration-300 select-none
        ${isOn ? `border border-${borderClass} shadow-[0_0_20px_rgba(0,0,0,0.2)] shadow-${borderClass}/10 bg-slate-800/80` : 'border border-slate-800 hover:border-slate-700/80'} 
        ${isLocked ? 'cursor-not-allowed' : 'cursor-pointer active:scale-[0.98]'}
      `}
    >
      <div
        className={`flex items-center justify-between z-10 w-full transition-opacity ${isLocked ? 'opacity-50 grayscale' : ''}`}
      >
        <div className="flex items-center space-x-4 overflow-hidden">
          <div
            className={`p-3 rounded-2xl shrink-0 transition-all duration-500 ${isOn ? `bg-slate-950 shadow-inner ${colorClass}` : 'bg-slate-800/50 text-slate-500'}`}
          >
            <Icon size={24} className={isOn ? 'animate-pulse' : ''} />
          </div>
          <div className="flex flex-col min-w-0">
            <span
              className={`font-bold tracking-wide truncate ${isOn ? 'text-white' : 'text-slate-300'}`}
            >
              {title}
            </span>
            {supportsPwm && isOn ? (
              <span className={`text-xs font-semibold mt-0.5 ${colorClass}`}>
                PWM: {currentPwm}%
              </span>
            ) : (
              <span className="text-[11px] font-medium text-slate-500 mt-0.5">
                {isOn ? 'Đang chạy' : 'Đã tắt'}
              </span>
            )}
          </div>
        </div>
        <div className="z-10 shrink-0 ml-2">
          {isLocked ? (
            <div className="h-7 w-12 flex items-center justify-center bg-slate-800/50 rounded-full border border-slate-700/50">
              <Lock size={14} className="text-slate-500" />
            </div>
          ) : (
            <Switch isOn={isOn} disabled={isProcessing || !isOnline} />
          )}
        </div>
      </div>

      {isLocked && (
        <div className="mt-3 pt-3 border-t border-red-500/20 text-[11px] text-red-400 font-medium flex items-center bg-red-500/5 p-2 rounded-xl">
          <Lock size={12} className="mr-1.5 shrink-0" /> {lockedMessage}
        </div>
      )}

      {supportsPwm && !isLocked && onPwmChange && onPwmCommit && (
        <div
          className={`transition-all duration-300 ease-out overflow-hidden ${isOn ? 'max-h-20 opacity-100 mt-4' : 'max-h-0 opacity-0 mt-0'}`}
        >
          <div className="px-1">
            <input
              type="range"
              min="10"
              max="100"
              step="5"
              value={currentPwm}
              onChange={(e) => onPwmChange(pumpId, parseInt(e.target.value))}
              onMouseUp={() => onPwmCommit(pumpId, currentPwm, title)}
              onTouchEnd={() => onPwmCommit(pumpId, currentPwm, title)}
              className={`w-full h-2 rounded-lg appearance-none cursor-pointer transition-colors ${isOn ? 'bg-slate-700' : 'bg-slate-800'}`}
              style={{ accentColor: 'currentColor' }}
            />
            <div className="flex justify-between text-[10px] text-slate-500 font-semibold mt-1.5 px-1">
              <span>Nhẹ</span>
              <span>Vừa</span>
              <span>Mạnh</span>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
