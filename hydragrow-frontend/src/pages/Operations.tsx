import { useState } from 'react';
import ControlPanel from './ControlPanel';
import { Automation } from './Automation';

const TABS = [
  { id: 'control', label: 'Điều khiển' },
  { id: 'automation', label: 'Tự động hóa' },
] as const;

export function Operations() {
  const [active, setActive] = useState<(typeof TABS)[number]['id']>('control');

  return (
    <div className="app-page h-[calc(100vh-4rem)] flex flex-col">
      <div role="tablist" className="flex gap-2 border-b border-gray-200 px-2">
        {TABS.map((tab) => (
          <button
            key={tab.id}
            role="tab"
            aria-selected={active === tab.id}
            className={`px-4 py-2 text-sm font-semibold ${
              active === tab.id ? 'border-b-2 border-emerald-700 text-emerald-900' : 'text-gray-500'
            }`}
            onClick={() => setActive(tab.id)}
          >
            {tab.label}
          </button>
        ))}
      </div>
      <div className="flex-1 overflow-hidden">
        {active === 'control' ? <ControlPanel variant="embedded" /> : <Automation />}
      </div>
    </div>
  );
}