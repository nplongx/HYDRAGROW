#include "CaptivePortal.h"
#include <WiFi.h>
#include <WiFiServer.h>
#include <WiFiClient.h>
#include "../utils/Logger.h"

static const char HTML_PAGE[] PROGMEM = R"rawlit(
<!DOCTYPE html><html><head>
<meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>HydraGrow WiFi Setup</title>
<style>body{font-family:sans-serif;max-width:400px;margin:40px auto;padding:16px;}
input,button{width:100%;padding:10px;margin:8px 0;box-sizing:border-box;font-size:16px;}
button{background:#2563eb;color:#fff;border:none;border-radius:6px;cursor:pointer;}
</style></head><body>
<h2>HydraGrow — Thiết Lập WiFi</h2>
<form method="POST" action="/save">
  <label>SSID</label>
  <input type="text" name="ssid" placeholder="Tên WiFi" required>
  <label>Mật khẩu</label>
  <input type="password" name="pass" placeholder="Mật khẩu WiFi">
  <label>Ưu tiên (0 = cao nhất)</label>
  <input type="number" name="priority" value="0" min="0" max="255">
  <button type="submit">Lưu & Kết Nối</button>
</form>
</body></html>
)rawlit";

static String urlDecode(const String& encoded) {
    String decoded;
    decoded.reserve(encoded.length());
    for (size_t i = 0; i < encoded.length(); i++) {
        if (encoded[i] == '+') {
            decoded += ' ';
        } else if (encoded[i] == '%' && i + 2 < encoded.length()) {
            char hex[3] = {encoded[i+1], encoded[i+2], 0};
            decoded += (char)strtol(hex, nullptr, 16);
            i += 2;
        } else {
            decoded += encoded[i];
        }
    }
    return decoded;
}

static String extractField(const String& body, const String& field) {
    int start = body.indexOf(field + "=");
    if (start < 0) return "";
    start += field.length() + 1;
    int end = body.indexOf('&', start);
    return urlDecode(end < 0 ? body.substring(start) : body.substring(start, end));
}

// Khởi động AP, trả về false nếu thất bại
static bool startAP() {
    // Disconnect sạch trước khi đổi mode
    WiFi.disconnect(true);   // ngắt STA, xóa credentials tạm
    WiFi.mode(WIFI_OFF);     // tắt hẳn
    delay(200);              // chờ radio reset

    WiFi.mode(WIFI_AP);
    delay(100);

    bool ok = WiFi.softAP("HydraGrow-Setup", "hydragrow");
    delay(500);  // chờ AP broadcast ổn định (quan trọng!)

    if (!ok) {
        Logger::debugPrintln("[PORTAL] softAP() failed!");
    } else {
        Logger::debugPrintf("[PORTAL] AP started. IP: %s\n",
                            WiFi.softAPIP().toString().c_str());
        Logger::debugPrintf("[PORTAL] softAPIP check: %s\n",
                    WiFi.softAPIP().toString().c_str());
    }
    return ok;
}

bool runCaptivePortal(WifiProvisioner& provisioner, unsigned long timeoutMs) {
    if (!startAP()) return false;

    WiFiServer server(80);
    server.begin();

    unsigned long start = millis();
    bool success = false;

    while (!success) {
        if (timeoutMs > 0 && (millis() - start) > timeoutMs) {
            Logger::debugPrintln("[PORTAL] Timeout, aborting portal.");
            break;
        }

        WiFiClient client = server.available();
        if (!client) { delay(10); continue; }

        // ---- Đọc HTTP request đúng cách (tránh block) ----
        unsigned long clientStart = millis();
        String requestLine = "";
        String body = "";

        // Đọc headers
        while (client.connected() && (millis() - clientStart) < 3000) {
            if (!client.available()) { delay(1); continue; }
            String line = client.readStringUntil('\n');
            if (requestLine.isEmpty()) requestLine = line; // Dòng đầu: "POST /save HTTP/1.1"
            if (line == "\r" || line == "") break;         // Hết headers
        }

        bool isPost = requestLine.startsWith("POST");

        // Đọc body nếu là POST
        if (isPost) {
            delay(10); // Chờ data vào buffer
            while (client.available()) body += (char)client.read();
        }
        // ---- Hết đọc request ----

        if (isPost && body.indexOf("ssid=") >= 0) {
            String ssid  = extractField(body, "ssid");
            String pass  = extractField(body, "pass");
            uint8_t prio = (uint8_t)extractField(body, "priority").toInt();

            if (ssid.length() > 0) {
                Logger::debugPrintf("[PORTAL] Thu ket noi: %s\n", ssid.c_str());

                // Thử kết nối trước khi lưu
                WiFi.mode(WIFI_AP_STA);
                WiFi.begin(ssid.c_str(), pass.c_str());

                unsigned long t = millis();
                while (WiFi.status() != WL_CONNECTED && millis() - t < 15000) delay(250);

                if (WiFi.status() == WL_CONNECTED) {
                    Logger::debugPrintf("[PORTAL] Ket noi thanh cong! IP: %s\n",
                                        WiFi.localIP().toString().c_str());

                    std::vector<WifiCandidate> list = provisioner.load();
                    list.push_back({ssid, pass, prio});
                    provisioner.save(list);

                    client.println("HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\n");
                    client.println("Connection: close\r\n");
                    client.println("<h2>Ket noi thanh cong! Thiet bi se khoi dong lai.</h2>");
                    client.stop();
                    success = true;
                    break;
                } else {
                    Logger::debugPrintln("[PORTAL] Ket noi that bai, quay lai AP mode.");

                    // Restart AP sau khi thử kết nối thất bại
                    WiFi.disconnect(true);
                    delay(500);
                    if (!startAP()) {
                        Logger::debugPrintln("[PORTAL] Khong the restart AP!");
                        client.stop();
                        break;
                    }
                    server.begin(); // Restart server sau khi đổi mode

                    client.println("HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\n");
                    client.println("Connection: close\r\n");
                    client.println("<h2>Khong the ket noi. Vui long thu lai.</h2><a href='/'>Quay lai</a>");
                    client.stop();
                    continue;
                }
            }
        }

        // Serve HTML form (GET hoặc body không hợp lệ)
        client.println("HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\n");
        client.println("Connection: close\r\n");
        client.println(FPSTR(HTML_PAGE));
        client.stop();
    }

    server.end();
    WiFi.softAPdisconnect(true);
    return success;
}