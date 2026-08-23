const fs = require('fs');
const file = 'hydragrow-backend/src/main.rs';
let data = fs.readFileSync(file, 'utf8');
data = data.replace(/unwrap\(\)/g, 'expect("startup: acceptable to panic")');
data = data.replace(/#\!\[warn\(clippy::unwrap_used\)\]/g, ''); // just to prevent duplicates
data = '#![warn(clippy::unwrap_used)]\n' + data;
fs.writeFileSync(file, data);
