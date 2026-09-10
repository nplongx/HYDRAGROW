import React from 'react';

interface SwitchProps {
  checked?: boolean;
  isOn?: boolean;
  onChange?: (checked: boolean) => void;
  onClick?: (checked: boolean) => void;
  disabled?: boolean;
  label?: string;
  size?: 'sm' | 'md';
  colorClass?: string;
}

export const Switch: React.FC<SwitchProps> = ({
  checked: checkedProp,
  isOn: isOnProp,
  onChange,
  onClick,
  disabled = false,
  label,
  size = 'md',
  colorClass,
}) => {
  const isChecked = checkedProp !== undefined ? checkedProp : (isOnProp ?? false);

  const handleToggle = (nextVal: boolean) => {
    if (disabled) return;
    if (onChange) onChange(nextVal);
    if (onClick) onClick(nextVal);
  };

  const trackW = size === 'sm' ? 'w-[30px]' : 'w-[38px]';
  const trackH = size === 'sm' ? 'h-[18px]' : 'h-[22px]';
  const thumbS = size === 'sm' ? 'w-3.5 h-3.5' : 'w-[18px] h-[18px]';
  const translate = size === 'sm' ? 'translate-x-3.5' : 'translate-x-4';

  const activeColor = colorClass || 'bg-primary';

  return (
    <label className={`inline-flex items-center gap-2.5 ${disabled ? 'opacity-50 cursor-not-allowed' : 'cursor-pointer'}`}>
      <button
        type="button"
        role="switch"
        aria-checked={isChecked}
        disabled={disabled}
        onClick={() => handleToggle(!isChecked)}
        className={`relative inline-flex items-center ${trackW} ${trackH} rounded-full transition-colors duration-200 focus:outline-none focus:ring-2 focus:ring-emerald-400/50 focus:ring-offset-1 ${isChecked ? activeColor : 'bg-toggleoff'}`}
      >
        <span className={`absolute left-0.5 inline-block ${thumbS} bg-white rounded-full shadow-sm transition-transform duration-200 ${isChecked ? translate : 'translate-x-0'}`} />
      </button>
      {label && <span className="text-sm font-medium text-primary-deep">{label}</span>}
    </label>
  );
};
