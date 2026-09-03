import React from 'react';
import { Bell } from 'lucide-react';

interface AlertBellProps {
  unreadCount: number;
  onClick: () => void;
}

export const AlertBell: React.FC<AlertBellProps> = ({ unreadCount, onClick }) => (
  <button
    type="button"
    onClick={onClick}
    aria-label="Xem cảnh báo"
    className="relative flex h-10 w-10 items-center justify-center rounded-full border border-emerald-100 bg-white text-emerald-700 shadow-sm shadow-emerald-950/5 transition-colors hover:bg-emerald-50"
  >
    <Bell size={17} strokeWidth={2.25} />
    {unreadCount > 0 && (
      <span className="absolute -top-1 -right-1 flex h-4 min-w-4 items-center justify-center rounded-full border border-white bg-red-600 px-1 text-[9px] font-bold text-white">
        {unreadCount > 9 ? '9+' : unreadCount}
      </span>
    )}
  </button>
);
