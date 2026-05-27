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
    <header className={`page-header mb-6 ${className}`}>
      <div className="page-header-main">
        {Icon && (
          <div className="page-header-icon">
            <Icon size={22} />
          </div>
        )}
        <div className="flex flex-col gap-1">
          <h1 className="page-header-title">{title}</h1>
          {subtitle && (
            <p className="page-header-subtitle">{subtitle}</p>
          )}
        </div>
      </div>
      {action && <div className="shrink-0">{action}</div>}
    </header>
  );
};
