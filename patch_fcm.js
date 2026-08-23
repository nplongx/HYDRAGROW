const fs = require('fs');
const file = 'hydragrow-backend/src/services/fcm.rs';
let data = fs.readFileSync(file, 'utf8');
data = data.replace('Ok(token.token().unwrap().to_string())', 'Ok(token.token().ok_or("Failed to get FCM token")?.to_string())');
fs.writeFileSync(file, data);
