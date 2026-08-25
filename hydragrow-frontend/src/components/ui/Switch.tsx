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

  const trackW = size === 'sm' ? 'w-9' : 'w-11';
  const trackH = size === 'sm' ? 'h-5' : 'h-6';
  const thumbS = size === 'sm' ? 'w-3.5 h-3.5' : 'w-4.5 h-4.5';
  const translate = size === 'sm' ? 'translate-x-4' : 'translate-x-5';

  const activeColor = colorClass || 'bg-emerald-600';

  return (
    <label className={`inline-flex items-center gap-2.5 ${disabled ? 'opacity-50 cursor-not-allowed' : 'cursor-pointer'}`}>
      <button
        type="button"
        role="switch"
        aria-checked={isChecked}
        disabled={disabled}
        onClick={() => handleToggle(!isChecked)}
        className={`relative inline-flex items-center ${trackW} ${trackH} rounded-full transition-colors duration-200 focus:outline-none focus:ring-2 focus:ring-emerald-400/50 focus:ring-offset-1 ${isChecked ? activeColor : 'bg-emerald-200'}`}
      >
        <span className={`absolute left-0.5 inline-block ${thumbS} bg-white rounded-full shadow-sm transition-transform duration-200 ${isChecked ? translate : 'translate-x-0.5'}`} />
      </button>
      {label && <span className="text-sm font-medium text-emerald-900">{label}</span>}
    </label>
  );
};
