const fs = require('fs');
const file = 'hydragrow-frontend/src/pages/Dashboard.tsx';
let data = fs.readFileSync(file, 'utf8');
data = data.replace('const rawScore = controllerHealth?.health_score_percent ?? controllerHealth?.diagnostics?.health_score_percent;', 'const rawScore = controllerHealth?.health_score_percent ?? (controllerHealth?.diagnostics as any)?.health_score_percent;');
fs.writeFileSync(file, data);
