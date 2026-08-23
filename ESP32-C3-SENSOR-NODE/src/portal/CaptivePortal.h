#pragma once
#include <Arduino.h>
#include "../wifi/WifiProvisioner.h"

// Mở AP "HydraGrow-Setup" + web server tại 192.168.4.1
// Blocking: chạy cho đến khi user submit WiFi credentials thành công,
// hoặc timeoutMs hết (0 = không timeout).
// Trả về true nếu credentials đã được lưu và WiFi đã kết nối thành công.
bool runCaptivePortal(WifiProvisioner& provisioner,
                      unsigned long timeoutMs = 0);
