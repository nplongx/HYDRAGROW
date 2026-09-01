import { useState } from 'react';
import { TabShell } from '../components/ui/TabShell';
import ControlPanel from './ControlPanel';
import Automation from './Automation';
import { useFlowCanvas } from '../hooks/useFlowCanvas';
import { useAutomationScripts } from '../hooks/useAutomationScripts';
import { useDeviceStore } from '../store/useDeviceStore';

const Operations = () => {
  const [activeTabId, setActiveTabId] = useState('control');
  const deviceId = useDeviceStore((s) => s.settings?.device_id ?? '');
  const { data: scripts } = useAutomationScripts(deviceId);
  const flow = useFlowCanvas(scripts);

  return (
    <TabShell
      title="Vận hành"
      subtitle="Bật/tắt thủ công hoặc chuyển sang tự động theo lịch."
      defaultTabId="control"
      onTabChange={setActiveTabId}
      action={
        activeTabId === 'automation' ? (
          <button className="ui-btn-primary" onClick={flow.openNewFlow}>
            <span aria-hidden="true">＋</span> Flow mới
          </button>
        ) : undefined
      }
      tabs={[
        { id: 'control', label: 'Điều khiển', content: <ControlPanel variant="embedded" /> },
        { id: 'automation', label: 'Tự động hoá', content: <Automation variant="embedded" flow={flow} scripts={scripts} /> },
      ]}
    />
  );
};

export default Operations;
