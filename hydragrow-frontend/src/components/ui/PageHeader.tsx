import React from 'react';
import { LucideIcon } from 'lucide-react';

interface PageHeaderProps {
  title: string;
  subtitle?: string;
  icon?: LucideIcon | React.ElementType;
  action?: React.ReactNode;
  className?: string;
}

export const PageHeader: React.FC<PageHeaderProps> = ({ title, subtitle, icon: Icon, action, className = '' }) => (
  <div className={`page-header ${className}`}>
    <div className="page-header-main">
      {Icon && (
        <div className="page-header-icon">
          <Icon size={20} strokeWidth={2.5} />
        </div>
      )}
      <div>
        <h1 className="page-header-title">{title}</h1>
        {subtitle && <p className="page-header-subtitle">{subtitle}</p>}
      </div>
    </div>
    {action && <div className="shrink-0">{action}</div>}
  </div>
);
