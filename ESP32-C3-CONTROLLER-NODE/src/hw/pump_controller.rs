// src/hw/pump_controller.rs
//! Driver điều khiển Bơm, Van và Xung PWM phần cứng ESP32-C3.

use esp_idf_hal::ledc::LedcDriver;

pub use hydragrow_controller_core::{PumpType, WaterDirection};

use log::{info, warn};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::hw::pcf857x::{ExpanderPin, I2cExpander, TankAlert};

pub struct PumpController<'d> {
    pump_a: LedcDriver<'static>,
    pump_b: LedcDriver<'static>,
    pump_ph_up: LedcDriver<'static>,
    pump_ph_down: LedcDriver<'static>,

    // PCF8574: P4/P5 = valves, P6/P7 = water pumps.
    valve: I2cExpander<'d>,

    osaka_en: esp_idf_hal::gpio::PinDriver<'static, esp_idf_hal::gpio::Output>,
    osaka_rpwm: Arc<Mutex<LedcDriver<'static>>>,

    soft_start_generation: Arc<AtomicUsize>,
}

impl<'d> PumpController<'d> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        mut pump_a: LedcDriver<'static>,
        mut pump_b: LedcDriver<'static>,
        mut pump_ph_up: LedcDriver<'static>,
        mut pump_ph_down: LedcDriver<'static>,

        mut valve: I2cExpander<'d>,

        mut osaka_en: esp_idf_hal::gpio::PinDriver<'static, esp_idf_hal::gpio::Output>,
        mut osaka_rpwm: LedcDriver<'static>,
    ) -> anyhow::Result<Self> {
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
        // P6    = Water Pump IN OUTPUT
        // P7    = Water Pump OUT OUTPUT
        //
        // init() đặt P0..P3 HIGH/released và P4..P7 LOW/OFF.
        // -------------------------------------------------------------------
        valve
            .init()
            .map_err(|e| anyhow::anyhow!("PCF8574 init failed: {:?}", e))?;

        valve
            .set_low(ExpanderPin::ValveMist)
            .map_err(|e| anyhow::anyhow!("PCF8574 ValveMist OFF failed: {:?}", e))?;
        valve
            .set_low(ExpanderPin::ValveMix)
            .map_err(|e| anyhow::anyhow!("PCF8574 ValveMix OFF failed: {:?}", e))?;
        valve
            .set_low(ExpanderPin::WaterPumpIn)
            .map_err(|e| anyhow::anyhow!("PCF8574 WaterPumpIn OFF failed: {:?}", e))?;
        valve
            .set_low(ExpanderPin::WaterPumpOut)
            .map_err(|e| anyhow::anyhow!("PCF8574 WaterPumpOut OFF failed: {:?}", e))?;

        osaka_en.set_low()?;
        osaka_rpwm.set_duty(0)?;

        info!("🔌 Initialize PumpController (Water Pump IN=P6, OUT=P7).");

        Ok(Self {
            pump_a,
            pump_b,
            pump_ph_up,
            pump_ph_down,
            valve,
            osaka_en,
            osaka_rpwm: Arc::new(Mutex::new(osaka_rpwm)),
            soft_start_generation: Arc::new(AtomicUsize::new(0)),
        })
    }

    // =========================================================================
    // DOSING PUMPS
    // =========================================================================

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

    pub fn set_pump_state(
        &mut self,
        pump: PumpType,
        state: bool,
    ) -> anyhow::Result<()> {
        self.set_dosing_pump_pwm(pump, state, 100)
    }

    // =========================================================================
    // WATER PUMP - PCF8574 P6/P7
    // =========================================================================

    /// Điều khiển bơm nước In / Out qua PCF8574:
    ///
    /// P6 = WaterPumpIn
    /// P7 = WaterPumpOut
    ///
    /// Khi đảo chiều, luôn tắt bơm đang chạy trước, chờ relay nhả 100ms,
    /// rồi mới bật bơm còn lại.
    pub fn set_water_pump(
        &mut self,
        direction: WaterDirection,
    ) -> anyhow::Result<()> {
        match direction {
            WaterDirection::In => {
                self.valve
                    .set_low(ExpanderPin::WaterPumpOut)
                    .map_err(|e| anyhow::anyhow!("PCF8574 WaterPumpOut OFF failed: {:?}", e))?;

                thread::sleep(Duration::from_millis(100));

                self.valve
                    .set_high(ExpanderPin::WaterPumpIn)
                    .map_err(|e| anyhow::anyhow!("PCF8574 WaterPumpIn ON failed: {:?}", e))?;
            }

            WaterDirection::Out => {
                self.valve
                    .set_low(ExpanderPin::WaterPumpIn)
                    .map_err(|e| anyhow::anyhow!("PCF8574 WaterPumpIn OFF failed: {:?}", e))?;

                thread::sleep(Duration::from_millis(100));

                self.valve
                    .set_high(ExpanderPin::WaterPumpOut)
                    .map_err(|e| anyhow::anyhow!("PCF8574 WaterPumpOut ON failed: {:?}", e))?;
            }

            WaterDirection::Stop => {
                self.valve
                    .set_low(ExpanderPin::WaterPumpIn)
                    .map_err(|e| anyhow::anyhow!("PCF8574 WaterPumpIn OFF failed: {:?}", e))?;
                self.valve
                    .set_low(ExpanderPin::WaterPumpOut)
                    .map_err(|e| anyhow::anyhow!("PCF8574 WaterPumpOut OFF failed: {:?}", e))?;
            }
        }

        Ok(())
    }

    // =========================================================================
    // MIST VALVE
    // =========================================================================

    pub fn set_mist_valve(&mut self, state: bool) -> anyhow::Result<()> {
        if state {
            self.valve
                .set_high(ExpanderPin::ValveMist)
                .map_err(|e| anyhow::anyhow!("PCF8574 ValveMist ON failed: {:?}", e))?;
        } else {
            self.valve
                .set_low(ExpanderPin::ValveMist)
                .map_err(|e| anyhow::anyhow!("PCF8574 ValveMist OFF failed: {:?}", e))?;
        }
        Ok(())
    }

    // =========================================================================
    // MIX VALVE
    // =========================================================================

    pub fn set_mix_valve(&mut self, state: bool) -> anyhow::Result<()> {
        if state {
            self.valve
                .set_high(ExpanderPin::ValveMix)
                .map_err(|e| anyhow::anyhow!("PCF8574 ValveMix ON failed: {:?}", e))?;
        } else {
            self.valve
                .set_low(ExpanderPin::ValveMix)
                .map_err(|e| anyhow::anyhow!("PCF8574 ValveMix OFF failed: {:?}", e))?;
        }
        Ok(())
    }

    // =========================================================================
    // OSAKA PUMP
    // =========================================================================

    pub fn start_osaka_pump_soft(
        &mut self,
        target_pwm_percent: u32,
    ) -> anyhow::Result<()> {
        info!(
            "🌀 Điều khiển khởi động mềm Osaka lên {}%...",
            target_pwm_percent
        );
        self.osaka_en.set_high()?;

        // Bump generation FIRST — any in-flight thread from a previous call
        // reads a stale generation on its next check and exits on its own,
        // instead of racing this new thread on the same PWM duty.
        let my_generation = self
            .soft_start_generation
            .fetch_add(1, Ordering::SeqCst)
            + 1;
        let generation_flag = Arc::clone(&self.soft_start_generation);
        let rpwm_clone = Arc::clone(&self.osaka_rpwm);
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

            let target_duty = (max_duty as f32 * safe_percent as f32 / 100.0) as u32;
            let steps = 30;
            let step_delay = Duration::from_millis(100);

            for i in 1..=steps {
                if generation_flag.load(Ordering::SeqCst) != my_generation {
                    warn!(
                        "⚠️ Hủy tiến trình khởi động mềm Osaka (generation {} superseded)!",
                        my_generation
                    );
                    return;
                }

                let current_duty = target_duty * i / steps;
                if let Ok(mut pump) = rpwm_clone.lock() {
                    let _ = pump.set_duty(current_duty);
                } else {
                    warn!("⚠️ Không thể lock Osaka RPWM!");
                    return;
                }

                thread::sleep(step_delay);
            }

            info!("✅ Bơm Osaka đạt {}%!", safe_percent);
        });

        Ok(())
    }

    pub fn set_osaka_pump_pwm(
        &mut self,
        duty_percent: u32,
    ) -> anyhow::Result<()> {
        if duty_percent == 0 {
            self.soft_start_generation.fetch_add(1, Ordering::SeqCst);
            self.osaka_en.set_low()?;

            let mut pump = self
                .osaka_rpwm
                .lock()
                .map_err(|_| anyhow::anyhow!("Osaka RPWM mutex poisoned"))?;
            pump.set_duty(0)?;
        } else {
            let percent = duty_percent.min(100);
            self.osaka_en.set_high()?;

            let mut pump = self
                .osaka_rpwm
                .lock()
                .map_err(|_| anyhow::anyhow!("Osaka RPWM mutex poisoned"))?;
            let max_duty = pump.get_max_duty();
            let target_duty = ((max_duty as f32 * percent as f32) / 100.0) as u32;
            pump.set_duty(target_duty)?;
        }

        Ok(())
    }

    // =========================================================================
    // TANK ALERT / TTP223
    // =========================================================================

    pub fn check_tank_alert(&mut self) -> anyhow::Result<TankAlert> {
        self.valve
            .parse_tank_alert()
            .map_err(|e| anyhow::anyhow!("Lỗi đọc I2C TankAlert: {:?}", e))
    }
}
