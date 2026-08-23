const fs = require('fs');
let file = 'hydragrow-backend/src/api/middleware/auth.rs';
let content = fs.readFileSync(file, 'utf8');
content = content.replace(/ErrorUnauthorized/g, 'actix_web::error::ErrorUnauthorized');
fs.writeFileSync(file, content);
