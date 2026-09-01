// App.tsx
import React, { Suspense } from 'react';
import { BrowserRouter as Router, Routes, Route, Navigate } from 'react-router-dom';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import MainLayout from './components/layout/MainLayout';
import { Toaster } from 'react-hot-toast';
import { LoadingState } from './components/ui/LoadingState';
import { AuthProvider, useAuth } from './contexts/AuthContext';
import { LoginScreen } from './components/auth/LoginScreen';
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
import Operations from './pages/Operations';
import Cultivation from './pages/Cultivation';
import Journal from './pages/Journal';
const Settings = React.lazy(() => import('./pages/Settings'));
import { DevicePairing } from './pages/DevicePairing';

function AuthGate({ children }: { children: React.ReactNode }) {
  const { status } = useAuth();

  if (status === 'loading') {
    return <LoadingState message="Đang kiểm tra đăng nhập..." />;
  }

  if (status === 'unauthenticated') {
    return <LoginScreen />;
  }

  return <>{children}</>;
}

function AppRoutes() {
  return (
    <Router>
      <Toaster position="top-center" />
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
