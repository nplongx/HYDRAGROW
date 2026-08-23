const fs = require('fs');
const file = 'hydragrow-backend/src/api/middleware/auth.rs';
let data = fs.readFileSync(file, 'utf8');
data = data.replace('let app_state = req\n            .app_data::<actix_web::web::Data<AppState>>()\n            .unwrap()\n            .clone();', 'let app_state = match req.app_data::<actix_web::web::Data<AppState>>() {\n            Some(state) => state.clone(),\n            None => return Box::pin(ready(Err(ErrorUnauthorized("Missing app state")))),\n        };');
fs.writeFileSync(file, data);
