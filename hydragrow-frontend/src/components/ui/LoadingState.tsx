import React from 'react';
import { Loader2 } from 'lucide-react';

interface LoadingStateProps {
  message?: string;
  fullscreen?: boolean;
  className?: string;
}

export const LoadingState: React.FC<LoadingStateProps> = ({
  message = 'Đang tải dữ liệu... Vui lòng chờ trong giây lát.',
  fullscreen = true,
  className = '',
}) => {
  return (
    <div className={`ui-loading ${fullscreen ? 'ui-loading-fullscreen' : ''} ${className}`.trim()}>
      <div className="ui-loading-card">
        <span className="ui-loading-spinner" aria-hidden="true">
          <Loader2 size={20} className="animate-spin" />
        </span>
        <p className="ui-loading-message">{message}</p>
      </div>
    </div>
  );
};
