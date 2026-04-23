import React from 'react';

interface SwitchProps {
  isOn: boolean;
  disabled?: boolean;
  onClick?: (newState: boolean) => void;
  colorClass?: string;
}

export const Switch: React.FC<SwitchProps> = ({
  isOn,
  disabled = false,
  onClick,
  colorClass = 'bg-emerald-500',
}) => {
  return (
    <div
      onClick={() => !disabled && onClick && onClick(!isOn)}
      className={`relative inline-flex h-7 w-12 shrink-0 cursor-pointer items-center rounded-full border-2 border-transparent transition-all duration-300 ease-in-out shadow-inner 
      ${isOn ? colorClass : 'bg-slate-700'} 
      ${disabled ? 'opacity-40 cursor-not-allowed' : ''}`}
    >
      <span
        className={`pointer-events-none inline-block h-6 w-6 transform rounded-full bg-white shadow-md ring-0 transition duration-300 ease-out 
        ${isOn ? 'translate-x-5' : 'translate-x-0'}`}
      />
    </div>
  );
};
