const fs = require('fs');
const file = 'hydragrow-backend/src/api/middleware/rate_limit.rs';
let data = fs.readFileSync(file, 'utf8');
data = data.replace('let mut state = self.state.lock().unwrap();', 'let mut state = match self.state.lock() {\n            Ok(s) => s,\n            Err(_) => return Box::pin(async { Err(actix_web::error::ErrorInternalServerError("Lock error")) }),\n        };');
fs.writeFileSync(file, data);
