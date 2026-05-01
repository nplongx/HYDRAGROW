import React from 'react';

interface SubCardProps {
  title?: string;
  children: React.ReactNode;
  className?: string;
}

export const SubCard: React.FC<SubCardProps> = ({ title, children, className = "" }) => (
  <div className={`rounded-xl border border-slate-800 bg-slate-900 p-5 ${className}`}>
    {title && (
      <h3 className="text-sm font-semibold text-slate-300 mb-4 flex items-center gap-2">
        <span className="w-1.5 h-4 rounded-sm bg-blue-500"></span>
        {title}
      </h3>
    )}
    {children}
  </div>
);
