import ControlPanel from './ControlPanel';
import { EmergencyStopButton } from '../components/safety/EmergencyStopButton';
import { useDeviceStore } from '../store/useDeviceStore';
import { NavLink } from 'react-router-dom';

const SURFACES = [
  { path: '/operations', label: 'Điều khiển' },
  { path: '/automation', label: 'Tự động hóa' },
] as const;

export function Operations() {
  const deviceId = useDeviceStore((s) => s.deviceId);

  return (
    <div className="app-page h-[calc(100vh-4rem)] flex flex-col">
      <nav aria-label="Khu vực vận hành" className="ui-tabbar px-2">
        {SURFACES.map((surface) => (
          <NavLink
            key={surface.path}
            to={surface.path}
            className={({ isActive }) => `ui-tab px-4 py-2 text-sm ${isActive ? 'ui-tab-active' : ''}`}
          >
            {surface.label}
          </NavLink>
        ))}
      </nav>
      <div className="flex-1 overflow-hidden pb-20 lg:pb-0">
        <ControlPanel variant="embedded" />
      </div>
      <EmergencyStopButton deviceId={deviceId} variant="bar" />
    </div>
  );
}
