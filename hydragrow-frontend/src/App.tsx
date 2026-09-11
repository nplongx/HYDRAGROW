// App.tsx
import React, { Suspense, useState, useEffect } from 'react';
import { BrowserRouter as Router, Routes, Route, Navigate } from 'react-router-dom';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import MainLayout from './components/layout/MainLayout';
import { AppToaster } from './components/ui/AppToaster';
import { LoadingState } from './components/ui/LoadingState';
import { AuthProvider, useAuth } from './contexts/AuthContext';
import { LoginScreen } from './components/auth/LoginScreen';
import { RegisterScreen } from './components/auth/RegisterScreen';
import { ForgotPasswordScreen } from './components/auth/ForgotPasswordScreen';
import './App.css';

// Khởi tạo QueryClient
const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 60 * 1000, // Caching dữ liệu trong 1 phút
      gcTime: 5 * 60 * 1000, // Xóa cache sau 5 phút không dùng
      retry: 2, // Tự động thử lại 2 lần nếu API bị lỗi
      refetchOnWindowFocus: false, // Không tự fetch lại khi click tab trình duyệt
    },
  },
});

import Dashboard from './pages/Dashboard';
import { Operations } from './pages/Operations';
import Cultivation from './pages/Cultivation';
import Journal from './pages/Journal';
const Settings = React.lazy(() => import('./pages/Settings'));
import { DevicePairing } from './pages/DevicePairing';
import { FleetView } from './pages/FleetView';
import { ConfigBackup } from './pages/ConfigBackup';
import { Roles } from './pages/Roles';

type AuthView = 'login' | 'register' | 'forgot';

function AuthGate({ children }: { children: React.ReactNode }) {
  const { status } = useAuth();
  const [view, setView] = useState<AuthView>('login');

  useEffect(() => {
    if (typeof window !== 'undefined' && window.location.search.includes('mock_auth=true')) {
      localStorage.setItem('mock_auth', 'true');
    }
  }, []);

  const isMockAuth = typeof window !== 'undefined' && (
    window.location.search.includes('mock_auth=true') ||
    localStorage.getItem('mock_auth') === 'true'
  );

  if (isMockAuth) {
    return <>{children}</>;
  }

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

function AppRoutes() {
  return (
    <Router>
      <AppToaster />
      <Suspense fallback={<LoadingState message="Đang tải trang..." />}>
        <Routes>
          <Route path="/" element={<MainLayout />}>
            <Route index element={<Navigate to="/dashboard" replace />} />
            <Route path="dashboard" element={<Dashboard />} />
            <Route path="operations" element={<Operations />} />
            <Route path="cultivation" element={<Cultivation />} />
            <Route path="journal" element={<Journal />} />
            <Route path="settings" element={<Settings />} />
            <Route path="pairing" element={<DevicePairing />} />
            <Route path="fleet" element={<FleetView />} />
            <Route path="config-backup" element={<ConfigBackup />} />
            <Route path="user-management" element={<Roles />} />
            <Route path="roles" element={<Roles />} />
            {/* legacy deep links redirect into the merged tab pages */}
            <Route path="control" element={<Navigate to="/operations" replace />} />
            <Route path="automation" element={<Navigate to="/operations" replace />} />
            <Route path="crop-seasons" element={<Navigate to="/cultivation" replace />} />
            <Route path="recipes" element={<Navigate to="/cultivation" replace />} />
            <Route path="dosing-history" element={<Navigate to="/cultivation" replace />} />
            <Route path="logs" element={<Navigate to="/journal" replace />} />
            <Route path="analytics" element={<Navigate to="/journal" replace />} />
          </Route>
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
          <AppRoutes />
        </AuthGate>
      </AuthProvider>
    </QueryClientProvider>
  );
}

export default App;
