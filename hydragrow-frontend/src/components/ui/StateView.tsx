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
    ? 'text-rose-400 border-rose-500/30 bg-rose-500/5'
    : 'text-slate-500 border-slate-700/70';

  return (
    <div className={`ui-state ${tone} ${className}`}>
      <Icon size={34} className="mx-auto" />
      <h3 className="ui-state-title">{title}</h3>
      {description && <p className="ui-state-desc">{description}</p>}
    </div>
  );
};
