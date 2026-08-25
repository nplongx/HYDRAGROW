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
const ControlPanel = React.lazy(() => import('./pages/ControlPanel'));
const Analytics = React.lazy(() => import('./pages/Analytics'));
const Settings = React.lazy(() => import('./pages/Settings'));
const DosingHistory = React.lazy(() => import('./pages/DosingHistory'));
const CropSeasons = React.lazy(() => import('./pages/CropSeasons'));
const SystemLog = React.lazy(() => import('./pages/SystemLog'));
const RecipeBuilder = React.lazy(() => import('./pages/RecipeBuilder'));
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
      <Toaster
        position="top-center"
        toastOptions={{
          style: {
            background: '#ffffff',
            color: '#14532d',
            borderRadius: '14px',
            border: '1px solid #d1fae5',
            boxShadow: '0 8px 32px rgba(20, 83, 45, 0.12)',
            fontSize: '13px',
            fontWeight: '600',
          },
          success: {
            iconTheme: { primary: '#16a34a', secondary: '#f0fdf4' },
          },
          error: {
            iconTheme: { primary: '#dc2626', secondary: '#fef2f2' },
          },
        }}
      />
      <Suspense fallback={<LoadingState message="Đang tải trang..." />}>
        <Routes>
          <Route path="/" element={<MainLayout />}>
            <Route index element={<Navigate to="/dashboard" replace />} />
            <Route path="dashboard" element={<Dashboard />} />
            <Route path="control" element={<ControlPanel />} />
            <Route path="analytics" element={<Analytics />} />
            <Route path="dosing-history" element={<DosingHistory />} />
            <Route path="crop-seasons" element={<CropSeasons />} />
            <Route path="recipes" element={<RecipeBuilder />} />
            <Route path="settings" element={<Settings />} />
            <Route path="logs" element={<SystemLog />} />
            <Route path="pairing" element={<DevicePairing />} />
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
