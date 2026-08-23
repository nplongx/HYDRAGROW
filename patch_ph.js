const fs = require('fs');
const file = 'hydragrow-backend/src/services/ph_calibration.rs';
let data = fs.readFileSync(file, 'utf8');
data = data.replace('let result = calibrate_ph(CalibrationMode::ThreePoint, &samples).unwrap();', 'let result = calibrate_ph(CalibrationMode::ThreePoint, &samples).unwrap();'); // Already in test
fs.writeFileSync(file, data);
