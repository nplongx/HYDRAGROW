#include <Arduino.h>
#include <ArduinoJson.h>
#include <DallasTemperature.h>
#include <OneWire.h>
#include <PubSubClient.h>
#include <WiFi.h>

// ================= NETWORK & MQTT CONFIG =================
const char *ssid = "Huynh Hong";
const char *password = "123443215";
const char *mqtt_server = "interchange.proxy.rlwy.net";
const int mqtt_port = 50133;
const char *device_id = "device_001";
const char *mqtt_user = "long";
const char *mqtt_pass = "53zx37kxq3epbexgqt6rjlce1d0e0gwq";

String topic_sensors = String("AGITECH/") + device_id + "/sensors";
String topic_config = String("AGITECH/") + device_id + "/sensors/config";
String topic_cmd = String("AGITECH/") + device_id + "/sensor/command";
String topic_status = String("AGITECH/") + device_id + "/sensor/status";

WiFiClient espClient;
PubSubClient client(espClient);

// ================= PIN MAPS =================
#define PIN_DS18B20 2
#define PIN_TRIG 3
#define PIN_ECHO 4
#define PIN_PH_ADC 0
#define PIN_EC_ADC 1

OneWire oneWire(PIN_DS18B20);
DallasTemperature sensors(&oneWire);

// ================= CALIBRATION & CONFIG =================
#define MAX_WINDOW 50

float ph_v7 = 2650.0, ph_v4 = 3555.0;
float ec_factor = 0.88, ec_offset = 0.0;
float temp_offset = 0.0;

// Các biến cấu hình từ xa
int ma_window = 15;          // Mặc định CHUẨN (lọc trong 3 giây)
int publish_interval = 5000; // Mặc định gửi 5s/lần

float tank_height = 100.0;     // Độ cao bể (cm)
bool continuous_level = false; // Cờ trạng thái đo liên tục (Bơm đang chạy)

#define V_REF_MV 3300.0
#define ADC_MAX 4095.0
#define VOLTAGE_DIVIDER_RATIO 1.5

// 🟢 MỚI: TỐC ĐỘ LẤY MẪU CỨNG
const int SAMPLING_INTERVAL = 200; // Đọc cảm biến liên tục mỗi 200ms

// Bộ đệm Lọc MA
float temp_history[MAX_WINDOW], water_history[MAX_WINDOW];
float ph_history[MAX_WINDOW], ec_history[MAX_WINDOW];
int history_idx = 0;

// Các biến lưu giá trị trung bình toàn cục
float current_avg_temp = 25.0;
float current_avg_water = 20.0;
float current_avg_ph = 7.0;
float current_avg_ec = 0.0;
float latest_raw_water = 20.0; // Lưu riêng giá trị nước thô (tức thời)

// Cờ bật/tắt cảm biến
bool enable_ph = true;
bool enable_ec = true;
bool enable_temp = true;
bool enable_water = true;

// CỜ BÙ NHIỆT
bool enable_ec_tc = true;
bool enable_ph_tc = true;
float temp_compensation_beta = 0.02;

// ================= HÀM TIỆN ÍCH =================

int read_adc_filtered(int pin) {
  int buffer[10];
  for (int i = 0; i < 10; i++) {
    buffer[i] = analogRead(pin);
    delay(5);
  }
  for (int i = 0; i < 9; i++) {
    for (int j = i + 1; j < 10; j++) {
      if (buffer[i] > buffer[j]) {
        int temp = buffer[i];
        buffer[i] = buffer[j];
        buffer[j] = temp;
      }
    }
  }
  long sum = 0;
  for (int i = 2; i < 8; i++)
    sum += buffer[i];
  return sum / 6;
}

float calc_average(float history[], float new_val) {
  history[history_idx] = new_val;
  float sum = 0;
  for (int i = 0; i < ma_window; i++)
    sum += history[i];
  return sum / ma_window;
}

float readWaterLevel() {
  digitalWrite(PIN_TRIG, LOW);
  delayMicroseconds(2);
  digitalWrite(PIN_TRIG, HIGH);
  delayMicroseconds(20);
  digitalWrite(PIN_TRIG, LOW);
  long duration = pulseIn(PIN_ECHO, HIGH, 20000);

  if (duration == 0) {
    Serial.println("⚠️ [DEBUG-Water] Không đọc được xung phản hồi (duration = "
                   "0)"); // [DEBUG]
    return -1;
  }

  float distance = (duration / 2.0) * 0.0343;
  float water_level = tank_height - distance;
  float final_level = (water_level < 0) ? 0 : water_level;

  // [DEBUG] Xem chi tiết cảm biến siêu âm
  // Serial.printf("💧 [DEBUG-Water] Duration: %ld us | Distance: %.2f cm |
  // Level thô: %.2f cm\n", duration, distance, final_level);

  return final_level;
}

float calculate_ph(float voltage_mv, float current_temp) {
  float diff = ph_v4 - ph_v7;
  float slope = (abs(diff) < 0.1) ? -0.006 : ((4.0 - 7.0) / diff);

  if (enable_ph_tc) {
    float temp_ratio = (current_temp + 273.15) / (25.0 + 273.15);
    slope = slope / temp_ratio;
  }
  float ph_result = constrain(7.0 + slope * (voltage_mv - ph_v7), 0.0, 14.0);

  // [DEBUG]
  Serial.printf("🧪 [DEBUG-pH] Volt: %.2f mV | Temp bù: %.2f °C | Slope: %.4f "
                "| pH calc: %.2f\n",
                voltage_mv, current_temp, slope, ph_result);

  return ph_result;
}

float calculate_ec(float voltage_mv, float current_temp) {
  float raw_ec = (voltage_mv / 1000.0) * ec_factor + ec_offset;
  float ec_result = raw_ec;

  if (enable_ec_tc) {
    float coef = 1.0 + temp_compensation_beta * (current_temp - 25.0);
    ec_result = raw_ec / coef;
  }
  ec_result = max(ec_result, 0.0f);

  // [DEBUG]
  // Serial.printf("⚡ [DEBUG-EC] Volt: %.2f mV | Raw EC: %.2f | Temp bù: %.2f
  // °C | EC calc: %.2f\n", voltage_mv, raw_ec, current_temp, ec_result);

  return ec_result;
}

// ================= MQTT CALLBACK =================
void mqttCallback(char *topic, byte *payload, unsigned int length) {
  String message = "";
  for (int i = 0; i < length; i++)
    message += (char)payload[i];

  String topicStr = String(topic);

  // [DEBUG] In ra mọi gói tin nhận được
  Serial.printf("📥 [DEBUG-MQTT] Nhận Topic: %s\nPayload: %s\n",
                topicStr.c_str(), message.c_str());

  // XỬ LÝ LỆNH COMMAND TỪ CONTROLLER
  if (topicStr == topic_cmd) {
    DynamicJsonDocument doc(256);

    if (!deserializeJson(doc, message)) {
      if (doc.containsKey("command") && doc["command"] == "continuous_level") {
        continuous_level = doc["state"].as<bool>();
        Serial.print("🔄 Lệnh Controller -> Chế độ đo liên tục (Bơm): ");
        Serial.println(continuous_level ? "BẬT" : "TẮT");
      }
    } else {
      Serial.println("❌ [DEBUG] Lỗi Parse JSON Command!"); // [DEBUG]
    }
    return;
  }

  // XỬ LÝ CẤU HÌNH SENSOR
  if (topicStr == topic_config) {
    DynamicJsonDocument doc(1024);
    DeserializationError error = deserializeJson(doc, message);

    if (error) {
      // Nếu lỗi sẽ in ra lý do tại đây
      Serial.print("❌ Lỗi Parse JSON Config: ");
      Serial.println(error.c_str());
      return;
    }

    // [DEBUG] Bắt đầu đọc cấu hình mới
    Serial.println("⚙️ [DEBUG] Đang nạp cấu hình mới...");

    if (doc.containsKey("ph_v7"))
      ph_v7 = doc["ph_v7"].as<float>();
    if (doc.containsKey("ph_v4"))
      ph_v4 = doc["ph_v4"].as<float>();

    if (doc.containsKey("ec_factor"))
      ec_factor = doc["ec_factor"].as<float>();
    if (doc.containsKey("ec_offset"))
      ec_offset = doc["ec_offset"].as<float>();
    if (doc.containsKey("temp_offset"))
      temp_offset = doc["temp_offset"].as<float>();

    if (doc.containsKey("tank_height"))
      tank_height = doc["tank_height"].as<float>();
    if (doc.containsKey("temp_compensation_beta"))
      temp_compensation_beta = doc["temp_compensation_beta"].as<float>();

    if (doc.containsKey("moving_average_window"))
      ma_window =
          constrain(doc["moving_average_window"].as<int>(), 1, MAX_WINDOW);

    if (doc.containsKey("publish_interval"))
      publish_interval = doc["publish_interval"].as<int>();

    if (doc.containsKey("enable_ph_sensor"))
      enable_ph = doc["enable_ph_sensor"].as<bool>();
    if (doc.containsKey("enable_ec_sensor"))
      enable_ec = doc["enable_ec_sensor"].as<bool>();
    if (doc.containsKey("enable_temp_sensor"))
      enable_temp = doc["enable_temp_sensor"].as<bool>();
    if (doc.containsKey("enable_water_level_sensor"))
      enable_water = doc["enable_water_level_sensor"].as<bool>();

    Serial.println("🔄 Đã nạp cấu hình Lõi mới từ Server thành công!");
  }
}

// ================= SETUP & TIMERS =================
void setup() {
  Serial.begin(115200);
  delay(1000); // [DEBUG] Đợi Serial Monitor sẵn sàng
  Serial.println("\n\n🚀 Bắt đầu khởi động thiết bị..."); // [DEBUG]

  pinMode(PIN_TRIG, OUTPUT);
  pinMode(PIN_ECHO, INPUT);

  analogReadResolution(12);
  analogSetAttenuation(ADC_11db);

  Serial.println("🌡️ Khởi tạo cảm biến nhiệt độ..."); // [DEBUG]
  sensors.begin();

  for (int i = 0; i < MAX_WINDOW; i++) {
    temp_history[i] = 25.0;
    water_history[i] = 20.0;
    ph_history[i] = 7.0;
    ec_history[i] = 0.0;
  }

  Serial.printf("🌐 Đang kết nối WiFi: %s...\n", ssid); // [DEBUG]
  WiFi.begin(ssid, password);
  while (WiFi.status() != WL_CONNECTED) { // [DEBUG] Chờ kết nối WiFi
    delay(500);
    Serial.print(".");
  }
  Serial.println("\n✅ Kết nối WiFi thành công!"); // [DEBUG]
  Serial.print("📡 IP Address: ");                 // [DEBUG]
  Serial.println(WiFi.localIP());                  // [DEBUG]

  client.setBufferSize(1024);
  client.setServer(mqtt_server, mqtt_port);
  client.setCallback(mqttCallback);
}

void reconnect() {
  while (!client.connected()) {
    Serial.print("Đang kết nối MQTT...");
    String clientId = "SensorNode_" + String(device_id);

    // Cấu hình thông số LWT (Sẽ tự động gửi khi ESP32 mất kết nối đột ngột)
    const char *willTopic = topic_status.c_str();
    const char *willMessage = "{\"online\": false}";
    int willQos = 1;
    boolean willRetain = true; // Giữ lại bản tin cuối cùng trên Broker

    // Sử dụng hàm connect có hỗ trợ LWT
    if (client.connect(clientId.c_str(), mqtt_user, mqtt_pass, willTopic,
                       willQos, willRetain, willMessage)) {
      Serial.println("Thành công!");

      // Báo cáo trạng thái Online ngay khi kết nối thành công
      client.publish(willTopic, "{\"online\": true}",
                     true); // Tham số true ở cuối là để Retain bản tin

      // Đăng ký nhận bản tin
      client.subscribe(topic_config.c_str());
      client.subscribe(topic_cmd.c_str());
    } else {
      Serial.print("Lỗi, rc=");
      Serial.print(client.state());
      Serial.println(" -> Thử lại sau 5 giây");
      delay(5000);
    }
  }
}

// Khai báo 2 bộ đếm thời gian độc lập
unsigned long last_sample_time = 0;
unsigned long last_publish_time = 0;

void loop() {
  if (WiFi.status() != WL_CONNECTED) {
    Serial.println("⚠️ [DEBUG] Mất kết nối WiFi, đang thử lại...");
    WiFi.disconnect();
    WiFi.reconnect();
    delay(5000);
    return;
  }
  if (!client.connected())
    reconnect();
  client.loop();

  unsigned long current_millis = millis();

  // Các biến cờ lỗi cục bộ trong vòng lặp này
  static bool err_water_flag = false;
  static bool err_temp_flag = false;
  static bool err_ph_flag = false;
  static bool err_ec_flag = false;

  // ==========================================
  // LUỒNG 1: LẤY MẪU VÀ LỌC NHIỄU (Mỗi 200ms)
  // ==========================================
  if (current_millis - last_sample_time >= SAMPLING_INTERVAL) {
    last_sample_time = current_millis;

    // 1. Nhiệt độ
    float raw_temp = current_avg_temp;
    err_temp_flag = false; // Reset cờ lỗi
    if (enable_temp) {
      sensors.requestTemperatures();
      float t = sensors.getTempCByIndex(0);
      // DEVICE_DISCONNECTED_C = -127.00
      if (t > -50.0 && t <= 80.0) {
        raw_temp = t + temp_offset;
      } else {
        err_temp_flag = true;
        Serial.printf("⚠️ [DEBUG] Lỗi hoặc đứt cảm biến nhiệt độ: %.2f\n", t);
      }
    }
    current_avg_temp = calc_average(temp_history, raw_temp);

    // 2. Mực nước
    err_water_flag = false; // Reset cờ
    if (enable_water) {
      float w = readWaterLevel();
      if (w >= 0) {
        latest_raw_water = w;
        current_avg_water = calc_average(water_history, w);
      } else {
        err_water_flag = true;
      }
    }

    // 3. pH
    err_ph_flag = false; // Reset cờ
    if (enable_ph) {
      int adc_ph = read_adc_filtered(PIN_PH_ADC);
      if (adc_ph <= 0 || adc_ph >= 4095) {
        err_ph_flag = true; // Lỗi đứt dây tín hiệu hoặc chạm chập
        Serial.println("⚠️ [DEBUG] Lỗi cảm biến pH: ADC rớt ngưỡng an toàn.");
      } else {
        float ph_mv = (adc_ph / ADC_MAX) * V_REF_MV * VOLTAGE_DIVIDER_RATIO;
        float ph_val = calculate_ph(ph_mv, current_avg_temp);
        current_avg_ph = calc_average(ph_history, ph_val);
      }
    }

    // 4. EC
    err_ec_flag = false; // Reset cờ
    if (enable_ec) {
      int adc_ec = read_adc_filtered(PIN_EC_ADC);
      if (adc_ec <= 0 || adc_ec >= 4095) {
        err_ec_flag = true;
        Serial.println("⚠️ [DEBUG] Lỗi cảm biến EC: ADC rớt ngưỡng an toàn.");
      } else {
        float ec_mv = (adc_ec / ADC_MAX) * V_REF_MV * VOLTAGE_DIVIDER_RATIO;
        float ec_val = calculate_ec(ec_mv, current_avg_temp);
        current_avg_ec = calc_average(ec_history, ec_val);
      }
    }

    // Tăng index đệm MA (Chỉ tăng 1 lần sau khi đã nạp đủ 4 mảng)
    history_idx = (history_idx + 1) % ma_window;
  }

  // ==========================================
  // LUỒNG 2: GỬI DỮ LIỆU LÊN SERVER (Publish)
  // ==========================================
  int current_pub_interval = continuous_level ? 500 : publish_interval;

  if (current_millis - last_publish_time >= current_pub_interval) {
    last_publish_time = current_millis;

    DynamicJsonDocument doc(512);

    // 1. Dữ liệu cảm biến cốt lõi (Gửi giá trị thô thay vì avg nếu bị lỗi nước
    // liên tục)
    doc["temp"] = current_avg_temp;
    doc["water_level"] =
        continuous_level ? latest_raw_water : current_avg_water;
    doc["ph"] = current_avg_ph;
    doc["ec"] = current_avg_ec;

    // 2. Bổ sung Sức khỏe thiết bị (Device Health)
    doc["rssi"] = WiFi.RSSI();
    doc["free_heap"] = ESP.getFreeHeap();
    doc["uptime"] = millis() / 1000;

    doc["is_continuous"] = continuous_level;

    // 3. Cờ báo lỗi của TOÀN BỘ CẢM BIẾN
    doc["err_water"] = err_water_flag;
    doc["err_temp"] = err_temp_flag;
    doc["err_ph"] = err_ph_flag;
    doc["err_ec"] = err_ec_flag;

    String payload;
    serializeJson(doc, payload);
    client.publish(topic_sensors.c_str(), payload.c_str());

    Serial.println("📡 Đã gửi: " + payload);
    Serial.println("-----------------------------------");
  }
}
