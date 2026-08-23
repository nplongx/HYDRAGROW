const fs = require('fs');
const file = 'hydragrow-frontend/src/types/models.ts';
let data = fs.readFileSync(file, 'utf8');
data = data.replace('export interface AppSettings {', 'export interface UnifiedSystemLog {\n  device_id: string;\n  level: string;\n  category: string;\n  title: string;\n  event: unknown;\n  timestamp_ms: number;\n}\n\nexport interface AppSettings {');
data = data.replace('export interface AppSettings {\n  backend_url: string;', 'export interface AppSettings {\n  control_mode?: string;\n  backend_url: string;');
data = data.replace('  [key: string]: any;', '  [key: string]: unknown;');
fs.writeFileSync(file, data);
