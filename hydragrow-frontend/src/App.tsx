// App.tsx
import React, { Suspense } from 'react';
import { BrowserRouter as Router, Routes, Route, Navigate } from 'react-router-dom';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import MainLayout from './components/layout/MainLayout';
import { Toaster } from 'react-hot-toast';
import { LoadingState } from './components/ui/LoadingState';
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

function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <Router>
        <Toaster position="top-center" />
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
            </Route>
          </Routes>
        </Suspense>
      </Router>
    </QueryClientProvider>
  );
}

export default App;
