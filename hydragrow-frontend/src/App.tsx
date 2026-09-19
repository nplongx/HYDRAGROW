// App.tsx
import React, { Suspense, useState, useEffect } from 'react';
import {
  BrowserRouter as Router,
  Routes,
  Route,
  Navigate,
  Link,
  useLocation,
} from 'react-router-dom';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import MainLayout from './components/layout/MainLayout';
import { AppToaster } from './components/ui/AppToaster';
import { LoadingState } from './components/ui/LoadingState';
import { AuthProvider, useAuth } from './contexts/AuthContext';
import { StationProvider, useStationContext } from './contexts/StationContext';
import { LoginScreen } from './components/auth/LoginScreen';
import { RegisterScreen } from './components/auth/RegisterScreen';
import { ForgotPasswordScreen } from './components/auth/ForgotPasswordScreen';
import { useWhoami } from './hooks/useWhoami';
import { legacyTarget } from './lib/routeState';
import {
  CANONICAL_ROUTES,
  LEGACY_ROUTES,
  type RouteDefinition,
} from './routes';
import './App.css';

// Khởi tạo QueryClient
const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 60 * 1000,
      gcTime: 5 * 60 * 1000,
      retry: 2,
      refetchOnWindowFocus: false,
    },
  },
});

const Dashboard = React.lazy(() => import('./pages/Dashboard'));
const Operations = React.lazy(() => import('./pages/Operations').then((module) => ({ default: module.Operations })));
const Cultivation = React.lazy(() => import('./pages/Cultivation'));
const Journal = React.lazy(() => import('./pages/Journal'));
const Settings = React.lazy(() => import('./pages/Settings'));
const DevicePairing = React.lazy(() => import('./pages/DevicePairing').then((module) => ({ default: module.DevicePairing })));
const FleetView = React.lazy(() => import('./pages/FleetView').then((module) => ({ default: module.FleetView })));
const ConfigBackup = React.lazy(() => import('./pages/ConfigBackup').then((module) => ({ default: module.ConfigBackup })));
const Roles = React.lazy(() => import('./pages/Roles').then((module) => ({ default: module.Roles })));
const UserManagement = React.lazy(() => import('./pages/UserManagement').then((module) => ({ default: module.UserManagement })));
const DesignLab = React.lazy(() => import('./pages/DesignLab'));

type AuthView = 'login' | 'register' | 'forgot';

function AuthGate({ children }: { children: React.ReactNode }) {
  const { status } = useAuth();
  const [view, setView] = useState<AuthView>('login');

  useEffect(() => {
    if (import.meta.env.DEV || import.meta.env.MODE === 'test') {
      if (typeof window !== 'undefined' && window.location.search.includes('mock_auth=true')) {
        localStorage.setItem('mock_auth', 'true');
      }
    }
  }, []);

  const isMockAuth = (import.meta.env.DEV || import.meta.env.MODE === 'test') && typeof window !== 'undefined' && (
    window.location.search.includes('mock_auth=true') ||
    localStorage.getItem('mock_auth') === 'true'
  );

  if (isMockAuth) return <>{children}</>;

  if (status === 'loading') {
    return <LoadingState message="Đang kiểm tra đăng nhập..." />;
  }

  if (status === 'unauthenticated') {
    switch (view) {
      case 'register':
        return <RegisterScreen key="register" onShowLogin={() => setView('login')} />;
      case 'forgot':
        return <ForgotPasswordScreen key="forgot" onShowLogin={() => setView('login')} />;
      default:
        return (
          <LoginScreen
            key="login"
            onShowRegister={() => setView('register')}
            onShowForgot={() => setView('forgot')}
          />
        );
    }
  }

  return <>{children}</>;
}

function AccessDenied({ capability }: { capability: string }) {
  return (
    <div className="min-h-screen flex items-center justify-center p-6">
      <div className="ui-card max-w-md w-full space-y-3 p-8 text-center">
        <h1 className="text-xl font-bold text-primary-deep">Không có quyền truy cập</h1>
        <p className="text-sm text-text-muted">
          Tài khoản hiện tại không có năng lực cần thiết cho trang này ({capability}).
        </p>
      </div>
    </div>
  );
}

function RouteRecovery({ status }: { status: string }) {
  const target = status === 'NoSelection' || status === 'InvalidSelection' || status === 'Unavailable' || status === 'PermissionDenied'
    ? '/pairing'
    : '/dashboard';
  const message = {
    NoSelection: 'Chưa chọn trạm. Chọn hoặc ghép nối trạm trước khi mở trang này.',
    InvalidSelection: 'Trạm đang chọn không còn khả dụng cho tài khoản này.',
    Unavailable: 'Không thể xác định trạm khả dụng lúc này.',
    PermissionDenied: 'Tài khoản không còn quyền truy cập trạm đang chọn.',
  }[status] ?? 'Không thể xác định ngữ cảnh cho trang này.';

  return (
    <div className="min-h-screen flex items-center justify-center p-6">
      <div className="ui-card max-w-md w-full space-y-4 p-8 text-center">
        <h1 className="text-xl font-bold text-primary-deep">Thiếu ngữ cảnh trạm</h1>
        <p className="text-sm text-text-muted">{message}</p>
        <Link className="ui-btn-primary inline-flex" to={target}>Tiếp tục</Link>
      </div>
    </div>
  );
}

function CapabilityGate({ definition, children }: { definition: RouteDefinition; children: React.ReactNode }) {
  const { data: whoami, isLoading, error } = useWhoami();
  if (isLoading) return <LoadingState message="Đang xác minh quyền truy cập..." />;
  if ((error as { status?: number } | null)?.status === 401) {
    return <LoadingState message="Phiên đăng nhập không còn hợp lệ." />;
  }
  if (error || !whoami || !whoami.is_active || (definition.capability === 'admin' && whoami.role !== 'admin')) {
    return <AccessDenied capability={definition.capability ?? 'unknown'} />;
  }
  return <>{children}</>;
}

function DeviceScopeGate({ children }: { children: React.ReactNode }) {
  const { status, selectedDevice } = useStationContext();
  if (status === 'LoadingSelection') return <LoadingState message="Đang xác định trạm đang chọn..." />;
  if (status !== 'Selected' || !selectedDevice) return <RouteRecovery status={status} />;
  return <>{children}</>;
}

function RouteEntry({ definition, children }: { definition: RouteDefinition; children: React.ReactNode }) {
  let content = children;
  if (definition.scope === 'device' || definition.scope === 'station') {
    content = <DeviceScopeGate>{content}</DeviceScopeGate>;
  }
  if (definition.capability) {
    content = <CapabilityGate definition={definition}>{content}</CapabilityGate>;
  }
  return <>{content}</>;
}

function renderCanonicalRoute(route: RouteDefinition) {
  switch (route.id) {
    case 'dashboard': return <Dashboard />;
    case 'operations': return <Operations />;
    case 'cultivation': return <Cultivation />;
    case 'journal': return <Journal />;
    case 'settings': return <Settings />;
    case 'pairing': return <DevicePairing />;
    case 'fleet': return <FleetView />;
    case 'config-backup': return <ConfigBackup />;
    case 'user-management': return <UserManagement />;
    case 'roles': return <Roles />;
    default: return <NotFound />;
  }
}

function NotFound() {
  return (
    <div className="min-h-screen flex items-center justify-center p-6">
      <div className="ui-card max-w-md w-full space-y-3 p-8 text-center">
        <h1 className="text-xl font-bold text-primary-deep">Không tìm thấy trang</h1>
        <p className="text-sm text-text-muted">Đường dẫn không thuộc không gian URL của HydraGrow.</p>
        <Link className="ui-btn-primary inline-flex" to="/dashboard">Về Tổng quan</Link>
      </div>
    </div>
  );
}

function LegacyRedirect() {
  const location = useLocation();
  const target = legacyTarget(location.pathname, location.search, location.hash);
  return <Navigate to={target ?? '/dashboard'} replace />;
}

function AppRoutes() {
  return (
    <Router>
      <AppToaster />
      <Suspense fallback={<LoadingState message="Đang tải trang..." />}>
        <Routes>
          <Route path="/design-lab" element={<DesignLab />} />
          <Route path="/" element={<MainLayout />}>
            <Route index element={<Navigate to="/dashboard" replace />} />
            {CANONICAL_ROUTES.map((route) => (
              <Route
                key={route.id}
                path={route.path.slice(1)}
                element={
                  <RouteEntry definition={route}>
                    {renderCanonicalRoute(route)}
                  </RouteEntry>
                }
              />
            ))}
            {LEGACY_ROUTES.filter((route) => route.path !== '/').map((route) => (
              <Route key={route.id} path={route.path.slice(1)} element={<LegacyRedirect />} />
            ))}
          </Route>
          <Route path="*" element={<NotFound />} />
        </Routes>
      </Suspense>
    </Router>
  );
}

function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <AuthProvider>
        <AuthGate>
          <StationProvider>
            <AppRoutes />
          </StationProvider>
        </AuthGate>
      </AuthProvider>
    </QueryClientProvider>
  );
}

export default App;
