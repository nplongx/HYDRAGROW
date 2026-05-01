import React from 'react';
import { ChevronDown } from 'lucide-react';

interface AccordionSectionProps {
  id: string;
  title: string;
  icon: React.ElementType;
  color?: string; // Giữ lại prop color để tương thích với Settings.tsx
  children: React.ReactNode;
  isOpen: boolean;
  onToggle: () => void;
}

export const AccordionSection: React.FC<AccordionSectionProps> = ({
  title,
  icon: Icon,
  color = 'text-blue-500', // Mặc định nếu không truyền
  children,
  isOpen,
  onToggle
}) => {
  return (
    <section className={`bg-slate-900 rounded-xl overflow-hidden transition-colors border ${isOpen ? 'border-slate-700' : 'border-slate-800 hover:border-slate-700'}`}>

      {/* Header Button */}
      <button
        onClick={onToggle}
        className={`w-full flex items-center justify-between p-4 transition-colors ${isOpen ? 'bg-slate-800/40' : 'hover:bg-slate-800/20'}`}
      >
        <div className="flex items-center gap-3">
          <div className={`p-2 rounded-lg bg-slate-950 border border-slate-800/50 ${color}`}>
            <Icon size={18} strokeWidth={2} />
          </div>
          <h2 className="text-sm font-medium text-slate-100">{title}</h2>
        </div>
        <div className={`transition-transform duration-300 ${isOpen ? 'rotate-180' : ''}`}>
          <ChevronDown size={18} className="text-slate-500" />
        </div>
      </button>

      {/* Content Area - Dùng CSS Grid để animate height mượt mà thay vì max-h-[5000px] */}
      <div
        className={`grid transition-all duration-300 ease-in-out ${isOpen ? 'grid-rows-[1fr] opacity-100' : 'grid-rows-[0fr] opacity-0'
          }`}
      >
        <div className="overflow-hidden">
          <div className="p-4 sm:p-5 border-t border-slate-800">
            {children}
          </div>
        </div>
      </div>

    </section>
  );
};
