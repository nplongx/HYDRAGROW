import React from "react";
import type { LucideIcon } from "lucide-react";
import { Sprout } from "lucide-react";
import { Link } from "react-router-dom";

export interface PrimaryNavItem {
  id: string;
  label: string;
  path: string;
  icon: LucideIcon;
  active: boolean;
  hasBadge?: boolean;
}

interface ConnectionStatusProps {
  availability?: string;
}

export function ConnectionStatus({ availability }: ConnectionStatusProps) {
  const online = availability === "ONLINE";
  const offline = availability === "OFFLINE";
  const label = online
    ? "Đang kết nối"
    : offline
      ? "Mất tín hiệu"
      : "Chưa rõ trạng thái";

  return (
    <div
      className={`inline-flex items-center gap-2 rounded-full border px-3 py-1.5 text-xs font-semibold ${
        online
          ? "border-line bg-pill text-status"
          : offline
            ? "border-line bg-danger-bg text-error"
            : "border-line bg-surface-muted text-text-muted"
      }`}
      aria-label={`Trạng thái kết nối: ${label}`}
    >
      <span
        className={`h-2 w-2 rounded-full ${online ? "bg-status" : offline ? "bg-error" : "bg-faint"}`}
      />
      {label}
    </div>
  );
}

interface StationStatusProps {
  availability?: string;
  deviceId?: string | null;
}

function StationStatus({ availability, deviceId }: StationStatusProps) {
  const online = availability === "ONLINE";
  const offline = availability === "OFFLINE";

  return (
    <div className="mt-auto rounded-2xl border border-line bg-pill/70 px-4 py-3.5 space-y-2">
      <div className="flex items-center gap-1.5">
        <span
          className={`h-2 w-2 rounded-full ${online ? "bg-status" : offline ? "bg-error" : "bg-faint"}`}
        />
        <span className="text-xs font-semibold text-primary-deep">
          {online
            ? "Trạm Online"
            : offline
              ? "Trạm Offline"
              : "Trạng thái chưa rõ"}
        </span>
      </div>
      <p className="truncate text-[11px] font-medium text-text-muted" title={deviceId ?? undefined}>ID: {deviceId ?? "—"}</p>
    </div>
  );
}

interface NavProps {
  items: PrimaryNavItem[];
  onNavigate: (path: string) => void;
}

function NavButton({
  item,
  mobile,
  onNavigate,
}: {
  item: PrimaryNavItem;
  mobile?: boolean;
  onNavigate: (path: string) => void;
}) {
  if (mobile) {
    return (
      <button
        type="button"
        onClick={() => onNavigate(item.path)}
        aria-current={item.active ? "page" : undefined}
        aria-label={item.label}
        className={`relative flex min-w-14 flex-col items-center justify-center gap-1 rounded-xl px-2 py-2 transition-colors ${item.active ? "bg-white shadow-sm" : "group"}`}
      >
        <span className={`relative flex h-7 w-7 items-center justify-center rounded-lg ${item.active ? "text-primary" : "text-faint group-hover:text-primary"}`}>
          <item.icon size={17} aria-hidden="true" />
          {item.hasBadge && <span className="absolute right-0 top-0 h-2 w-2 rounded-full bg-red-600" aria-label="Có cảnh báo mới" />}
        </span>
        <span
          className={`text-[10px] font-semibold tracking-tight transition-colors ${item.active ? "text-primary-deep" : "text-faint group-hover:text-primary/70"}`}
        >
          {item.label}
        </span>
      </button>
    );
  }

  const Icon = item.icon;
  return (
    <button
      type="button"
      onClick={() => onNavigate(item.path)}
      aria-current={item.active ? "page" : undefined}
      className={`relative flex w-full items-center gap-2.5 rounded-[10px] px-3.5 py-2.5 text-sm transition-colors ${
        item.active
          ? "bg-pill font-semibold text-primary-deep"
          : "font-normal text-text-muted hover:bg-surface-muted hover:text-primary-deep"
      }`}
    >
      <Icon
        size={16}
        className={item.active ? "text-primary" : "text-faint"}
        aria-hidden="true"
      />
      <span>{item.label}</span>
      {item.hasBadge && (
        <span
          className="ml-auto h-2 w-2 rounded-full bg-red-600"
          aria-label="Có cảnh báo mới"
        />
      )}
    </button>
  );
}

export function MobileHeader({ availability }: ConnectionStatusProps) {
  return (
    <header className="flex items-center justify-between border-b border-line bg-white/95 px-4 py-3.5 backdrop-blur z-30 pt-[calc(env(safe-area-inset-top)+12px)] lg:hidden">
      <Link
        to="/dashboard"
        className="flex items-center gap-2.5 rounded-lg focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/30"
        aria-label="HydraGrow - Tổng quan"
      >
        <div className="flex h-9 w-9 items-center justify-center rounded-xl bg-primary-deep shadow-sm">
          <Sprout
            size={16}
            className="text-white"
            strokeWidth={2.5}
            aria-hidden="true"
          />
        </div>
        <div>
          <div className="text-sm font-extrabold tracking-tight text-primary-deep leading-none">
            HydraGrow
          </div>
          <div className="text-[10px] text-faint font-semibold mt-0.5 tracking-wide">
            Khí canh thông minh
          </div>
        </div>
      </Link>
      <ConnectionStatus availability={availability} />
    </header>
  );
}

export function DesktopSidebar({
  items,
  availability,
  deviceId,
  onNavigate,
}: NavProps & StationStatusProps) {
  return (
    <aside className="hidden lg:flex fixed inset-y-0 left-0 z-20 w-64 flex-col border-r border-line bg-white px-5 pb-5 pt-6">
      <Link
        to="/dashboard"
        className="flex items-center gap-2.5 rounded-lg focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/30"
        aria-label="HydraGrow - Tổng quan"
      >
        <div className="flex h-10 w-10 items-center justify-center rounded-xl bg-primary-deep shadow-sm">
          <Sprout
            size={16}
            className="text-white"
            strokeWidth={2.5}
            aria-hidden="true"
          />
        </div>
        <div>
          <span className="block text-[18px] font-extrabold tracking-tight text-primary-deep">HydraGrow</span>
          <span className="block text-[10px] font-medium tracking-wide text-faint">SMART AEROPONICS</span>
        </div>
      </Link>

      <nav aria-label="Điều hướng chính" className="mt-8 flex flex-col gap-1.5">
        {items.map((item) => (
          <NavButton key={item.id} item={item} onNavigate={onNavigate} />
        ))}
      </nav>

      <StationStatus availability={availability} deviceId={deviceId} />
    </aside>
  );
}

export function MobileBottomNav({
  items,
  onNavigate,
}: Pick<NavProps, "items" | "onNavigate">) {
  return (
    <nav
      aria-label="Điều hướng chính mobile"
      className="fixed bottom-0 left-0 right-0 z-50 px-3 pb-[calc(env(safe-area-inset-bottom)+10px)] lg:hidden"
    >
      <div className="mx-auto flex max-w-md items-center justify-between rounded-2xl border border-line bg-white/95 px-2 py-2 shadow-medium backdrop-blur-md">
        {items.map((item) => (
          <NavButton key={item.id} item={item} mobile onNavigate={onNavigate} />
        ))}
      </div>
    </nav>
  );
}

interface AppShellProps {
  children: React.ReactNode;
  navItems: PrimaryNavItem[];
  availability?: string;
  deviceId?: string | null;
  onNavigate: (path: string) => void;
}

export function AppShell({
  children,
  navItems,
  availability,
  deviceId,
  onNavigate,
}: AppShellProps) {
  return (
    <div className="flex h-screen flex-col overflow-hidden bg-surface-muted font-sans text-primary-deep">
      <MobileHeader availability={availability} />
      <DesktopSidebar
        items={navItems}
        availability={availability}
        deviceId={deviceId}
        onNavigate={onNavigate}
      />
      <main className="relative z-10 flex-1 overflow-y-auto pb-24 scroll-smooth custom-scrollbar lg:ml-64 lg:pb-6">
        {children}
      </main>
      <MobileBottomNav items={navItems} onNavigate={onNavigate} />
    </div>
  );
}
