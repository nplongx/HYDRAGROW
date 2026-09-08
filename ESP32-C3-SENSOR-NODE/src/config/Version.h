#pragma once

// Phiên bản firmware hiện tại của ESP32-C3-SENSOR-NODE, dùng để so sánh với
// tag_name của GitHub Release mới nhất (xem OtaVersionCheck::isUpdateAvailable,
// OtaUpdater::performUpdate). PHẢI khớp với git tag khi cắt release —
// workflow firmware-release.yml (Task 9) sẽ chặn release nếu lệch.
#define FIRMWARE_VERSION "v0.1.0"