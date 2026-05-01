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
    ? 'text-red-400 bg-red-500/5 border-red-500/20'
    : 'text-slate-400 bg-slate-900/50 border-slate-800';

  return (
    <div className={`flex flex-col items-center justify-center p-8 rounded-xl border text-center ${tone} ${className}`}>
      <div className={`p-3 rounded-full mb-4 ${variant === 'error' ? 'bg-red-500/10' : 'bg-slate-800'}`}>
        <Icon size={24} />
      </div>
      <h3 className={`text-base font-semibold mb-1 ${variant === 'error' ? 'text-red-400' : 'text-slate-200'}`}>
        {title}
      </h3>
      {description && <p className="text-sm text-slate-500 max-w-sm">{description}</p>}
    </div>
  );
};
