import { BrowserRouter as Router, Routes, Route, Navigate } from 'react-router-dom';
import MainLayout from './components/layout/MainLayout';
import Dashboard from './pages/Dashboard';
import ControlPanel from './pages/ControlPanel';
import Analytics from './pages/Analytics';
import Settings from './pages/Settings';
import BlockchainHistory from './pages/BlockchainHistory';
import { DeviceProvider } from './context/DeviceContext';
// 🟢 1. IMPORT TOASTER TỪ REACT-HOT-TOAST
import { Toaster } from 'react-hot-toast';
import './App.css';
import { CropSeasons } from './pages/CropSeasons';
import SystemLog from './pages/SystemLog';
import SystemFlow from './pages/SystemFlow';

function App() {
  return (
    <DeviceProvider>
      <Router>
        {/* 🟢 2. ĐẶT TOASTER Ở ĐÂY ĐỂ NÓ HOẠT ĐỘNG TOÀN CỤC */}
        <Toaster
          position="top-center"
          toastOptions={{
            style: {
              background: '#1e293b', // bg-slate-800
              color: '#fff',
              borderRadius: '16px',
              border: '1px solid #334155', // border-slate-700
            }
          }}
        />

        <Routes>
          <Route path="/" element={<MainLayout />}>
            <Route index element={<Navigate to="/dashboard" replace />} />
            <Route path="dashboard" element={<Dashboard />} />
            <Route path="control" element={<ControlPanel />} />
            <Route path="analytics" element={<Analytics />} />
            <Route path="blockchain" element={<BlockchainHistory />} />
            <Route path="/crop-seasons" element={<CropSeasons />} />
            <Route path="settings" element={<Settings />} />
            <Route path="/logs" element={<SystemLog />} />
            <Route path="/system-flow" element={<SystemFlow />} />
          </Route>
        </Routes>
      </Router>
    </DeviceProvider>
  );
}

export default App;
