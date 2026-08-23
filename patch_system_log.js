const fs = require('fs');
const file = 'hydragrow-backend/src/mqtt/handlers/system_log.rs';
let data = fs.readFileSync(file, 'utf8');
data = data.replace(/metadata: Some\(serde_json::to_value\(\&log_data\.event\)\.unwrap\(\)\)/g, 'metadata: serde_json::to_value(&log_data.event).ok()');
fs.writeFileSync(file, data);
