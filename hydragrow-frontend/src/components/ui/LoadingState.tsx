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
    ? "fixed inset-0 z-50 flex items-center justify-center bg-slate-950/80 backdrop-blur-sm"
    : "flex flex-col items-center justify-center w-full h-full p-8";

  return (
    <div className={`${containerClass} ${className}`}>
      <div className="flex flex-col items-center gap-3 p-6 bg-slate-900 rounded-xl border border-slate-800 shadow-xl">
        <Loader2 size={24} className="animate-spin text-blue-500" />
        <p className="text-sm font-medium text-slate-300">{message}</p>
      </div>
    </div>
  );
};
