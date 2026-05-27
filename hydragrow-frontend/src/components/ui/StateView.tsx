import React from 'react';

interface StateViewProps {
  icon: React.ElementType;
  title: string;
  description?: string;
  variant?: 'empty' | 'error';
  className?: string;
}

export const StateView: React.FC<StateViewProps> = ({
  icon: Icon,
  title,
  description,
  variant = 'empty',
  className = ''
}) => {
  const tone = variant === 'error'
    ? 'text-red-700 bg-red-50 border-red-200'
    : 'text-emerald-700 bg-white border-emerald-200';

  return (
    <div className={`flex flex-col items-center justify-center p-8 rounded-xl border text-center shadow-sm shadow-emerald-950/5 ${tone} ${className}`}>
      <div className={`p-3 rounded-full mb-4 ${variant === 'error' ? 'bg-red-100' : 'bg-emerald-100'}`}>
        <Icon size={24} />
      </div>
      <h3 className={`text-base font-semibold mb-1 ${variant === 'error' ? 'text-red-800' : 'text-emerald-950'}`}>
        {title}
      </h3>
      {description && <p className="text-sm text-emerald-800/75 max-w-sm">{description}</p>}
    </div>
  );
};
