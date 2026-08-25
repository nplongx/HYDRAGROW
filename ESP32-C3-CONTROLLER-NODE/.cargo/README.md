# .cargo/config.toml

Các biến `HYDRAGROW_*` trong `[env]` để trống — điền qua `export` trong
shell trước khi build (xem hướng dẫn ở `hydragrow-frontend/README.md`,
mục "ESP32-C3 controller node"), KHÔNG commit giá trị thật vào file này.

File này từng chứa WiFi/MQTT password thật bị commit — đã rotate và
gỡ khỏi đây. Nếu bạn đang dùng lại các giá trị cũ (SSID "Huynh Hong",
broker.hivemq.com, device_001), coi chúng là đã lộ và đã rotate.
