import React from 'react';

interface InputProps extends React.InputHTMLAttributes<HTMLInputElement> {
  label?: string;
  hint?: string;
  error?: string;
}

export const Input: React.FC<InputProps> = ({ label, hint, error, id, className, ...rest }) => {
  const inputId = id ?? (label ? `${label}-input`.replace(/\s+/g, '-').toLowerCase() : undefined);
  return (
    <div className="flex flex-col gap-1.5">
      {label && (
        <label htmlFor={inputId} className="ui-form-label">
          {label}
        </label>
      )}
      <input
        id={inputId}
        aria-invalid={error ? true : undefined}
        className={`ui-input ${error ? 'border-error focus:ring-error/30 focus:border-error' : ''} ${className ?? ''}`}
        {...rest}
      />
      {error ? (
        <p className="text-[11px] text-error pl-1 leading-tight" role="alert">
          {error}
        </p>
      ) : hint ? (
        <p className="ui-helper-text">{hint}</p>
      ) : null}
    </div>
  );
};