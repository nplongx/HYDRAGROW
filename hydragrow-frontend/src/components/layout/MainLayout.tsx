import React, { useMemo } from 'react';
import { Outlet, useLocation, useNavigate } from 'react-router-dom';
import {
  LayoutDashboard, SlidersHorizontal, Settings, Sprout,
  AlignLeft, Leaf
} from 'lucide-react';
import { useDeviceStore } from '../../store/useDeviceStore';
import { useDeviceSync } from '../../hooks/useDeviceSync';
import { SystemEvent } from '../../types/models';

const MainLayout: React.FC = () => {
  useDeviceSync();
  const location = useLocation();
  const navigate = useNavigate();

  const deviceId = useDeviceStore((state) => state.deviceId);
  const isSensorOnline = useDeviceStore((state) => state.isSensorOnline);
  const isMissingConfig = useDeviceStore((state) => state.isMissingConfig);
  const systemEvents = useDeviceStore((state) => state.systemEvents);

  const unreadAlertCount = useMemo(() => {
    if (!systemEvents || !Array.isArray(systemEvents)) return 0;
    return systemEvents.filter((ev: SystemEvent) => {
      const rawTs = ev?.timestamp_ms ?? (ev as any)?.timestamp ?? 0;
      const ts = typeof rawTs === 'number' ? (rawTs > 1e12 ? rawTs : rawTs * 1000) : new Date(rawTs).getTime();
      if (!ts || Number.isNaN(ts)) return false;
      const within24h = Date.now() - ts <= 24 * 60 * 60 * 1000;
      const level = String(ev?.level || '').toLowerCase();
      return within24h && (level === 'warning' || level === 'critical' || level === 'error');
    }).length;
  }, [systemEvents]);


  const navItems = [
    { path: '/dashboard', icon: LayoutDashboard, label: 'Tổng quan' },
    { path: '/operations', icon: SlidersHorizontal, label: 'Vận hành' },
    { path: '/cultivation', icon: Leaf, label: 'Canh tác' },
    { path: '/journal', icon: AlignLeft, label: 'Nhật ký', hasBadge: unreadAlertCount > 0 },
    { path: '/settings', icon: Settings, label: 'Cài đặt' },
  ];

  const isActive = (path: string) => location.pathname === path || (path === '/dashboard' && location.pathname === '/');

  if (isMissingConfig && location.pathname !== '/settings') {
    return (
      <div className="min-h-screen bg-emerald-50 flex items-center justify-center p-6">
        <div className="max-w-md w-full ui-card text-center space-y-5 p-8">
          <div className="mx-auto w-16 h-16 bg-amber-50 border border-amber-100 rounded-2xl flex items-center justify-center">
            <Settings size={28} className="text-amber-600" />
          </div>
          <div className="space-y-2">
            <h2 className="text-xl font-bold text-primary-deep">Chưa kết nối máy chủ</h2>
            <p className="text-sm text-emerald-800/70 leading-relaxed">
              Ứng dụng cần cấu hình backend để nhận dữ liệu trực tiếp. Vui lòng kiểm tra lại trong phần Cài đặt.
            </p>
          </div>
          <button onClick={() => navigate('/settings')} className="ui-btn-primary w-full">
            Đi tới Cài đặt
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-screen bg-page-bg text-primary-deep font-sans overflow-hidden">
      {/* ── Header (Mobile) ── */}
      <header className="flex items-center justify-between px-4 py-3 bg-white border-b border-line z-30 pt-[calc(env(safe-area-inset-top)+12px)] lg:hidden">
        <div className="flex items-center gap-2.5">
          <div className="w-8 h-8 bg-primary rounded-xl flex items-center justify-center">
            <Sprout size={16} className="text-white" strokeWidth={2.5} />
          </div>
          <div>
            <div className="text-sm font-extrabold tracking-tight text-primary-deep leading-none">HydraGrow</div>
            <div className="text-[10px] text-faint font-semibold mt-0.5 tracking-wide">Khí canh thông minh</div>
          </div>
        </div>
        <div className={`inline-flex items-center gap-1.5 rounded-full px-3 py-1 text-xs font-bold ${
          isSensorOnline ? 'bg-pill text-status' : 'bg-[#FEE2E2] text-error'
        }`}>
          <span className={`w-1.5 h-1.5 rounded-full ${isSensorOnline ? 'bg-status' : 'bg-error'}`} />
          {isSensorOnline ? 'Đang kết nối' : 'Mất tín hiệu'}
        </div>
      </header>

      {/* ── Desktop Sidebar ── */}
      <aside className="hidden lg:flex fixed inset-y-0 left-0 z-20 w-64 flex-col gap-7 border-r border-line bg-white px-5 pb-6 pt-6 shadow-sm">
        <div className="flex items-center gap-2.5">
          <div className="w-9 h-9 bg-gradient-to-br from-emerald-500 to-emerald-700 rounded-full flex items-center justify-center">
            <Sprout size={16} className="text-white" strokeWidth={2.5} />
          </div>
          <span className="text-[18px] font-extrabold tracking-tight text-primary-deep">HydraGrow</span>
        </div>

        <nav aria-label="Điều hướng chính" className="flex flex-col gap-1">
          {navItems.map((item) => {
            const active = isActive(item.path);
            return (
              <button
                key={item.path}
                onClick={() => navigate(item.path)}
                className={`relative flex w-full items-center gap-2.5 rounded-[10px] px-3.5 py-2.5 text-sm transition-colors ${
                  active
                    ? 'bg-emerald-50 font-semibold text-emerald-800'
                    : 'font-normal text-emerald-800/70 hover:bg-emerald-50/70 hover:text-emerald-900'
                }`}
              >
                <item.icon size={16} className={active ? 'text-emerald-700' : 'text-emerald-500'} />
                <span>{item.label}</span>
                {item.hasBadge && (
                  <span className="ml-auto h-2 w-2 rounded-full bg-red-600" aria-label="Có cảnh báo mới" />
                )}
              </button>
            );
          })}
        </nav>

        <div className="mt-auto rounded-xl bg-emerald-50 px-3.5 py-3 space-y-1.5">
          <div className="flex items-center gap-1.5">
            <span className={`h-2 w-2 rounded-full ${isSensorOnline ? 'bg-emerald-500' : 'bg-red-500'}`} />
            <span className="text-xs font-bold text-emerald-800">{isSensorOnline ? 'Trạm Online' : 'Trạm Offline'}</span>
          </div>
          <p className="text-[11px] text-emerald-700/75">ID: {deviceId ?? '—'}</p>
        </div>
      </aside>

      {/* ── Main Content ── */}
      <main className="flex-1 overflow-y-auto pb-24 relative z-10 custom-scrollbar scroll-smooth lg:ml-64 lg:pb-6">
        <Outlet />
      </main>

      {/* ── Bottom Navigation ── */}
      <nav className="fixed bottom-0 left-0 right-0 z-50 lg:hidden px-4 pb-[calc(env(safe-area-inset-bottom)+12px)]">
        <div className="flex items-center justify-between bg-line/80 backdrop-blur-md rounded-full px-3 py-2 border border-white/40 shadow-[0_-8px_24px_rgba(20,83,45,0.07)]">
          {navItems.map((item) => {
            const active = isActive(item.path);
            return (
              <button
                key={item.path}
                onClick={() => navigate(item.path)}
                aria-current={active ? 'page' : undefined}
                className={`relative flex flex-col items-center justify-center w-full gap-1 py-1 rounded-full transition-colors ${active ? '' : 'group'}`}
              >
                <span className={`relative flex items-center justify-center w-1.5 h-1.5 rounded-full transition-colors ${active ? 'bg-primary' : 'bg-faint group-hover:bg-primary/60'}`}>
                  {item.hasBadge && (
                    <span className="absolute -top-1 -right-1.5 w-2 h-2 bg-red-500 rounded-full" />
                  )}
                </span>
                <span className={`text-[9px] font-bold tracking-wide transition-colors ${active ? 'text-primary' : 'text-faint group-hover:text-primary/70'}`}>
                  {item.label}
                </span>
              </button>
            );
          })}
        </div>
      </nav>
    </div>
  );
};

export default MainLayout;
