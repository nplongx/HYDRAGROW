import React from 'react';

interface SwitchProps {
  isOn: boolean;
  disabled?: boolean;
  onClick?: (newState: boolean) => void;
  colorClass?: string; // Ví dụ: bg-blue-500
}

export const Switch: React.FC<SwitchProps> = ({
  isOn,
  disabled = false,
  onClick,
  colorClass = 'bg-blue-600'
}) => {
  return (
    <div
      onClick={() => !disabled && onClick && onClick(!isOn)}
      className={`relative inline-flex h-6 w-11 shrink-0 cursor-pointer items-center rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out
      ${isOn ? colorClass : 'bg-slate-700'} 
      ${disabled ? 'opacity-50 cursor-not-allowed' : ''}`}
    >
      <span
        className={`pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white transition duration-200 ease-in-out
        ${isOn ? 'translate-x-5' : 'translate-x-0'}`}
      />
    </div>
  );
};
