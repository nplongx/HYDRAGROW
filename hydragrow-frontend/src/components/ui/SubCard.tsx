import React from 'react';

interface SubCardProps {
  title?: string;
  children: React.ReactNode;
  className?: string;
}

export const SubCard: React.FC<SubCardProps> = ({ title, children, className = "" }) => (
  <div className={`rounded-xl border border-emerald-100 bg-white p-5 shadow-sm shadow-emerald-950/5 ${className}`}>
    {title && (
      <h3 className="text-sm font-semibold text-emerald-950 mb-4 flex items-center gap-2">
        <span className="w-1.5 h-4 rounded-sm bg-emerald-600"></span>
        {title}
      </h3>
    )}
    {children}
  </div>
);
