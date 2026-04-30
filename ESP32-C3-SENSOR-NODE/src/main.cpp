#include <Arduino.h>
#include <ArduinoJson.h>
#include <DallasTemperature.h>
#include <OneWire.h>
#include <PubSubClient.h>
#include <WiFi.h>

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

#define PIN_DS18B20 2
#define PIN_TRIG 3
#define PIN_ECHO 5
#define PIN_PH_ADC 0
#define PIN_EC_ADC 1

OneWire oneWire(PIN_DS18B20);
DallasTemperature sensors(&oneWire);

// ==========================================
// Cấu hình vật lý & Cảm biến
// ==========================================
float ph_v686 = 2650.0, ph_v4 = 3555.0, ph_v918 = 1750.0;
String ph_calibration_mode = "2-point";

float ec_factor = 0.88, ec_offset = 0.0;
float temp_offset = 0.0;

int publish_interval = 5000;
float tank_height = 100.0;
bool continuous_level = false;

#define V_REF_MV 3300.0
#define ADC_MAX 4095.0
#define VOLTAGE_DIVIDER_RATIO 1.5
const int SAMPLING_INTERVAL = 200;

float current_avg_temp = 25.0;
float current_avg_water = 20.0;
float current_avg_ph = 6.86;
float current_avg_ec = 0.0;
float latest_ph_voltage_mv = NAN;
float latest_raw_water = 20.0;
float latest_raw_ph =
    6.86; // [THÊM MỚI CẬP NHẬT]: Biến lưu giá trị pH thô chưa qua lọc

bool enable_ph = true;
bool enable_ec = true;
bool enable_temp = true;
bool enable_water = true;

bool enable_ec_tc = true;
bool enable_ph_tc = true;
float temp_compensation_beta = 0.02;

extern unsigned long last_publish_time;

// =====================================================================
// CLASS: BỘ LỌC TÍN HIỆU LAI (HYBRID FILTER) - O(1) Memory Complexity
// =====================================================================
class HybridFilter {
private:
  float X_prev;
  float Y_prev;
  int error_streak;
  float delta_max;
  float alpha;
  bool initialized;

public:
  HybridFilter(float _delta, float _alpha) {
    delta_max = _delta;
    alpha = _alpha;
    error_streak = 0;
    initialized = false;
    X_prev = 0;
    Y_prev = 0;
  }

  void setAlpha(float _alpha) { alpha = _alpha; }
  void setDelta(float _delta) { delta_max = _delta; }

  float update(float x_t) {
    if (!initialized || isnan(x_t)) {
      if (!isnan(x_t)) {
        X_prev = x_t;
        Y_prev = x_t;
        initialized = true;
      }
      return Y_prev;
    }

    float X_t = x_t;

    if (abs(x_t - X_prev) > delta_max) {
      error_streak++;
      Serial.printf("⚠️ [DEBUG-Filter] Phát hiện dị biệt: x_t=%.2f, "
                    "X_prev=%.2f, streak=%d\n",
                    x_t, X_prev, error_streak);

      if (error_streak > 5) {
        X_prev = x_t;
        error_streak = 0;
        Serial.println(
            "🛑 [DEBUG-Filter] Streak > 5: Cập nhật X_prev thành giá trị mới.");
      } else {
        X_t = X_prev;
      }
    } else {
      X_prev = x_t;
      error_streak = 0;
    }

    float Y_t = (alpha * X_t) + ((1.0 - alpha) * Y_prev);
    Y_prev = Y_t;

    return Y_t;
  }
};

HybridFilter tempFilter(5.0, 0.125);
HybridFilter waterFilter(20.0, 0.125);
HybridFilter phFilter(1.5, 0.125);
HybridFilter ecFilter(1.0, 0.125);

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

float readWaterLevel() {
  digitalWrite(PIN_TRIG, LOW);
  delayMicroseconds(2);
  digitalWrite(PIN_TRIG, HIGH);
  delayMicroseconds(20);
  digitalWrite(PIN_TRIG, LOW);
  long duration = pulseIn(PIN_ECHO, HIGH, 20000);

  if (duration == 0) {
    Serial.println("⚠️ [DEBUG-Water] Không đọc được xung phản hồi");
    return -1;
  }
  float distance = (duration / 2.0) * 0.0343;
  float water_level = tank_height - distance;
  return (water_level < 0) ? 0 : water_level;
}

float calculate_ph(float voltage_mv, float current_temp) {
  float slope;
  float base_ph;
  float base_v;

  if (ph_calibration_mode == "3-point") {
    if (voltage_mv > ph_v686) {
      float diff = ph_v4 - ph_v686;
      slope = (abs(diff) < 0.1) ? -0.006 : ((4.0 - 6.86) / diff);
      base_ph = 6.86;
      base_v = ph_v686;
    } else {
      float diff = ph_v686 - ph_v918;
      slope = (abs(diff) < 0.1) ? -0.006 : ((6.86 - 9.18) / diff);
      base_ph = 9.18;
      base_v = ph_v918;
    }
  } else {
    float diff = ph_v4 - ph_v686;
    slope = (abs(diff) < 0.1) ? -0.006 : ((4.0 - 6.86) / diff);
    base_ph = 6.86;
    base_v = ph_v686;
  }

  if (enable_ph_tc) {
    float temp_ratio = (current_temp + 273.15) / (25.0 + 273.15);
    slope = slope / temp_ratio;
  }

  float ph_result =
      constrain(base_ph + slope * (voltage_mv - base_v), 0.0, 14.0);
  return ph_result;
}

float calculate_ec(float voltage_mv, float current_temp) {
  float raw_ec = (voltage_mv / 1000.0) * ec_factor + ec_offset;
  float ec_result = raw_ec;

  if (enable_ec_tc) {
    float coef = 1.0 + temp_compensation_beta * (current_temp - 25.0);
    ec_result = raw_ec / coef;
  }
  return max(ec_result, 0.0f);
}

// ================= MQTT CALLBACK =================
void mqttCallback(char *topic, byte *payload, unsigned int length) {
  String message = "";
  for (int i = 0; i < length; i++)
    message += (char)payload[i];
  String topicStr = String(topic);

  Serial.printf("📥 [DEBUG-MQTT] Nhận Topic: %s\n", topicStr.c_str());
  Serial.printf("📦 [DEBUG-MQTT] Payload: %s\n", message.c_str());

  if (topicStr == topic_cmd) {
    DynamicJsonDocument doc(384);
    if (!deserializeJson(doc, message)) {
      String action = doc.containsKey("action") ? doc["action"].as<String>()
                                                : doc["command"].as<String>();
      Serial.printf("⚙️ [DEBUG-CMD] Action: %s\n", action.c_str());

      if (action == "set_continuous" || action == "continuous_level") {
        continuous_level = doc.containsKey("params")
                               ? doc["params"]["state"].as<bool>()
                               : doc["state"].as<bool>();
        Serial.printf("⚙️ [DEBUG-CMD] Set continuous_level = %d\n",
                      continuous_level);
      } else if (action == "force_publish") {
        last_publish_time = 0;
        Serial.println("⚙️ [DEBUG-CMD] Force publish kích hoạt!");
      }
    } else {
      Serial.println("❌ [DEBUG-CMD] Lỗi parse JSON command");
    }
    return;
  }

  if (topicStr == topic_config) {
    DynamicJsonDocument doc(1024);
    if (deserializeJson(doc, message)) {
      Serial.println("❌ [DEBUG-CONFIG] Lỗi parse JSON config");
      return;
    }

    if (doc.containsKey("ph_calibration_mode"))
      ph_calibration_mode = doc["ph_calibration_mode"].as<String>();

    if (doc.containsKey("ph_v7"))
      ph_v686 = doc["ph_v7"].as<float>() * 1000;
    if (doc.containsKey("ph_v4"))
      ph_v4 = doc["ph_v4"].as<float>() * 1000;

    if (doc.containsKey("ph_v10"))
      ph_v918 = doc["ph_v10"].as<float>() * 1000;
    else if (doc.containsKey("ph_v918"))
      ph_v918 = doc["ph_v918"].as<float>() * 1000;

    if (doc.containsKey("ec_factor"))
      ec_factor = doc["ec_factor"].as<float>();
    if (doc.containsKey("ec_offset"))
      ec_offset = doc["ec_offset"].as<float>();
    if (doc.containsKey("temp_offset"))
      temp_offset = doc["temp_offset"].as<float>();
    if (doc.containsKey("tank_height"))
      tank_height = doc["tank_height"].as<float>();

    if (doc.containsKey("moving_average_window")) {
      int window = constrain(doc["moving_average_window"].as<int>(), 1, 100);
      float new_alpha = 2.0 / (window + 1.0);
      tempFilter.setAlpha(new_alpha);
      waterFilter.setAlpha(new_alpha);
      phFilter.setAlpha(new_alpha);
      ecFilter.setAlpha(new_alpha);
      Serial.printf(
          "⚙️ [DEBUG-CONFIG] Cập nhật Filter Alpha: %.4f (Window: %d)\n",
          new_alpha, window);
    }

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
    Serial.printf("📋 [DEBUG-CONFIG-VARS] pH Mode: %s, V686: %.1f, V4: %.1f, "
                  "V918: %.1f\n",
                  ph_calibration_mode.c_str(), ph_v686, ph_v4, ph_v918);
    Serial.printf("📋 [DEBUG-CONFIG-VARS] EC Fac: %.3f, EC Off: %.3f, Temp "
                  "Off: %.2f, Tank: %.2f\n",
                  ec_factor, ec_offset, temp_offset, tank_height);
  }
}

void setup() {
  Serial.begin(115200);
  delay(1000);

  pinMode(PIN_TRIG, OUTPUT);
  pinMode(PIN_ECHO, INPUT);

  analogReadResolution(12);
  analogSetAttenuation(ADC_11db);

  sensors.begin();

  Serial.printf("🌐 Đang kết nối WiFi: %s...\n", ssid);
  WiFi.begin(ssid, password);
  while (WiFi.status() != WL_CONNECTED) {
    delay(500);
    Serial.print(".");
  }
  Serial.println("\n✅ Kết nối WiFi thành công!");
  Serial.print("📶 [DEBUG-WIFI] IP Address: ");
  Serial.println(WiFi.localIP());

  client.setBufferSize(1024);
  client.setServer(mqtt_server, mqtt_port);
  client.setCallback(mqttCallback);
}

void reconnect() {
  while (!client.connected()) {
    String clientId = "SensorNode_" + String(device_id);
    Serial.printf("🔄 [DEBUG-MQTT] Đang thử kết nối MQTT... ClientID: %s\n",
                  clientId.c_str());

    if (client.connect(clientId.c_str(), mqtt_user, mqtt_pass,
                       topic_status.c_str(), 1, true, "{\"online\": false}")) {
      Serial.println("✅ [DEBUG-MQTT] Kết nối MQTT thành công!");
      client.publish(topic_status.c_str(), "{\"online\": true}", true);
      client.subscribe(topic_config.c_str());
      client.subscribe(topic_cmd.c_str());
    } else {
      Serial.printf("❌ [DEBUG-MQTT] Kết nối thất bại, state = %d. Đợi 5 giây "
                    "để thử lại...\n",
                    client.state());
      delay(5000);
    }
  }
}

unsigned long last_sample_time = 0;
unsigned long last_publish_time = 0;

void loop() {
  if (WiFi.status() != WL_CONNECTED) {
    Serial.println("⚠️ [DEBUG-WIFI] Mất kết nối WiFi, đang thử kết nối lại...");
    WiFi.disconnect();
    WiFi.reconnect();
    delay(5000);
    return;
  }
  if (!client.connected())
    reconnect();
  client.loop();

  unsigned long current_millis = millis();
  static bool err_water_flag = false;
  static bool err_temp_flag = false;
  static bool err_ph_flag = false;
  static bool err_ec_flag = false;

  // LUỒNG 1: LẤY MẪU VÀ LỌC NHIỄU (Mỗi 200ms)
  if (current_millis - last_sample_time >= SAMPLING_INTERVAL) {
    last_sample_time = current_millis;

    // 1. Nhiệt độ
    float raw_temp = current_avg_temp;
    err_temp_flag = false;
    if (enable_temp) {
      sensors.requestTemperatures();
      float t = sensors.getTempCByIndex(0);
      if (t > -50.0 && t <= 80.0) {
        raw_temp = t + temp_offset;
      } else {
        err_temp_flag = true;
      }
    }
    current_avg_temp = tempFilter.update(raw_temp);

    // 2. Mực nước
    err_water_flag = false;
    if (enable_water) {
      float w = readWaterLevel();
      if (w >= 0) {
        latest_raw_water = w;
        current_avg_water = waterFilter.update(w);
      } else {
        err_water_flag = true;
      }
    }

    // 3. pH
    err_ph_flag = false;
    if (enable_ph) {
      int adc_ph = read_adc_filtered(PIN_PH_ADC);
      if (adc_ph <= 0 || adc_ph >= 4095) {
        err_ph_flag = true;
        latest_ph_voltage_mv = NAN;
      } else {
        float ph_mv = (adc_ph / ADC_MAX) * V_REF_MV * VOLTAGE_DIVIDER_RATIO;
        latest_ph_voltage_mv = ph_mv;
        float ph_val = calculate_ph(ph_mv, current_avg_temp);

        // [THÊM MỚI CẬP NHẬT]: Lưu giá trị raw và cập nhật vào bộ lọc
        latest_raw_ph = ph_val;
        current_avg_ph = phFilter.update(ph_val);

        // [THÊM MỚI CẬP NHẬT]: In log so sánh rõ ràng giữa RAW và EMA mỗi 200ms
        Serial.printf("🧪 [DEBUG-pH-Filter] V_mv: %.2f mV | pH Raw (ko EMA): "
                      "%.2f | pH Filtered (EMA): %.2f\n",
                      ph_mv, latest_raw_ph, current_avg_ph);
      }
    }

    // 4. EC
    err_ec_flag = false;
    if (enable_ec) {
      int adc_ec = read_adc_filtered(PIN_EC_ADC);
      if (adc_ec <= 0 || adc_ec >= 4095) {
        err_ec_flag = true;
      } else {
        float ec_mv = (adc_ec / ADC_MAX) * V_REF_MV * VOLTAGE_DIVIDER_RATIO;
        float ec_val = calculate_ec(ec_mv, current_avg_temp);
        current_avg_ec = ecFilter.update(ec_val);
      }
    }
  }

  // LUỒNG 2: GỬI DỮ LIỆU LÊN SERVER (Publish)
  int current_pub_interval = continuous_level ? 500 : publish_interval;
  if (current_millis - last_publish_time >= current_pub_interval) {
    last_publish_time = current_millis;
    DynamicJsonDocument doc(512);

    doc["temp"] = current_avg_temp;
    doc["water_level"] =
        continuous_level ? latest_raw_water : current_avg_water;
    doc["ph"] = current_avg_ph;

    // [THÊM MỚI CẬP NHẬT]: Đẩy cả giá trị pH Raw lên MQTT để tiện so sánh trên
    // biểu đồ Server
    doc["ph_raw"] = latest_raw_ph;

    doc["ec"] = current_avg_ec;
    if (!isnan(latest_ph_voltage_mv))
      doc["ph_voltage_mv"] = latest_ph_voltage_mv;

    doc["rssi"] = WiFi.RSSI();
    doc["free_heap"] = ESP.getFreeHeap();
    doc["uptime"] = millis() / 1000;
    doc["is_continuous"] = continuous_level;

    doc["err_water"] = err_water_flag;
    doc["err_temp"] = err_temp_flag;
    doc["err_ph"] = err_ph_flag;
    doc["err_ec"] = err_ec_flag;

    String payload;
    serializeJson(doc, payload);
    client.publish(topic_sensors.c_str(), payload.c_str());

    Serial.println("\n-----------------------------------------");
    Serial.printf("📤 [DEBUG-PUB] Topic: %s\n", topic_sensors.c_str());
    Serial.printf("📤 [DEBUG-PUB] Payload: %s\n", payload.c_str());

    // [THÊM MỚI CẬP NHẬT]: Bổ sung hiển thị cả pH Raw vào tóm tắt Debug Publish
    Serial.printf("📊 [DEBUG-DATA] Temp: %.2f °C | Water: %.2f cm | pH Raw: "
                  "%.2f | pH (EMA): %.2f | EC: %.2f\n",
                  current_avg_temp,
                  continuous_level ? latest_raw_water : current_avg_water,
                  latest_raw_ph, current_avg_ph, current_avg_ec);
    Serial.println("-----------------------------------------");
  }
}
