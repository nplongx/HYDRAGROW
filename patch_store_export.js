const fs = require('fs');
const file = 'hydragrow-frontend/src/store/useDeviceStore.ts';
let data = fs.readFileSync(file, 'utf8');
data = data.replace('export interface ControllerHealth', 'export interface ControllerHealth');
fs.writeFileSync(file, data);
