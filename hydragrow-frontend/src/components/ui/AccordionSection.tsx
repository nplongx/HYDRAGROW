import React, { useEffect, useState } from "react";
import { ChevronDown, LucideIcon } from "lucide-react";

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
  hidden?: boolean;
}

export const AccordionSection: React.FC<AccordionSectionProps> = ({
  id,
  title,
  icon: Icon,
  color,
  children,
  defaultOpen = false,
  isOpen: controlledIsOpen,
  onToggle,
  badge,
  hidden = false,
}) => {
  const isControlled = controlledIsOpen !== undefined;
  const [internalOpen, setInternalOpen] = useState(
    defaultOpen || controlledIsOpen === true,
  );

  // Settings uses controlledIsOpen for active tab selection, not accordion state.
  // Sync the local accordion state when the active tab changes.
  useEffect(() => {
    if (isControlled) setInternalOpen(controlledIsOpen);
  }, [controlledIsOpen, isControlled]);

  const open = internalOpen;
  const contentId = `${id ?? title.toLowerCase().replace(/[^a-z0-9]+/g, "-")}-content`;

  const handleToggle = () => {
    setInternalOpen((current) => !current);
    // Controlled Settings accordions must not change the active tab.
    if (!isControlled) onToggle?.();
  };

  return (
    <div className={`ui-card overflow-hidden p-0 ${hidden ? "hidden" : ""}`}>
      <button
        type="button"
        onClick={handleToggle}
        aria-expanded={open}
        aria-controls={contentId}
        className="flex min-h-12 w-full items-center justify-between px-4 py-3.5 text-left transition-colors hover:bg-surface-interactive-hover focus:outline-none focus-visible:ring-2 focus-visible:ring-primary/25 focus-visible:ring-inset"
      >
        <div className="flex items-center gap-2">
          {Icon && (
            <div
              className={`p-1.5 rounded-lg bg-pill border border-line ${color || "text-primary"}`}
            >
              <Icon size={16} strokeWidth={2} />
            </div>
          )}
          <span className="ui-section-title">
            {title}
          </span>
          {badge && (
            <span className="px-2 py-0.5 rounded-full bg-pill text-status text-[10px] font-bold">
              {badge}
            </span>
          )}
        </div>
        <ChevronDown
          size={16}
          strokeWidth={2.5}
          className={`text-primary transition-transform duration-200 ${open ? "rotate-180" : ""}`}
        />
      </button>
      {open && (
        <div id={contentId} className="border-t border-line px-4 pb-4 pt-3">
          {children}
        </div>
      )}
    </div>
  );
};
