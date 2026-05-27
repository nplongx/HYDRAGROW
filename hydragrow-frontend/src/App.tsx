import { BrowserRouter as Router, Routes, Route, Navigate } from 'react-router-dom';
import MainLayout from './components/layout/MainLayout';
import Dashboard from './pages/Dashboard';
import ControlPanel from './pages/ControlPanel';
import Analytics from './pages/Analytics';
import Settings from './pages/Settings';
import DosingHistory from './pages/DosingHistory';
import { DeviceProvider } from './context/DeviceContext';
import { Toaster } from 'react-hot-toast';
import './App.css';
import { CropSeasons } from './pages/CropSeasons';
import SystemLog from './pages/SystemLog';

function App() {
  return (
    <DeviceProvider>
      <Router>
        <Toaster
          position="top-center"
          toastOptions={{
            style: {
              background: '#ffffff',
              color: '#14532d',
              borderRadius: '14px',
              border: '1px solid #bbf7d0',
              boxShadow: '0 16px 40px rgba(20, 83, 45, 0.12)',
            }
          }}
        />

        <Routes>
          <Route path="/" element={<MainLayout />}>
            <Route index element={<Navigate to="/dashboard" replace />} />
            <Route path="dashboard" element={<Dashboard />} />
            <Route path="control" element={<ControlPanel />} />
            <Route path="analytics" element={<Analytics />} />
            <Route path="dosing-history" element={<DosingHistory />} />
            <Route path="crop-seasons" element={<CropSeasons />} />
            <Route path="settings" element={<Settings />} />
            <Route path="logs" element={<SystemLog />} />
          </Route>
        </Routes>
      </Router>
    </DeviceProvider>
  );
}

export default App;
