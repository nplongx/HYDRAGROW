import React from 'react';

interface SubCardProps {
  title?: string;
  children: React.ReactNode;
  className?: string;
}

export const SubCard: React.FC<SubCardProps> = ({ title, children, className = "" }) => (
  <div className={`bg-slate-950/40 border border-slate-800/60 rounded-2xl p-4 transition-all hover:border-slate-700/60 ${className}`}>
    {title && (
      <h3 className="text-xs font-bold text-slate-400 mb-4 uppercase tracking-wider flex items-center gap-2">
        <span className="w-1.5 h-1.5 rounded-full bg-slate-600"></span>
        {title}
      </h3>
    )}
    {children}
  </div>
);
