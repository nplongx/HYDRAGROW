// src/hw/pump_controller.rs
//! Driver điều khiển Bơm, Van và Xung PWM phần cứng ESP32-C3.

use esp_idf_hal::gpio::{Output, PinDriver};
use esp_idf_hal::ledc::LedcDriver;

pub use hydragrow_controller_core::{PumpType, WaterDirection};

use log::{info, warn};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::hw::pcf857x::{ExpanderPin, I2cExpander, TankAlert};

pub struct PumpController<'d> {
    pump_a: LedcDriver<'static>,
    pump_b: LedcDriver<'static>,
    pump_ph_up: LedcDriver<'static>,
    pump_ph_down: LedcDriver<'static>,

    // Valve Mist / Mix sử dụng PCF8574
    valve: I2cExpander<'d>,

    water_pump_in: PinDriver<'static, Output>,
    water_pump_out: PinDriver<'static, Output>,

    osaka_en: PinDriver<'static, Output>,
    osaka_rpwm: Arc<Mutex<LedcDriver<'static>>>,

    cancel_soft_start: Arc<AtomicBool>,
}

impl<'d> PumpController<'d> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        mut pump_a: LedcDriver<'static>,
        mut pump_b: LedcDriver<'static>,
        mut pump_ph_up: LedcDriver<'static>,
        mut pump_ph_down: LedcDriver<'static>,

        mut valve: I2cExpander<'d>,

        mut water_pump_in: PinDriver<'static, Output>,
        mut water_pump_out: PinDriver<'static, Output>,

        mut osaka_en: PinDriver<'static, Output>,
        mut osaka_rpwm: LedcDriver<'static>,
    ) -> anyhow::Result<Self> {
        // -------------------------------------------------------------------
        // Dosing pumps OFF
        // -------------------------------------------------------------------

        pump_a.set_duty(0)?;
        pump_b.set_duty(0)?;
        pump_ph_up.set_duty(0)?;
        pump_ph_down.set_duty(0)?;

        // -------------------------------------------------------------------
        // PCF8574
        //
        // P0..P3 = TTP223 INPUT
        // P4    = Mist Valve OUTPUT
        // P5    = Mix Valve OUTPUT
        //
        // init() sẽ set:
        //
        // P0 = HIGH/released
        // P1 = HIGH/released
        // P2 = HIGH/released
        // P3 = HIGH/released
        // P4 = LOW
        // P5 = LOW
        // -------------------------------------------------------------------

        valve
            .init()
            .map_err(|e| anyhow::anyhow!("PCF8574 init failed: {:?}", e))?;

        // Đảm bảo hai valve OFF khi khởi động.
        //
        // Lưu ý:
        // KHÔNG dùng .mask() ở đây.
        valve
            .set_low(ExpanderPin::ValveMist)
            .map_err(|e| anyhow::anyhow!("PCF8574 ValveMist OFF failed: {:?}", e))?;

        valve
            .set_low(ExpanderPin::ValveMix)
            .map_err(|e| anyhow::anyhow!("PCF8574 ValveMix OFF failed: {:?}", e))?;

        // -------------------------------------------------------------------
        // Water pump OFF
        // -------------------------------------------------------------------

        water_pump_in.set_low()?;
        water_pump_out.set_low()?;

        // -------------------------------------------------------------------
        // Osaka pump OFF
        // -------------------------------------------------------------------

        osaka_en.set_low()?;
        osaka_rpwm.set_duty(0)?;

        info!("🔌 Initialize PumpController (Bơm In/Out dùng Relay Bật/Tắt).");

        Ok(Self {
            pump_a,
            pump_b,
            pump_ph_up,
            pump_ph_down,

            valve,

            water_pump_in,
            water_pump_out,

            osaka_en,
            osaka_rpwm: Arc::new(Mutex::new(osaka_rpwm)),

            cancel_soft_start: Arc::new(AtomicBool::new(false)),
        })
    }

    // =========================================================================
    // DOSING PUMPS
    // =========================================================================

    /// Điều khiển PWM cho bơm định lượng.
    ///
    /// `percent`: 0..100
    /// `state = false`: OFF
    /// `state = true`: chạy theo percent
    pub fn set_dosing_pump_pwm(
        &mut self,
        pump: PumpType,
        state: bool,
        percent: u32,
    ) -> anyhow::Result<()> {
        let safe_percent = percent.clamp(0, 100);

        let target_driver = match pump {
            PumpType::NutrientA => &mut self.pump_a,
            PumpType::NutrientB => &mut self.pump_b,
            PumpType::PhUp => &mut self.pump_ph_up,
            PumpType::PhDown => &mut self.pump_ph_down,
        };

        let max = target_driver.get_max_duty();

        let duty = if state {
            (max as f32 * safe_percent as f32 / 100.0) as u32
        } else {
            0
        };

        target_driver.set_duty(duty)?;

        Ok(())
    }

    /// Bật/tắt dosing pump ở 100%.
    pub fn set_pump_state(
        &mut self,
        pump: PumpType,
        state: bool,
    ) -> anyhow::Result<()> {
        self.set_dosing_pump_pwm(pump, state, 100)
    }

    // =========================================================================
    // WATER PUMP
    // =========================================================================

    /// Điều khiển bơm nước In / Out.
    ///
    /// Có delay 100ms khi đảo chiều để tránh đóng hai relay cùng lúc.
    pub fn set_water_pump(
        &mut self,
        direction: WaterDirection,
    ) -> anyhow::Result<()> {
        match direction {
            WaterDirection::In => {
                // Tắt Out trước
                self.water_pump_out.set_low()?;

                // Chờ relay nhả
                thread::sleep(Duration::from_millis(100));

                // Bật In
                self.water_pump_in.set_high()?;
            }

            WaterDirection::Out => {
                // Tắt In trước
                self.water_pump_in.set_low()?;

                // Chờ relay nhả
                thread::sleep(Duration::from_millis(100));

                // Bật Out
                self.water_pump_out.set_high()?;
            }

            WaterDirection::Stop => {
                self.water_pump_in.set_low()?;
                self.water_pump_out.set_low()?;
            }
        }

        Ok(())
    }

    // =========================================================================
    // MIST VALVE
    // =========================================================================

    /// Bật/tắt van Mist.
    ///
    /// PCF8574:
    /// P4 = ValveMist
    pub fn set_mist_valve(
        &mut self,
        state: bool,
    ) -> anyhow::Result<()> {
        if state {
            self.valve
                .set_high(ExpanderPin::ValveMist)
                .map_err(|e| {
                    anyhow::anyhow!(
                        "PCF8574 ValveMist ON failed: {:?}",
                        e
                    )
                })?;
        } else {
            self.valve
                .set_low(ExpanderPin::ValveMist)
                .map_err(|e| {
                    anyhow::anyhow!(
                        "PCF8574 ValveMist OFF failed: {:?}",
                        e
                    )
                })?;
        }

        Ok(())
    }

    // =========================================================================
    // MIX VALVE
    // =========================================================================

    /// Bật/tắt van Mix.
    ///
    /// PCF8574:
    /// P5 = ValveMix
    pub fn set_mix_valve(
        &mut self,
        state: bool,
    ) -> anyhow::Result<()> {
        if state {
            self.valve
                .set_high(ExpanderPin::ValveMix)
                .map_err(|e| {
                    anyhow::anyhow!(
                        "PCF8574 ValveMix ON failed: {:?}",
                        e
                    )
                })?;
        } else {
            self.valve
                .set_low(ExpanderPin::ValveMix)
                .map_err(|e| {
                    anyhow::anyhow!(
                        "PCF8574 ValveMix OFF failed: {:?}",
                        e
                    )
                })?;
        }

        Ok(())
    }

    // =========================================================================
    // OSAKA PUMP
    // =========================================================================

    /// Khởi động mềm bơm Osaka.
    ///
    /// PWM tăng dần từ 0 -> target_pwm_percent trong khoảng 3 giây.
    pub fn start_osaka_pump_soft(
        &mut self,
        target_pwm_percent: u32,
    ) -> anyhow::Result<()> {
        info!(
            "🌀 Điều khiển khởi động mềm Osaka lên {}%...",
            target_pwm_percent
        );

        self.osaka_en.set_high()?;

        // Hủy soft-start cũ nếu có
        self.cancel_soft_start.store(false, Ordering::SeqCst);

        let rpwm_clone = Arc::clone(&self.osaka_rpwm);
        let cancel_flag = Arc::clone(&self.cancel_soft_start);

        let safe_percent = target_pwm_percent.clamp(0, 100);

        thread::spawn(move || {
            let max_duty = {
                let pump = match rpwm_clone.lock() {
                    Ok(pump) => pump,
                    Err(_) => {
                        warn!("⚠️ Không thể lock Osaka RPWM!");
                        return;
                    }
                };

                pump.get_max_duty()
            };

            let target_duty =
                (max_duty as f32 * safe_percent as f32 / 100.0) as u32;

            let steps = 30;
            let step_delay = Duration::from_millis(100);

            for i in 1..=steps {
                // -----------------------------------------------------------
                // Kiểm tra cancel
                // -----------------------------------------------------------

                if cancel_flag.load(Ordering::SeqCst) {
                    warn!("⚠️ Hủy tiến trình khởi động mềm Osaka!");

                    if let Ok(mut pump) = rpwm_clone.lock() {
                        let _ = pump.set_duty(0);
                    }

                    return;
                }

                // -----------------------------------------------------------
                // Tính duty hiện tại
                // -----------------------------------------------------------

                let current_duty =
                    target_duty * i / steps;

                // -----------------------------------------------------------
                // Ghi PWM
                // -----------------------------------------------------------

                if let Ok(mut pump) = rpwm_clone.lock() {
                    let _ = pump.set_duty(current_duty);
                } else {
                    warn!("⚠️ Không thể lock Osaka RPWM!");
                    return;
                }

                thread::sleep(step_delay);
            }

            info!(
                "✅ Bơm Osaka đạt {}%!",
                safe_percent
            );
        });

        Ok(())
    }

    /// Điều khiển trực tiếp PWM bơm Osaka.
    ///
    /// duty_percent = 0 -> OFF
    /// duty_percent > 0 -> ON + PWM
    pub fn set_osaka_pump_pwm(
        &mut self,
        duty_percent: u32,
    ) -> anyhow::Result<()> {
        if duty_percent == 0 {
            // ---------------------------------------------------------------
            // OFF
            // ---------------------------------------------------------------

            // Hủy soft-start nếu đang chạy
            self.cancel_soft_start
                .store(true, Ordering::SeqCst);

            // Tắt enable
            self.osaka_en.set_low()?;

            // PWM = 0
            let mut pump = self
                .osaka_rpwm
                .lock()
                .map_err(|_| anyhow::anyhow!("Osaka RPWM mutex poisoned"))?;

            pump.set_duty(0)?;
        } else {
            // ---------------------------------------------------------------
            // ON
            // ---------------------------------------------------------------

            let percent = duty_percent.min(100);

            self.osaka_en.set_high()?;

            let mut pump = self
                .osaka_rpwm
                .lock()
                .map_err(|_| anyhow::anyhow!("Osaka RPWM mutex poisoned"))?;

            let max_duty = pump.get_max_duty();

            let target_duty =
                ((max_duty as f32 * percent as f32) / 100.0) as u32;

            pump.set_duty(target_duty)?;
        }

        Ok(())
    }

    // =========================================================================
    // TANK ALERT / TTP223
    // =========================================================================

    /// Đọc trạng thái 4 cảm biến TTP223 thông qua PCF8574.
    ///
    /// Mapping:
    ///
    /// P0 -> Tank A
    /// P1 -> Tank B
    /// P2 -> Tank pH Down
    /// P3 -> Tank pH Up
    ///
    /// TTP223 active-HIGH:
    ///
    /// OUT = LOW  -> không chạm
    /// OUT = HIGH -> chạm
    pub fn check_tank_alert(
        &mut self,
    ) -> anyhow::Result<TankAlert> {
        self.valve
            .parse_tank_alert()
            .map_err(|e| {
                anyhow::anyhow!(
                    "Lỗi đọc I2C TankAlert: {:?}",
                    e
                )
            })
    }
}
