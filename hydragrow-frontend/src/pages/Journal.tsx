import { TabShell } from '../components/ui/TabShell';
import SystemLog from './SystemLog';
import Analytics from './Analytics';

const Journal = () => (
  <TabShell
    title="Nhật ký"
    subtitle="Sự kiện hệ thống, thiết bị và cảnh báo, cùng các chỉ số phân tích chính."
    defaultTabId="events"
    tabs={[
      { id: 'events', label: 'Sự kiện', content: <SystemLog variant="embedded" /> },
      { id: 'analytics', label: 'Phân tích', content: <Analytics variant="embedded" /> },
    ]}
  />
);

export default Journal;
