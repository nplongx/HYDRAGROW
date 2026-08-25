import React, { useState } from 'react';
import { ChevronDown, LucideIcon } from 'lucide-react';

interface AccordionSectionProps {
  id?: string;
  title: string;
  icon?: LucideIcon | React.ElementType;
  color?: string;
  children: React.ReactNode;
  defaultOpen?: boolean;
  isOpen?: boolean;
  onToggle?: () => void;
  badge?: string;
}

export const AccordionSection: React.FC<AccordionSectionProps> = ({
  title,
  icon: Icon,
  color,
  children,
  defaultOpen = false,
  isOpen: controlledIsOpen,
  onToggle,
  badge,
}) => {
  const [internalOpen, setInternalOpen] = useState(defaultOpen);
  const isControlled = controlledIsOpen !== undefined;
  const open = isControlled ? controlledIsOpen : internalOpen;

  const handleToggle = () => {
    if (onToggle) {
      onToggle();
    }
    if (!isControlled) {
      setInternalOpen(!internalOpen);
    }
  };

  return (
    <div className="ui-card overflow-hidden p-0">
      <button
        type="button"
        onClick={handleToggle}
        className="w-full flex items-center justify-between px-4 py-3.5 text-left hover:bg-emerald-50/50 transition-colors"
      >
        <div className="flex items-center gap-2">
          {Icon && (
            <div className={`p-1.5 rounded-lg bg-emerald-50 border border-emerald-100 ${color || 'text-emerald-600'}`}>
              <Icon size={16} strokeWidth={2} />
            </div>
          )}
          <span className="text-sm font-semibold text-emerald-900">{title}</span>
          {badge && (
            <span className="px-2 py-0.5 rounded-full bg-emerald-100 text-emerald-700 text-[10px] font-bold">
              {badge}
            </span>
          )}
        </div>
        <ChevronDown
          size={16}
          strokeWidth={2.5}
          className={`text-emerald-500 transition-transform duration-200 ${open ? 'rotate-180' : ''}`}
        />
      </button>
      {open && (
        <div className="border-t border-emerald-100 px-4 pb-4 pt-3">
          {children}
        </div>
      )}
    </div>
  );
};
