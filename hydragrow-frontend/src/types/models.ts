export interface AppSettings {
  backend_url: string;
  api_key: string;
  device_id: string;
  [key: string]: any;
}

// src/types/models.ts

/**
 * Trạng thái hoạt động cơ bản của thiết bị
 */
export type DeviceState = 'on' | 'off';

/**
 * Trạng thái chi tiết của tất cả các máy bơm và van trong hệ thống
 */
export interface PumpStatus {
  pump_a: boolean;          // Bơm dinh dưỡng A
  pump_b: boolean;          // Bơm dinh dưỡng B
  ph_up: boolean;      // Bơm tăng pH
  ph_down: boolean;    // Bơm giảm pH
  osaka_pump: boolean; // Bơm trộn/phun sương chính (Osaka)
  mist_valve: boolean; // Van điện từ phun sương
  mix_valve: boolean;
  water_pump_in: boolean; // Bơm cấp nước vào (In)
  water_pump_out: boolean; // Bơm thoát nước ra (Out)
  pump_a_pwm?: number;
  pump_b_pwm?: number;
  ph_up_pwm?: number;
  ph_down_pwm?: number;
  osaka_pwm?: number;
  dosing_pulse_active?: boolean;
  dosing_pulse_count?: number;
}

/**
 * Dữ liệu thu thập từ các cảm biến của thiết bị
 */
export interface SensorData extends DeviceHealth {
  device_id: string;
  tds: number;       // Giá trị TDS (tổng chất rắn hòa tan)
  ph: number;        // Giá trị pH
  temp: number;      // Nhiệt độ nước/môi trường
  water_level: number;     // Mực nước (cm)
  pump_status: PumpStatus; // Trạng thái bơm đồng bộ từ FSM
  time: string;       // Thời gian ghi nhận dữ liệu
  rssi?: number;
  free_heap?: number;
  uptime?: number;
  err_water?: boolean;
  err_temp?: boolean;
  err_ph?: boolean;
  err_tds?: boolean;
  is_continuous?: boolean;
  ph_voltage_mv?: number;
}

/**
 * Cấu trúc thông báo cảnh báo hệ thống
 */
export interface AlertPayload {
  id: string;
  metric: string;          // Chỉ số gây ra lỗi (ví dụ: "TDS", "WaterLevel")
  value: number;           // Giá trị tại thời điểm xảy ra lỗi
  severity: 'info' | 'warning' | 'critical';
  message: string;         // Nội dung cảnh báo chi tiết
  timestamp: string;
}

/**
 * Trạng thái kết nối của thiết bị với Server/Broker
 */
export interface StatusPayload {
  is_online: boolean;      // Thiết bị có đang kết nối MQTT không
  last_seen: string;       // Lần cuối cùng nhận được tín hiệu (Heartbeat)
}

/**
 * Trạng thái máy trạng thái (FSM) gửi từ Controller
 */
export interface FsmStatePayload {
  current_state: string;   // Ví dụ: "Monitoring", "DosingTDS", "EmergencyStop"
  timestamp: string;
}

// src/types/models.ts

export interface UnifiedDeviceConfig {
  device_id: string;
  control_mode: 'auto' | 'manual';
  is_enabled: boolean;

  // --- Ngưỡng mục tiêu ---
  tds_target: number;
  tds_tolerance: number;
  ph_target: number;
  ph_tolerance: number;

  // --- Nước & Bơm ---
  water_level_min: number;
  water_level_target: number;
  water_level_max: number;
  water_level_tolerance: number;
  auto_refill_enabled: boolean;
  auto_drain_overflow: boolean;
  auto_dilute_enabled: boolean;
  dilute_drain_amount_cm: number;
  scheduled_water_change_enabled: boolean;
  water_change_cron: number;
  scheduled_drain_amount_cm: number;
  misting_on_duration_ms: number;
  misting_off_duration_ms: number;

  // --- An Toàn ---
  emergency_shutdown: boolean;
  max_tds_limit: number;
  min_tds_limit: number;
  min_ph_limit: number;
  max_ph_limit: number;
  max_tds_delta: number;
  max_ph_delta: number;
  max_dose_per_cycle: number;
  water_level_critical_min: number;
  max_refill_duration_sec: number;
  max_drain_duration_sec: number;
  tds_ack_threshold: number;
  ph_ack_threshold: number;
  water_ack_threshold: number;

  // --- Châm Phân ---
  tds_gain_per_ml: number;
  ph_shift_up_per_ml: number;
  ph_shift_down_per_ml: number;
  active_mixing_sec: number;
  sensor_stabilize_sec: number;
  tds_step_ratio: number;
  ph_step_ratio: number;
  best_tds_ratio?: number;
  best_ph_ratio?: number;
  tuner_state?: number;
  adaptive_mixing_sec?: number;
  adaptive_stabilize_sec?: number;
  effective_tds_tolerance?: number;
  effective_ph_tolerance?: number;
  interaction_matrix?: number[];
  matrix_update_count?: number;
  matrix_is_warm?: boolean;
  kalman_confidence?: number[];
  dosing_pump_capacity_ml_per_sec: number;
  soft_start_duration: number;
  scheduled_mixing_interval_sec: number;
  scheduled_mixing_duration_sec: number;

  // --- Cảm biến & Lọc nhiễu ---
  ph_v7: number;
  ph_v4: number;
  tds_factor: number;
  tds_offset: number;
  temp_offset: number;
  temp_compensation_beta: number;
  sampling_interval: number;
  publish_interval: number;
  moving_average_window: number;

  // --- Cờ Bật/Tắt Cảm Biến ---
  enable_tds_sensor: boolean;
  enable_ph_sensor: boolean;
  enable_water_level_sensor: boolean;
  enable_temp_sensor: boolean;

  dosing_pwm_percent: number;
  osaka_mixing_pwm_percent: number;
  osaka_misting_pwm_percent: number;
}

// src/types/models.ts

// Cập nhật các trạng thái FSM mới nhất từ ESP32
export type SystemState =
  | "Monitoring"
  | "EmergencyStop"
  | "WaterRefilling"
  | "WaterDraining"
  | "DosingPumpA"
  | "WaitingBetweenDose"
  | "DosingPumpB"
  | "DosingPH"
  | "StartingOsakaPump"
  | "ActiveMixing"
  | "Stabilizing"
  // Thêm [key: string] để bắt các lỗi linh động dạng "SystemFault:Lý_do"
  | string;

export interface AlertMessage {
  level: "info" | "success" | "warning" | "critical";
  title: string;
  message: string;
  device_id: string;
  timestamp: number;
}

export interface CropSeason {
  id: string;
  device_id: string;
  name: string;
  plant_type: string | null;
  description: string | null;
  start_time: string;
  end_time: string | null;
  status: 'active' | 'completed';
}

export interface DeviceHealth {
  rssi?: number;
  free_heap?: number;
  uptime?: number;
}

export interface ControllerData extends DeviceHealth {
  // Có thể chứa thêm trạng thái FSM hoặc các config khác nếu cần
}

export interface TankAlert {
  tank_a_low: boolean;
  tank_b_low: boolean;
  tank_ph_down_low: boolean;
  tank_ph_up_low: boolean;
}
