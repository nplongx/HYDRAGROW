import React, { useState, useEffect, useRef, useMemo } from 'react';
import { Outlet, useLocation, useNavigate } from 'react-router-dom';
import {
  LayoutDashboard,
  SlidersHorizontal,
  LineChart,
  Settings,
  ShieldCheck,
  Sprout,
  AlignLeft,
  MoreHorizontal,
  X,
  Activity,
  Leaf
} from 'lucide-react';
import { useDeviceContext } from '../../context/DeviceContext';

const MainLayout: React.FC = () => {
  const [isMenuOpen, setIsMenuOpen] = useState(false);
  const location = useLocation();
  const navigate = useNavigate();
  const menuRef = useRef<HTMLDivElement>(null);

  // Lấy dữ liệu systemEvents từ Context thay vì tự fetch
  const { isSensorOnline, systemEvents, isMissingConfig } = useDeviceContext();

  // Tự động tính số lượng thông báo chưa đọc / cảnh báo mới trong 24h
  const unreadAlertCount = useMemo(() => {
    if (!systemEvents || !Array.isArray(systemEvents)) return 0;

    return systemEvents.filter((ev: any) => {
      const ts = ev?.timestamp ? new Date(ev.timestamp).getTime() : 0;
      if (!ts || Number.isNaN(ts)) return false;

      // Lọc các cảnh báo trong 24h qua có level là warning/critical
      const within24h = Date.now() - ts <= 24 * 60 * 60 * 1000;
      const level = String(ev?.level || '').toLowerCase();
      const isWarning = level === 'warning' || level === 'critical' || level === 'error';

      // Nếu dữ liệu của bạn có cờ is_read, hãy thêm điều kiện: && !ev.is_read
      return within24h && isWarning;
    }).length;
  }, [systemEvents]);

  // Đóng menu "Thêm" khi chuyển trang
  useEffect(() => {
    setIsMenuOpen(false);
  }, [location.pathname]);

  const mainNavItems = [
    { path: '/', icon: LayoutDashboard, label: 'Dashboard' },
    { path: '/control', icon: SlidersHorizontal, label: 'Điều Khiển' },
    { path: '/analytics', icon: LineChart, label: 'Phân Tích' },
    { path: '/logs', icon: AlignLeft, label: 'Nhật Ký', hasBadge: unreadAlertCount > 0 }
  ];

  const moreMenuItems = [
    { path: '/crop-seasons', icon: Leaf, label: 'Mùa Vụ' },
    { path: '/history', icon: ShieldCheck, label: 'Giao Dịch' },
    { path: '/settings', icon: Settings, label: 'Cài Đặt' }
  ];

  const isActiveMore = moreMenuItems.some(item => location.pathname === item.path);


  if (isMissingConfig) {
    return (
      <div className="min-h-screen bg-slate-950 text-slate-100 flex items-center justify-center p-6">
        <div className="max-w-md w-full ui-card text-center space-y-3">
          <h2 className="text-xl font-semibold">Thiếu cấu hình ứng dụng</h2>
          <p className="text-slate-400 text-sm">
            Ứng dụng web chưa có <b>backend URL</b> hoặc <b>API key</b>. Hãy cung cấp qua
            <code className="mx-1">window.__APP_CONFIG__</code>, localStorage hoặc <code>/config.json</code>.
          </p>
        </div>
      </div>
    );
  }
  return (
    <div className="flex flex-col h-screen bg-slate-950 text-slate-100 font-sans overflow-hidden">

      {/* 🟢 Top Header (Trạng thái thiết bị) */}
      <header className="flex items-center justify-between px-5 py-3 bg-slate-900 border-b border-slate-800 z-30 pt-[calc(env(safe-area-inset-top)+12px)]">
        <div className="flex items-center gap-2.5">
          <div className="w-7 h-7 bg-blue-600 rounded flex items-center justify-center">
            <Sprout size={16} className="text-white" />
          </div>
          <div>
            <h1 className="text-sm font-semibold tracking-tight leading-none">HydraGrow</h1>
            <p className="text-[10px] text-slate-400 font-medium">Tủ điện thông minh</p>
          </div>
        </div>

        <div className={`flex items-center gap-1.5 px-2.5 py-1 rounded text-[11px] font-medium border ${isSensorOnline ? 'bg-emerald-500/10 text-emerald-400 border-emerald-500/20' : 'bg-red-500/10 text-red-400 border-red-500/20'}`}>
          <Activity size={12} />
          {isSensorOnline ? 'Online' : 'Offline'}
        </div>
      </header>

      {/* 🟢 Main Content Area */}
      <main className="flex-1 overflow-y-auto pb-24 relative z-10 custom-scrollbar scroll-smooth">
        <Outlet />
      </main>

      {/* 🟢 Overlay mờ khi mở Menu "Thêm" */}
      <div
        className={`fixed inset-0 bg-black/60 backdrop-blur-sm z-40 transition-opacity duration-300 ${isMenuOpen ? 'opacity-100' : 'opacity-0 pointer-events-none'}`}
        onClick={() => setIsMenuOpen(false)}
      />

      {/* 🟢 Menu "Thêm" (Popup từ dưới lên) */}
      <div
        ref={menuRef}
        className={`fixed bottom-[84px] left-4 right-4 z-50 transition-all duration-300 ease-out ${isMenuOpen ? 'translate-y-0 opacity-100' : 'translate-y-4 opacity-0 pointer-events-none'}`}
      >
        <div className="bg-slate-900 border border-slate-800 rounded-xl overflow-hidden shadow-xl">
          <div className="flex flex-col">
            {moreMenuItems.map((item, index) => {
              const isActive = location.pathname === item.path;
              return (
                <button
                  key={item.path}
                  onClick={() => navigate(item.path)}
                  className={`flex items-center gap-3 px-4 py-3.5 transition-colors ${index !== moreMenuItems.length - 1 ? 'border-b border-slate-800/50' : ''
                    } ${isActive ? 'bg-slate-800/80 text-blue-400' : 'text-slate-300 hover:bg-slate-800/50'}`}
                >
                  <item.icon size={18} strokeWidth={isActive ? 2.5 : 2} />
                  <span className="text-sm font-medium">{item.label}</span>
                </button>
              );
            })}
          </div>
        </div>
      </div>

      {/* 🟢 Bottom Navigation Bar (Minimalist Flat Design) */}
      <nav className="fixed bottom-0 left-0 right-0 z-50 bg-slate-900 border-t border-slate-800 pb-[env(safe-area-inset-bottom)]">
        <div className="flex items-center justify-around h-16 px-2">
          {mainNavItems.map((item) => {
            const isActive = location.pathname === item.path;
            return (
              <button
                key={item.path}
                onClick={() => navigate(item.path)}
                className="relative flex flex-col items-center justify-center w-full h-full group"
              >
                <div className={`transition-colors duration-200 ${isActive ? 'text-blue-500' : 'text-slate-400 group-hover:text-slate-300'}`}>
                  <item.icon size={22} strokeWidth={isActive ? 2.5 : 2} />
                  {item.hasBadge && (
                    <span className="absolute top-2 right-1/4 translate-x-2 -translate-y-1 w-2.5 h-2.5 bg-red-500 rounded-full border-2 border-slate-900"></span>
                  )}
                </div>
                <span className={`text-[10px] mt-1 font-medium tracking-wide ${isActive ? 'text-blue-500' : 'text-slate-500 group-hover:text-slate-400'}`}>
                  {item.label}
                </span>

                {/* Dấu chấm active thay thế cho gạch chân rườm rà */}
                {isActive && (
                  <div className="absolute top-1 w-1 h-1 bg-blue-500 rounded-full" />
                )}
              </button>
            );
          })}

          {/* Nút "Thêm" (More Menu) */}
          <button
            onClick={() => setIsMenuOpen(!isMenuOpen)}
            className="relative flex flex-col items-center justify-center w-full h-full group"
          >
            <div className={`transition-all duration-300 ${isMenuOpen
              ? 'bg-slate-200 text-slate-900 p-1 rounded-full rotate-90'
              : isActiveMore
                ? 'text-blue-500'
                : 'text-slate-400 group-hover:text-slate-300'
              }`}>
              {isMenuOpen ? <X size={18} strokeWidth={2.5} /> : <MoreHorizontal size={22} strokeWidth={isActiveMore ? 2.5 : 2} />}
            </div>
            {!isMenuOpen && (
              <span className={`text-[10px] mt-1 font-medium tracking-wide ${isActiveMore ? 'text-blue-500' : 'text-slate-500 group-hover:text-slate-400'}`}>
                Thêm
              </span>
            )}
            {isActiveMore && !isMenuOpen && (
              <div className="absolute top-1 w-1 h-1 bg-blue-500 rounded-full" />
            )}
          </button>
        </div>
      </nav>
    </div>
  );
};

export default MainLayout;
