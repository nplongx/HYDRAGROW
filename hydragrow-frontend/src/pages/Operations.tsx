import { useSearchParams } from 'react-router-dom';
import ControlPanel from './ControlPanel';
import { Automation } from './Automation';
import { EmergencyStopButton } from '../components/safety/EmergencyStopButton';
import { useStationContext } from '../contexts/StationContext';
import { parseTab, serializeTab } from '../lib/routeState';

const TABS = [
  { id: 'control', label: 'Điều khiển' },
  { id: 'automation', label: 'Tự động hóa' },
] as const;

export function Operations() {
  const [searchParams, setSearchParams] = useSearchParams();
  const active = parseTab(searchParams.toString(), TABS.map((tab) => tab.id), 'control') as (typeof TABS)[number]['id'];
  const { selectedDeviceId: deviceId } = useStationContext();

  return (
    <div className="app-page h-[calc(100vh-4rem)] flex flex-col">
      <div role="tablist" className="ui-tabbar px-2">
        {TABS.map((tab) => (
          <button
            key={tab.id}
            role="tab"
            aria-selected={active === tab.id}
            className={`ui-tab px-4 py-2 text-sm ${active === tab.id ? 'ui-tab-active' : ''}`}
            onClick={() => setSearchParams(serializeTab(searchParams.toString(), tab.id, 'control'), { replace: true })}
          >
            {tab.label}
          </button>
        ))}
      </div>
      <div className="flex-1 overflow-hidden pb-20 lg:pb-0">
        {active === 'control' ? <ControlPanel variant="embedded" /> : <Automation />}
      </div>
      <EmergencyStopButton deviceId={deviceId} variant="bar" />
    </div>
  );
}
