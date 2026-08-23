const fs = require('fs');
const file = 'hydragrow-frontend/src/store/useDeviceStore.test.ts';
let data = fs.readFileSync(file, 'utf8');
data = data.replace("{ systemEvents: [{ id: 1, message: 'Event 1' }] }", "{ systemEvents: [{ timestamp_ms: 1, message: 'Event 1' } as unknown as UnifiedSystemLog] }");
data = data.replace("{ id: 1, message: 'Event 1' },\n      { id: 2, message: 'Event 2' }", "{ timestamp_ms: 1, message: 'Event 1' },\n      { timestamp_ms: 2, message: 'Event 2' }");
data = data.replace("import { ControllerHealth } from './useDeviceStore';\n", "");
fs.writeFileSync(file, data);
