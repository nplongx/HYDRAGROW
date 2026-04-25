import { BrowserRouter as Router, Routes, Route, Navigate } from 'react-router-dom';
import MainLayout from './components/layout/MainLayout';
import Dashboard from './pages/Dashboard';
import ControlPanel from './pages/ControlPanel';
import Analytics from './pages/Analytics';
import Settings from './pages/Settings';
import BlockchainHistory from './pages/BlockchainHistory';
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
              background: '#1e293b',
              color: '#fff',
              borderRadius: '16px',
              border: '1px solid #334155',
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
          </Route>
        </Routes>
      </Router>
    </DeviceProvider>
  );
}

export default App;
