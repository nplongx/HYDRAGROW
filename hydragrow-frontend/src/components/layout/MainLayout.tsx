import React, { useState, useEffect, useRef, useMemo } from 'react';
import { Outlet, useLocation, useNavigate } from 'react-router-dom';
import {
  LayoutDashboard, SlidersHorizontal, Settings, Sprout,
  AlignLeft, MoreHorizontal, X, Leaf, ClipboardList,
  Box, LineChart, Link, Wifi, WifiOff, Workflow
} from 'lucide-react';
import { useDeviceStore } from '../../store/useDeviceStore';
import { useDeviceSync } from '../../hooks/useDeviceSync';

const MainLayout: React.FC = () => {
  useDeviceSync();
  const [isMenuOpen, setIsMenuOpen] = useState(false);
  const location = useLocation();
  const navigate = useNavigate();
  const menuRef = useRef<HTMLDivElement>(null);

  const isSensorOnline = useDeviceStore((state) => state.isSensorOnline);
  const isMissingConfig = useDeviceStore((state) => state.isMissingConfig);
  const systemEvents = useDeviceStore((state) => state.systemEvents);

  const unreadAlertCount = useMemo(() => {
    if (!systemEvents || !Array.isArray(systemEvents)) return 0;
    return systemEvents.filter((ev: any) => {
      const ts = ev?.timestamp ? new Date(ev.timestamp).getTime() : 0;
      if (!ts || Number.isNaN(ts)) return false;
      const within24h = Date.now() - ts <= 24 * 60 * 60 * 1000;
      const level = String(ev?.level || '').toLowerCase();
      return within24h && (level === 'warning' || level === 'critical' || level === 'error');
    }).length;
  }, [systemEvents]);

  useEffect(() => { setIsMenuOpen(false); }, [location.pathname]);

  const mainNavItems = [
    { path: '/dashboard', icon: LayoutDashboard, label: 'Tổng quan' },
    { path: '/control', icon: SlidersHorizontal, label: 'Điều khiển' },
    { path: '/crop-seasons', icon: Leaf, label: 'Mùa vụ' },
    { path: '/recipes', icon: ClipboardList, label: 'Công thức' },
    { path: '/logs', icon: AlignLeft, label: 'Nhật ký', hasBadge: unreadAlertCount > 0 },
  ];

  const moreMenuItems = [
    { path: '/dosing-history', icon: Box, label: 'Lịch sử châm' },
    { path: '/analytics', icon: LineChart, label: 'Grafana metrics' },
    { path: '/automation', icon: Workflow, label: 'Tự động hóa' },
    { path: '/pairing', icon: Link, label: 'Thiết bị của tôi' },
    { path: '/settings', icon: Settings, label: 'Cài đặt trạm' },
  ];

  const isActiveMore = moreMenuItems.some(item => location.pathname === item.path);
  const isActive = (path: string) => location.pathname === path || (path === '/dashboard' && location.pathname === '/');

  if (isMissingConfig && location.pathname !== '/settings') {
    return (
      <div className="min-h-screen bg-emerald-50 flex items-center justify-center p-6">
        <div className="max-w-md w-full ui-card text-center space-y-5 p-8">
          <div className="mx-auto w-16 h-16 bg-amber-50 border border-amber-100 rounded-2xl flex items-center justify-center">
            <Settings size={28} className="text-amber-600" />
          </div>
          <div className="space-y-2">
            <h2 className="text-xl font-bold text-emerald-950">Chưa cấu hình API Key</h2>
            <p className="text-sm text-emerald-800/70 leading-relaxed">
              Ứng dụng cần <span className="font-semibold text-emerald-800">API Key</span> để kết nối với máy chủ. Vui lòng nhập thông tin trong phần Cài đặt.
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
    <div className="flex flex-col h-screen bg-emerald-50/60 text-emerald-950 font-sans overflow-hidden">
      {/* ── Header ── */}
      <header className="flex items-center justify-between px-4 py-3 bg-white/90 backdrop-blur-md border-b border-emerald-100 z-30 pt-[calc(env(safe-area-inset-top)+12px)]">
        <div className="flex items-center gap-2.5">
          <div className="w-8 h-8 bg-gradient-to-br from-emerald-500 to-emerald-700 rounded-xl flex items-center justify-center shadow-sm shadow-emerald-500/30">
            <Sprout size={16} className="text-white" strokeWidth={2.5} />
          </div>
          <div>
            <div className="text-sm font-extrabold tracking-tight text-emerald-950 leading-none">HydraGrow</div>
            <div className="text-[10px] text-emerald-700/60 font-semibold mt-0.5 tracking-wide">Khí canh thông minh</div>
          </div>
        </div>
        <div className={`farm-status-pill ${isSensorOnline
          ? 'bg-emerald-50 text-emerald-700 border-emerald-200'
          : 'bg-red-50 text-red-700 border-red-200'
        }`}>
          {isSensorOnline
            ? <><Wifi size={11} strokeWidth={2.5} /> Đang kết nối</>
            : <><WifiOff size={11} strokeWidth={2.5} /> Mất tín hiệu</>
          }
        </div>
      </header>

      {/* ── Main Content ── */}
      <main className="flex-1 overflow-y-auto pb-24 relative z-10 custom-scrollbar scroll-smooth">
        <Outlet />
      </main>

      {/* ── More Menu Overlay ── */}
      <div
        className={`fixed inset-0 bg-emerald-950/20 backdrop-blur-[2px] z-40 transition-opacity duration-200 ${isMenuOpen ? 'opacity-100' : 'opacity-0 pointer-events-none'}`}
        onClick={() => setIsMenuOpen(false)}
      />

      {/* ── More Menu Popup ── */}
      <div
        ref={menuRef}
        className={`fixed bottom-[88px] left-3 right-3 z-50 transition-all duration-200 ease-out origin-bottom ${isMenuOpen ? 'scale-100 opacity-100' : 'scale-95 opacity-0 pointer-events-none'}`}
      >
        <div className="bg-white border border-emerald-100 rounded-2xl overflow-hidden shadow-xl shadow-emerald-950/10">
          {moreMenuItems.map((item, index) => {
            const active = location.pathname === item.path;
            return (
              <button
                key={item.path}
                onClick={() => navigate(item.path)}
                className={`w-full flex items-center gap-3 px-4 py-3.5 text-sm transition-colors ${index !== moreMenuItems.length - 1 ? 'border-b border-emerald-50' : ''} ${active ? 'bg-emerald-50 text-emerald-800 font-bold' : 'text-emerald-900 hover:bg-emerald-50/50'}`}
              >
                <item.icon size={17} strokeWidth={active ? 2.5 : 2} className={active ? 'text-emerald-700' : 'text-emerald-500'} />
                <span>{item.label}</span>
              </button>
            );
          })}
        </div>
      </div>

      {/* ── Bottom Navigation ── */}
      <nav className="fixed bottom-0 left-0 right-0 z-50 bg-white/95 backdrop-blur-md border-t border-emerald-100 pb-[env(safe-area-inset-bottom)] shadow-[0_-8px_24px_rgba(20,83,45,0.07)]">
        <div className="flex items-center justify-around h-[60px] px-1">
          {mainNavItems.map((item) => {
            const active = isActive(item.path);
            return (
              <button
                key={item.path}
                onClick={() => navigate(item.path)}
                className="relative flex flex-col items-center justify-center w-full h-full gap-1 group"
              >
                <div className={`relative flex items-center justify-center transition-all duration-200 ${active ? 'bg-emerald-100 rounded-xl px-2.5 py-1 -mt-1' : ''}`}>
                  <item.icon
                    size={active ? 20 : 22}
                    strokeWidth={active ? 2.5 : 1.8}
                    className={active ? 'text-emerald-700' : 'text-emerald-400 group-hover:text-emerald-600'}
                  />
                  {item.hasBadge && (
                    <span className="absolute -top-0.5 -right-0.5 w-2 h-2 bg-red-500 rounded-full border-2 border-white" />
                  )}
                </div>
                <span className={`text-[10px] font-semibold tracking-wide transition-colors ${active ? 'text-emerald-800 font-bold' : 'text-emerald-700/55 group-hover:text-emerald-800'}`}>
                  {item.label}
                </span>
              </button>
            );
          })}
          {/* More button */}
          <button
            onClick={() => setIsMenuOpen(!isMenuOpen)}
            className="relative flex flex-col items-center justify-center w-full h-full gap-1 group"
          >
            <div className={`flex items-center justify-center transition-all duration-200 ${isMenuOpen ? 'bg-emerald-700 rounded-xl px-2.5 py-1 -mt-1' : isActiveMore ? 'bg-emerald-100 rounded-xl px-2.5 py-1 -mt-1' : ''}`}>
              {isMenuOpen
                ? <X size={20} strokeWidth={2.5} className="text-white" />
                : <MoreHorizontal size={22} strokeWidth={isActiveMore ? 2.5 : 1.8} className={isActiveMore ? 'text-emerald-700' : 'text-emerald-400 group-hover:text-emerald-600'} />
              }
            </div>
            <span className={`text-[10px] font-semibold tracking-wide transition-colors ${isActiveMore ? 'text-emerald-800 font-bold' : 'text-emerald-700/55 group-hover:text-emerald-800'}`}>
              Thêm
            </span>
          </button>
        </div>
      </nav>
    </div>
  );
};

export default MainLayout;
