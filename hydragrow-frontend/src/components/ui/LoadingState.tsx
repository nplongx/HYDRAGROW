import React from 'react';
import { Loader2 } from 'lucide-react';

interface LoadingStateProps {
  message?: string;
  fullscreen?: boolean;
  className?: string;
}

export const LoadingState: React.FC<LoadingStateProps> = ({
  message = 'Đang tải dữ liệu...',
  fullscreen = true,
  className = '',
}) => {
  const containerClass = fullscreen
    ? "fixed inset-0 z-50 flex items-center justify-center bg-emerald-50/85 backdrop-blur-sm"
    : "flex flex-col items-center justify-center w-full h-full p-8";

  return (
    <div className={`${containerClass} ${className}`}>
      <div className="ui-loading-card flex-col p-6">
        <div className="ui-loading-spinner">
          <Loader2 size={24} className="animate-spin" />
        </div>
        <p className="ui-loading-message">{message}</p>
      </div>
    </div>
  );
};
