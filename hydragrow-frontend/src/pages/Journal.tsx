import { TabShell } from '../components/ui/TabShell';
import { useSearchParams } from 'react-router-dom';
import { parseTab, serializeTab } from '../lib/routeState';
import SystemLog from './SystemLog';
import Analytics from './Analytics';

const Journal = () => {
  const [searchParams, setSearchParams] = useSearchParams();
  const activeTabId = parseTab(searchParams.toString(), ['events', 'analytics'], 'events');
  return (
    <TabShell
      title="Nhật ký"
      subtitle="Sự kiện hệ thống, thiết bị và cảnh báo, cùng các chỉ số phân tích chính."
      defaultTabId="events"
      activeTabId={activeTabId}
      onTabChange={(tabId) => setSearchParams(serializeTab(searchParams.toString(), tabId, 'events'), { replace: true })}
      tabs={[
        { id: 'events', label: 'Sự kiện', content: <SystemLog variant="embedded" /> },
        { id: 'analytics', label: 'Phân tích', content: <Analytics variant="embedded" /> },
      ]}
    />
  );
};

export default Journal;
