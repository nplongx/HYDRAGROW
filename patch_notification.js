const fs = require('fs');
const file = 'hydragrow-backend/src/api/notification.rs';
let data = fs.readFileSync(file, 'utf8');
data = data.replace('let mut tokens = state.fcm_tokens.lock().unwrap();', 'let mut tokens = match state.fcm_tokens.lock() {\n        Ok(t) => t,\n        Err(_) => return HttpResponse::InternalServerError().body("Lock error"),\n    };');
fs.writeFileSync(file, data);
