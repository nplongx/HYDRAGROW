import { useState, useEffect, useRef } from 'react';
import { Outlet, NavLink, useLocation } from 'react-router-dom';
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
  Zap,
} from 'lucide-react';

const MainLayout = () => {
  const [isMenuOpen, setIsMenuOpen] = useState(false);
  const location = useLocation();
  const menuRef = useRef<HTMLDivElement>(null);

  // Danh sách các mục điều hướng chính
  const mainNavItems = [
    { path: '/', icon: LayoutDashboard, label: 'Tổng quan' },
    { path: '/control', icon: SlidersHorizontal, label: 'Điều khiển' },
    { path: '/analytics', icon: LineChart, label: 'Phân tích' },
    // { path: '/system-flow', icon: ActivityIcon, label: 'FSM' },
    { path: '/logs', icon: AlignLeft, label: 'Nhật ký' },
  ];

  // Các mục bổ sung trong menu "Thêm"
  const moreMenuItems = [
    {
      path: '/crop-seasons',
      icon: Sprout,
      label: 'Mùa vụ',
      color: 'text-emerald-400',
      bg: 'bg-emerald-500/10',
    },
    {
      path: '/blockchain',
      icon: ShieldCheck,
      label: 'Niêm phong',
      color: 'text-blue-400',
      bg: 'bg-blue-500/10',
    },
    {
      path: '/settings',
      icon: Settings,
      label: 'Cài đặt',
      color: 'text-slate-400',
      bg: 'bg-slate-500/10',
    },
  ];

  const isActiveMore = moreMenuItems.some((item) => location.pathname === item.path);

  useEffect(() => {
    setIsMenuOpen(false);
  }, [location.pathname]);

  return (
    <div className="flex flex-col h-screen bg-slate-950 text-slate-50 font-sans overflow-hidden pt-[env(safe-area-inset-top)] relative">
      {/* 🟢 Hiệu ứng nền Mesh Gradient mờ ảo (Tạo chiều sâu cho App) */}
      <div className="absolute top-[-10%] left-[-10%] w-[40%] h-[40%] bg-emerald-500/5 rounded-full blur-[120px] pointer-events-none"></div>
      <div className="absolute bottom-[-10%] right-[-10%] w-[40%] h-[40%] bg-blue-500/5 rounded-full blur-[120px] pointer-events-none"></div>

      {/* Khu vực nội dung chính */}
      <main className="flex-1 overflow-y-auto pb-28 relative z-10">
        <Outlet />
      </main>

      {/* 🟢 OVERLAY MỜ ẢO KHI MỞ MENU */}
      <div
        className={`fixed inset-0 bg-slate-950/40 backdrop-blur-md z-40 transition-opacity duration-500 ${
          isMenuOpen ? 'opacity-100' : 'opacity-0 pointer-events-none'
        }`}
        onClick={() => setIsMenuOpen(false)}
      />

      {/* 🟢 MENU "THÊM" DẠNG CRYSTAL SHEET */}
      <div
        ref={menuRef}
        className={`fixed bottom-28 left-4 right-4 z-50 transition-all duration-500 cubic-bezier(0.4, 0, 0.2, 1) ${
          isMenuOpen
            ? 'translate-y-0 opacity-100 scale-100'
            : 'translate-y-20 opacity-0 scale-90 pointer-events-none'
        }`}
      >
        <div className="bg-slate-900/80 backdrop-blur-2xl border border-slate-700/50 rounded-[32px] p-3 shadow-[0_20px_50px_rgba(0,0,0,0.5)]">
          <div className="grid grid-cols-1 gap-2">
            {moreMenuItems.map((item) => (
              <NavLink
                key={item.path}
                to={item.path}
                className={({ isActive }) =>
                  `flex items-center gap-4 p-4 rounded-2xl transition-all duration-300 ${
                    isActive
                      ? 'bg-gradient-to-r from-emerald-500/20 to-blue-500/10 border border-emerald-500/30 shadow-lg shadow-emerald-500/5'
                      : 'hover:bg-white/5 border border-transparent'
                  }`
                }
              >
                <div
                  className={`p-2.5 rounded-xl bg-slate-800 shadow-inner ${location.pathname === item.path ? item.color : 'text-slate-400'}`}
                >
                  <item.icon size={20} />
                </div>
                <div className="flex flex-col">
                  <span
                    className={`text-sm font-bold ${location.pathname === item.path ? 'text-white' : 'text-slate-300'}`}
                  >
                    {item.label}
                  </span>
                </div>
                {location.pathname === item.path && (
                  <Zap size={14} className="ml-auto text-emerald-400 animate-pulse fill-current" />
                )}
              </NavLink>
            ))}
          </div>
        </div>
      </div>

      {/* 🟢 BOTTOM NAVIGATION - NEON GLASSMORMISM */}
      <nav className="fixed bottom-6 left-6 right-6 z-50">
        <div className="bg-slate-900/70 backdrop-blur-xl border border-white/10 rounded-full h-16 px-4 shadow-[0_10px_30px_rgba(0,0,0,0.3),inset_0_1px_1px_rgba(255,255,255,0.1)] flex justify-between items-center relative overflow-hidden">
          {mainNavItems.map((item) => (
            <NavLink
              key={item.path}
              to={item.path}
              className={({ isActive }) =>
                `relative flex flex-col items-center justify-center flex-1 h-full transition-all duration-300 z-10 ${
                  isActive ? 'text-emerald-400' : 'text-slate-500 hover:text-slate-300'
                }`
              }
            >
              <div className="relative group">
                <item.icon
                  size={22}
                  strokeWidth={location.pathname === item.path ? 2.5 : 2}
                  className={`transition-all duration-300 ${location.pathname === item.path ? 'drop-shadow-[0_0_8px_rgba(52,211,153,0.8)] scale-110' : ''}`}
                />
              </div>
              <span
                className={`text-[9px] mt-1 font-bold tracking-tighter uppercase transition-opacity duration-300 ${location.pathname === item.path ? 'opacity-100' : 'opacity-0'}`}
              >
                {item.label}
              </span>

              {/* Chỉ báo ánh sáng dưới chân mục đang active */}
              {location.pathname === item.path && (
                <div className="absolute -bottom-1 w-8 h-1 bg-emerald-400 rounded-full blur-[2px] animate-in slide-in-from-bottom-1" />
              )}
            </NavLink>
          ))}

          {/* Nút "Thêm" đặc biệt */}
          <button
            onClick={() => setIsMenuOpen(!isMenuOpen)}
            className={`relative flex flex-col items-center justify-center flex-1 h-full transition-all duration-300 z-10 ${
              isActiveMore || isMenuOpen
                ? 'text-emerald-400'
                : 'text-slate-500 hover:text-slate-300'
            }`}
          >
            <div
              className={`p-1.5 rounded-full transition-all duration-500 ${isMenuOpen ? 'rotate-90 bg-emerald-500 text-slate-950 scale-110' : ''}`}
            >
              {isMenuOpen ? (
                <X size={20} strokeWidth={3} />
              ) : (
                <MoreHorizontal
                  size={22}
                  strokeWidth={isActiveMore ? 2.5 : 2}
                  className={isActiveMore ? 'drop-shadow-[0_0_8px_rgba(52,211,153,0.8)]' : ''}
                />
              )}
            </div>
            <span
              className={`text-[9px] mt-1 font-bold uppercase transition-opacity duration-300 ${isActiveMore || isMenuOpen ? 'opacity-100' : 'opacity-0'}`}
            >
              Thêm
            </span>
          </button>

          {/* 🟢 Hiệu ứng vết sáng quét ngang thanh điều hướng (Optional trang trí) */}
          <div className="absolute inset-0 bg-gradient-to-r from-transparent via-white/5 to-transparent -translate-x-full animate-[shimmer_5s_infinite] pointer-events-none"></div>
        </div>
      </nav>

      {/* Padding dưới cùng để bù cho thanh điều hướng đang lơ lửng */}
      <div className="h-10 pb-[env(safe-area-inset-bottom)]"></div>
    </div>
  );
};

export default MainLayout;
