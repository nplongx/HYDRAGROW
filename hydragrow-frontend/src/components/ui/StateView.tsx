import React from 'react';
import { LucideIcon } from 'lucide-react';

interface StateViewProps {
  icon: LucideIcon | React.ElementType;
  title: string;
  description?: string;
  action?: React.ReactNode;
  className?: string;
}

export const StateView: React.FC<StateViewProps> = ({ icon: Icon, title, description, action, className = '' }) => (
  <div className={`ui-state ${className}`}>
    <div className="ui-state-icon">
      <Icon size={32} className="text-emerald-600" strokeWidth={1.5} />
    </div>
    <div className="space-y-1">
      <h3 className="ui-state-title">{title}</h3>
      {description && <p className="ui-state-desc">{description}</p>}
    </div>
    {action && <div>{action}</div>}
  </div>
);
