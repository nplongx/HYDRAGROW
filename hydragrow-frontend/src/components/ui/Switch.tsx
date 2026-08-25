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
  colorClass = 'bg-emerald-600',
}) => {
  const isChecked =
    checkedProp !== undefined ? checkedProp : (isOnProp ?? false);

  const handleToggle = () => {
    if (disabled) return;

    const nextValue = !isChecked;

    onChange?.(nextValue);
    onClick?.(nextValue);
  };

  const trackWidth = size === 'sm' ? 'w-9' : 'w-11';
  const trackHeight = size === 'sm' ? 'h-5' : 'h-6';
  const thumbSize = size === 'sm' ? 'h-3.5 w-3.5' : 'h-5 w-5';
  const thumbTranslate =
    size === 'sm' ? 'translate-x-4' : 'translate-x-5';

  return (
    <label
      className={`inline-flex items-center gap-2.5 ${
        disabled ? 'cursor-not-allowed opacity-50' : 'cursor-pointer'
      }`}
    >
      <button
        type="button"
        role="switch"
        aria-checked={isChecked}
        disabled={disabled}
        onClick={handleToggle}
        className={`relative inline-flex shrink-0 items-center rounded-full transition-colors duration-200 ease-in-out focus:outline-none focus:ring-2 focus:ring-emerald-400/50 focus:ring-offset-1 ${trackWidth} ${trackHeight} ${
          isChecked ? colorClass : 'bg-emerald-200'
        }`}
      >
        <span
          className={`pointer-events-none absolute left-0.5 inline-block rounded-full bg-white shadow-sm transition-transform duration-200 ${thumbSize} ${
            isChecked ? thumbTranslate : 'translate-x-0'
          }`}
        />
      </button>

      {label && (
        <span className="text-sm font-medium text-emerald-900">
          {label}
        </span>
      )}
    </label>
  );
};