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
} from 'lucide-react';

const MainLayout = () => {
  const [isMenuOpen, setIsMenuOpen] = useState(false);
  const location = useLocation();
  const menuRef = useRef<HTMLDivElement>(null);

  const mainNavItems = [
    { path: '/', icon: LayoutDashboard, label: 'Tổng quan' },
    { path: '/control', icon: SlidersHorizontal, label: 'Điều khiển' },
    { path: '/analytics', icon: LineChart, label: 'Phân tích' },
    { path: '/logs', icon: AlignLeft, label: 'Nhật ký' }
  ];

  const moreMenuItems = [
    { path: '/crop-seasons', icon: Sprout, label: 'Mùa vụ' },
    { path: '/blockchain', icon: ShieldCheck, label: 'Niêm phong' },
    { path: '/settings', icon: Settings, label: 'Cài đặt' }
  ];

  const isActiveMore = moreMenuItems.some(item => location.pathname === item.path);

  useEffect(() => {
    setIsMenuOpen(false);
  }, [location.pathname]);

  return (
    <div className="flex flex-col h-screen bg-slate-950 text-slate-50 font-sans overflow-hidden pt-[env(safe-area-inset-top)] relative">
      <main className="flex-1 overflow-y-auto pb-28 relative z-10">
        <Outlet />
      </main>

      <div
        className={`fixed inset-0 bg-slate-950/70 z-40 transition-opacity duration-300 ${isMenuOpen ? 'opacity-100' : 'opacity-0 pointer-events-none'}`}
        onClick={() => setIsMenuOpen(false)}
      />

      <div
        ref={menuRef}
        className={`fixed bottom-28 left-4 right-4 z-50 transition-all duration-300 ${isMenuOpen ? 'translate-y-0 opacity-100' : 'translate-y-8 opacity-0 pointer-events-none'}`}
      >
        <div className="bg-slate-900 border border-slate-800 rounded-2xl p-3">
          <div className="grid grid-cols-1 gap-2">
            {moreMenuItems.map((item) => (
              <NavLink
                key={item.path}
                to={item.path}
                className={({ isActive }) =>
                  `flex items-center gap-4 p-3.5 rounded-xl border transition-colors ${isActive
                    ? 'bg-slate-800 text-slate-100 border-slate-600'
                    : 'text-slate-300 border-transparent hover:bg-slate-800/70'
                  }`
                }
              >
                <div className="p-2 rounded-lg bg-slate-800 text-slate-300">
                  <item.icon size={20} />
                </div>
                <span className="text-sm font-semibold">{item.label}</span>
              </NavLink>
            ))}
          </div>
        </div>
      </div>

      <nav className="fixed bottom-6 left-6 right-6 z-50">
        <div className="bg-slate-900 border border-slate-800 rounded-2xl h-16 px-3 flex justify-between items-center relative">
          {mainNavItems.map((item) => (
            <NavLink
              key={item.path}
              to={item.path}
              className={({ isActive }) =>
                `relative flex flex-col items-center justify-center flex-1 h-full transition-colors z-10 ${isActive ? 'text-slate-100' : 'text-slate-400 hover:text-slate-200'}`
              }
            >
              <item.icon size={22} strokeWidth={location.pathname === item.path ? 2.5 : 2} />
              <span className={`text-[9px] mt-1 font-semibold tracking-tight uppercase ${location.pathname === item.path ? 'opacity-100' : 'opacity-0'}`}>
                {item.label}
              </span>
              {location.pathname === item.path && (
                <div className="absolute -bottom-0.5 w-7 h-0.5 bg-slate-300 rounded-full" />
              )}
            </NavLink>
          ))}

          <button
            onClick={() => setIsMenuOpen(!isMenuOpen)}
            className={`relative flex flex-col items-center justify-center flex-1 h-full transition-colors z-10 ${isActiveMore || isMenuOpen ? 'text-slate-100' : 'text-slate-400 hover:text-slate-200'}`}
          >
            <div className={`p-1.5 rounded-full transition-transform duration-300 ${isMenuOpen ? 'rotate-90 bg-slate-200 text-slate-900' : ''}`}>
              {isMenuOpen ? <X size={20} strokeWidth={3} /> : <MoreHorizontal size={22} strokeWidth={isActiveMore ? 2.5 : 2} />}
            </div>
            <span className={`text-[9px] mt-1 font-semibold uppercase ${isActiveMore || isMenuOpen ? 'opacity-100' : 'opacity-0'}`}>
              Thêm
            </span>
          </button>
        </div>
      </nav>

      <div className="h-10 pb-[env(safe-area-inset-bottom)]"></div>
    </div>
  );
};

export default MainLayout;
