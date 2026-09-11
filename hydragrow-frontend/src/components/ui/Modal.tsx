import React, { useEffect } from 'react';
import { X } from 'lucide-react';

interface ModalProps {
  open: boolean;
  onClose: () => void;
  title?: string;
  footer?: React.ReactNode;
  children?: React.ReactNode;
  className?: string;
}

export const Modal: React.FC<ModalProps> = ({ open, onClose, title, footer, children, className }) => {
  useEffect(() => {
    if (!open) return;
    const handleKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    document.addEventListener('keydown', handleKey);
    document.body.style.overflow = 'hidden';
    return () => {
      document.removeEventListener('keydown', handleKey);
      document.body.style.overflow = '';
    };
  }, [open, onClose]);

  if (!open) return null;

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-primary-deep/40 p-4"
      role="presentation"
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div
        role="dialog"
        aria-modal="true"
        aria-label={title}
        className={`w-full max-w-md rounded-2xl border border-line bg-white shadow-xl shadow-primary/10 ${className ?? ''}`}
      >
        <div className="flex items-center justify-between gap-3 border-b border-line px-5 py-4">
          {title && <h2 className="text-base font-bold text-primary-deep">{title}</h2>}
          <button
            type="button"
            onClick={onClose}
            aria-label="Đóng"
            className="ml-auto inline-flex h-8 w-8 items-center justify-center rounded-lg text-faint transition-colors hover:bg-surface-muted hover:text-primary-deep"
          >
            <X size={16} aria-hidden="true" />
          </button>
        </div>
        <div className="px-5 py-4">{children}</div>
        {footer && <div className="flex flex-col-reverse gap-2 border-t border-line px-5 py-4 sm:flex-row sm:justify-end">{footer}</div>}
      </div>
    </div>
  );
};