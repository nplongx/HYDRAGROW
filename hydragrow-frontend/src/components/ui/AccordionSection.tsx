import React from 'react';
import { ChevronDown } from 'lucide-react';

interface AccordionSectionProps {
  id: string;
  title: string;
  icon: React.ElementType;
  color: string;
  children: React.ReactNode;
  isOpen: boolean;
  onToggle: () => void;
}

export const AccordionSection: React.FC<AccordionSectionProps> = ({ title, icon: Icon, color, children, isOpen, onToggle }) => {
  return (
    <section className={`bg-slate-900 rounded-2xl overflow-hidden transition-all duration-300 border ${isOpen ? 'border-slate-700 shadow-lg shadow-black/20' : 'border-slate-800/80 hover:border-slate-700'}`}>
      <button onClick={onToggle} className={`w-full flex items-center justify-between p-4 transition-colors ${isOpen ? 'bg-slate-800/30' : 'hover:bg-slate-800/50 active:bg-slate-800'}`}>
        <div className={`flex items-center space-x-3 ${color}`}>
          <div className={`p-2 rounded-xl bg-slate-950/50 ${isOpen ? 'shadow-inner' : ''}`}>
            <Icon size={20} />
          </div>
          <h2 className="text-sm font-bold text-white tracking-wide">{title}</h2>
        </div>
        <div className={`p-1.5 rounded-full transition-transform duration-300 ${isOpen ? 'bg-slate-800 rotate-180' : 'bg-transparent'}`}>
          <ChevronDown size={18} className="text-slate-400" />
        </div>
      </button>
      <div className={`transition-all duration-300 ease-in-out origin-top ${isOpen ? 'max-h-[5000px] opacity-100 scale-y-100' : 'max-h-0 opacity-0 scale-y-95 pointer-events-none'}`}>
        <div className="p-4 space-y-4 border-t border-slate-800/60 bg-slate-900/50">
          {children}
        </div>
      </div>
    </section>
  );
};
