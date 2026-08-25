import React from 'react';

interface SwitchProps {
  isOn?: boolean;
  checked?: boolean;
  disabled?: boolean;
  onClick?: (newState: boolean) => void;
  onChange?: (newState: boolean) => void;
  colorClass?: string; // Ví dụ: bg-blue-500
}

export const Switch: React.FC<SwitchProps> = ({
  isOn,
  checked,
  disabled = false,
  onClick,
  onChange,
  colorClass = 'bg-blue-600'
}) => {
  const active = checked ?? isOn ?? false;
  const toggleHandler = onChange ?? onClick;

  return (
    <div
      onClick={() => !disabled && toggleHandler && toggleHandler(!active)}
      role="switch"
      aria-checked={active}
      aria-disabled={disabled}
      tabIndex={disabled ? -1 : 0}
      onKeyDown={(event) => {
        if (disabled || !onClick) return;
        if (event.key === 'Enter' || event.key === ' ') {
          event.preventDefault();
          onClick(!isOn);
        }
      }}
      className={`relative inline-flex h-6 w-11 shrink-0 items-center rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none focus:ring-2 focus:ring-emerald-500/30
      ${active ? colorClass : 'bg-emerald-200'}
      ${disabled ? 'opacity-50 cursor-not-allowed' : 'cursor-pointer'}`}
    >
      <span
        className={`pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow-sm transition duration-200 ease-in-out
        ${active ? 'translate-x-5' : 'translate-x-0'}`}
      />
    </div>
  );
};
