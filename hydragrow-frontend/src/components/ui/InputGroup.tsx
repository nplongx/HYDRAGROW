import React, { useId } from "react";

interface InputGroupProps {
  label: string;
  unit?: string;
  helperText?: string;
  desc?: string; // @deprecated Use helperText instead
  error?: string;
  errorText?: string; // @deprecated Use error instead
  children?: React.ReactNode;
  // Input props for direct use when children is omitted
  type?: string;
  value?: string | number;
  onChange?: (e: React.ChangeEvent<HTMLInputElement>) => void;
  onChangeValue?: (value: string | number) => void; // Preferred for direct value access
  step?: string;
  min?: number;
  max?: number;
  disabled?: boolean;
  placeholder?: string;
  id?: string;
}

export const InputGroup: React.FC<InputGroupProps> = ({
  label,
  unit,
  helperText,
  desc,
  error,
  errorText,
  children,
  type = "number",
  value,
  onChange,
  onChangeValue,
  step,
  min,
  max,
  disabled,
  placeholder,
  id,
}) => {
  const displayHelper = helperText || desc;
  const displayError = error || errorText;
  const generatedId = useId();
  const inputId = id ?? generatedId;
  const helperId = displayHelper ? `${inputId}-help` : undefined;
  const errorId = displayError ? `${inputId}-error` : undefined;

  const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    if (onChange) {
      onChange(e);
    }
    if (onChangeValue) {
      const newValue =
        type === "number" ? parseFloat(e.target.value) : e.target.value;
      onChangeValue(newValue);
    }
  };

  return (
    <div className="ui-form-row">
      <label
        htmlFor={inputId}
        className="ui-form-label"
      >
        {label}
        {unit && (
          <span className="ml-1 font-normal text-text-muted">({unit})</span>
        )}
      </label>
      {children ? (
        children
      ) : (
        <input
          id={inputId}
          type={type}
          step={step}
          min={min}
          max={max}
          value={value ?? ""}
          onChange={handleChange}
          disabled={disabled}
          placeholder={placeholder}
          aria-invalid={displayError ? "true" : undefined}
          aria-describedby={errorId ?? helperId}
          className={`ui-input ${
            displayError
              ? "border-error/50 focus:border-error focus:ring-2 focus:ring-error/20"
              : ""
          }`}
        />
      )}
      {displayHelper && !displayError && (
        <p
          id={helperId}
          className="ui-helper-text"
        >
          {displayHelper}
        </p>
      )}
      {displayError && (
        <p id={errorId} className="mt-1 text-[11px] font-medium text-error">
          {displayError}
        </p>
      )}
    </div>
  );
};
