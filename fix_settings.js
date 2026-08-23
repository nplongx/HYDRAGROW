const fs = require('fs');
let file = 'hydragrow-frontend/src/pages/Settings.tsx';
let content = fs.readFileSync(file, 'utf8');

content = content.replace("toast.error((error as Error).message)('Không gửi được lệnh cập nhật firmware.');", "toast.error('Không gửi được lệnh cập nhật firmware.');");
content = content.replace("toast.error((error as Error).message)('Không gửi được danh sách WiFi.');", "toast.error('Không gửi được danh sách WiFi.');");
content = content.replace("toast.error((error as Error).message)(`Không thể đo pH ${activePoint}.`);", "toast.error(`Không thể đo pH ${activePoint}.`);");

fs.writeFileSync(file, content);
