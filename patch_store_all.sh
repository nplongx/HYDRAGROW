const fileModels = 'hydragrow-frontend/src/types/models.ts';
let dataModels = require('fs').readFileSync(fileModels, 'utf8');
dataModels = dataModels.replace('export interface AppSettings {', 'export interface UnifiedSystemLog {\n  device_id: string;\n  level: string;\n  category: string;\n  title: string;\n  event: unknown;\n  timestamp_ms: number;\n}\n\nexport interface AppSettings {');
dataModels = dataModels.replace('export interface AppSettings {\n  backend_url: string;', 'export interface AppSettings {\n  control_mode?: string;\n  backend_url: string;');
require('fs').writeFileSync(fileModels, dataModels);

const file = 'hydragrow-frontend/src/store/useDeviceStore.ts';
let data = require('fs').readFileSync(file, 'utf8');
data = data.replace("import { SensorData, StatusPayload, AppSettings, TankAlert } from '../types/models';", "import { SensorData, StatusPayload, AppSettings, TankAlert, UnifiedSystemLog } from '../types/models';");
data = data.replace('interface DeviceState {', 'export interface ControllerHealth {\n  device_id: string;\n  free_heap: number;\n  uptime_sec: number;\n  rssi: number;\n  health_score_percent: number;\n  fsm_state_display: string;\n  log_drop_count: number;\n  firmware_version: string;\n  diagnostics?: unknown;\n}\n\ninterface DeviceState {');
data = data.replace('  controllerHealth: any;', '  controllerHealth: ControllerHealth | null;');
data = data.replace('  systemEvents: any[];', '  systemEvents: UnifiedSystemLog[];');
data = data.replace('  setControllerHealth: (health: any) => void;', '  setControllerHealth: (health: ControllerHealth | null) => void;');
data = data.replace('  setSystemEvents: (events: any[] | ((prev: any[]) => any[])) => void;', '  setSystemEvents: (events: UnifiedSystemLog[] | ((prev: UnifiedSystemLog[]) => UnifiedSystemLog[])) => void;');
require('fs').writeFileSync(file, data);

const fileTest = 'hydragrow-frontend/src/store/useDeviceStore.test.ts';
let dataTest = require('fs').readFileSync(fileTest, 'utf8');
dataTest = dataTest.replace("import { useDeviceStore } from './useDeviceStore';", "import { useDeviceStore, ControllerHealth } from './useDeviceStore';\nimport { UnifiedSystemLog } from '../types/models';");
dataTest = dataTest.replace("{ systemEvents: [{ id: 1, message: 'Event 1' }] }", "{ systemEvents: [{ timestamp_ms: 1, title: 'Event 1' } as unknown as UnifiedSystemLog] }");
dataTest = dataTest.replace("setSystemEvents([{ id: 1, message: 'test' }]);", "setSystemEvents([{ timestamp_ms: 1, title: 'test' } as unknown as UnifiedSystemLog]);");
dataTest = dataTest.replace("setSystemEvents((prev: any)", "setSystemEvents((prev: UnifiedSystemLog[])");
dataTest = dataTest.replace("...prev, { id: 2, message: 'Event 2' }", "...prev, { timestamp_ms: 2, message: 'Event 2' } as unknown as UnifiedSystemLog");
dataTest = dataTest.replace("setControllerHealth({ status: 'ok' });", "setControllerHealth({ firmware_version: 'ok' } as unknown as ControllerHealth);");
dataTest = dataTest.replace("expect(useDeviceStore.getState().systemEvents).toEqual([\n      { id: 1, message: 'Event 1' },\n      { id: 2, message: 'Event 2' }\n    ]);", "expect(useDeviceStore.getState().systemEvents).toEqual([\n      { timestamp_ms: 1, title: 'Event 1' } as unknown as UnifiedSystemLog,\n      { timestamp_ms: 2, message: 'Event 2' } as unknown as UnifiedSystemLog\n    ]);");
require('fs').writeFileSync(fileTest, dataTest);
