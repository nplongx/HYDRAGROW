import React from 'react';

interface PageHeaderProps {
  title: string;
  subtitle?: string;
  icon?: React.ElementType;
  action?: React.ReactNode;
  className?: string;
}

export const PageHeader: React.FC<PageHeaderProps> = ({
  title,
  subtitle,
  icon: Icon,
  action,
  className = ''
}) => {
  return (
    <header className={`flex items-start justify-between mb-6 ${className}`}>
      <div className="flex flex-col gap-1">
        <div className="flex items-center gap-2.5 text-slate-100">
          {Icon && <Icon size={24} className="text-slate-400" />}
          <h1 className="text-2xl font-semibold tracking-tight leading-none">{title}</h1>
        </div>
        {subtitle && (
          <p className="text-sm text-slate-500 font-medium ml-[34px]">{subtitle}</p>
        )}
      </div>
      {action && <div className="shrink-0">{action}</div>}
    </header>
  );
};
