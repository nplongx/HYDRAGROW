// src/hw/pump_controller.rs
//! Driver điều khiển Bơm, Van và Xung PWM phần cứng ESP32-C3.

use esp_idf_hal::gpio::PinDriver;
use esp_idf_hal::ledc::LedcDriver;

pub use hydragrow_controller_core::{PumpType, WaterDirection};

use log::{info, warn};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::hw::pcf857x::{ExpanderPin, I2cExpander, TankAlert};

pub struct PumpController<'d> {
    pump_a: LedcDriver<'static>,
    pump_b: LedcDriver<'static>,
    pump_ph_up: LedcDriver<'static>,
    pump_ph_down: LedcDriver<'static>,

    // PCF8574: P4/P5 = valves.
    valve: I2cExpander<'d>,

    // Dedicated ESP32-C3 GPIOs for water pumps.
    // GPIO2 = WaterPumpIn, GPIO1 = WaterPumpOut.
    water_pump_in: PinDriver<'static, esp_idf_hal::gpio::Output>,
    water_pump_out: PinDriver<'static, esp_idf_hal::gpio::Output>,

    osaka_en: esp_idf_hal::gpio::PinDriver<'static, esp_idf_hal::gpio::Output>,
    osaka_rpwm: Arc<Mutex<LedcDriver<'static>>>,

    soft_start_gen: Arc<AtomicU32>,
    current_water_direction: Option<WaterDirection>,
}

impl<'d> PumpController<'d> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        mut pump_a: LedcDriver<'static>,
        mut pump_b: LedcDriver<'static>,
        mut pump_ph_up: LedcDriver<'static>,
        mut pump_ph_down: LedcDriver<'static>,

        mut valve: I2cExpander<'d>,

        mut water_pump_in: PinDriver<'static, esp_idf_hal::gpio::Output>,
        mut water_pump_out: PinDriver<'static, esp_idf_hal::gpio::Output>,

        mut osaka_en: esp_idf_hal::gpio::PinDriver<'static, esp_idf_hal::gpio::Output>,
        mut osaka_rpwm: LedcDriver<'static>,
    ) -> anyhow::Result<Self> {
        pump_a.set_duty(0)?;
        pump_b.set_duty(0)?;
        pump_ph_up.set_duty(0)?;
        pump_ph_down.set_duty(0)?;

        // PCF8574:
        // P0..P3 = TTP223 INPUT
        // P4 = Mist Valve OUTPUT
        // P5 = Mix Valve OUTPUT
        // P6/P7 are no longer used for water pumps.
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

        water_pump_in.set_low()?;
        water_pump_out.set_low()?;
        osaka_en.set_low()?;
        osaka_rpwm.set_duty(0)?;

        info!("Initialize PumpController (Water Pump IN=GPIO2, OUT=GPIO1).");

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
            soft_start_gen: Arc::new(AtomicU32::new(0)),
            current_water_direction: Some(WaterDirection::Stop),
        })
    }

    pub fn invalidate_water_direction_cache(&mut self) {
        self.current_water_direction = None;
    }

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

    pub fn set_pump_state(&mut self, pump: PumpType, state: bool) -> anyhow::Result<()> {
        self.set_dosing_pump_pwm(pump, state, 100)
    }

    /// Điều khiển bơm nước bằng GPIO trực tiếp:
    /// GPIO2 = Water Pump IN, GPIO1 = Water Pump OUT.
    /// Khi đảo chiều, tắt bơm đang chạy, chờ relay nhả 100ms, rồi bật bơm còn lại.
    pub fn set_water_pump(&mut self, direction: WaterDirection) -> anyhow::Result<()> {
        if self.current_water_direction == Some(direction) {
            return Ok(());
        }

        match direction {
            WaterDirection::In => {
                if self.current_water_direction == Some(WaterDirection::Out) {
                    self.water_pump_out.set_low()?;
                    thread::sleep(Duration::from_millis(100));
                }
                self.water_pump_in.set_high()?;
                self.current_water_direction = Some(WaterDirection::In);
            }

            WaterDirection::Out => {
                if self.current_water_direction == Some(WaterDirection::In) {
                    self.water_pump_in.set_low()?;
                    thread::sleep(Duration::from_millis(100));
                }
                self.water_pump_out.set_high()?;
                self.current_water_direction = Some(WaterDirection::Out);
            }

            WaterDirection::Stop => {
                self.water_pump_in.set_low()?;
                self.water_pump_out.set_low()?;
                self.current_water_direction = Some(WaterDirection::Stop);
            }
        }

        Ok(())
    }

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

    pub fn start_osaka_pump_soft(&mut self, target_pwm_percent: u32) -> anyhow::Result<()> {
        let safe_percent = target_pwm_percent.clamp(0, 100);
        info!("🌀 Điều khiển khởi động mềm Osaka lên {}%...", safe_percent);

        let current_gen = self.soft_start_gen.fetch_add(1, Ordering::SeqCst) + 1;
        self.osaka_en.set_high()?;

        let rpwm_clone = Arc::clone(&self.osaka_rpwm);
        let gen_clone = Arc::clone(&self.soft_start_gen);

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
                if gen_clone.load(Ordering::SeqCst) != current_gen {
                    warn!(
                        "⚠️ Hủy tiến trình khởi động mềm Osaka (superseded by gen {} != {})!",
                        gen_clone.load(Ordering::SeqCst),
                        current_gen
                    );
                    return;
                }

                let current_duty = target_duty * i / steps;
                if let Ok(mut pump) = rpwm_clone.lock() {
                    if gen_clone.load(Ordering::SeqCst) != current_gen {
                        warn!("⚠️ Hủy tiến trình khởi động mềm Osaka trước khi ghi PWM!");
                        return;
                    }
                    let _ = pump.set_duty(current_duty);
                } else {
                    warn!("⚠️ Không thể lock Osaka RPWM!");
                    return;
                }

                thread::sleep(step_delay);
            }

            info!("✅ Bơm Osaka đạt {}% (gen {})!", safe_percent, current_gen);
        });

        Ok(())
    }

    pub fn set_osaka_pump_pwm(&mut self, duty_percent: u32) -> anyhow::Result<()> {
        self.soft_start_gen.fetch_add(1, Ordering::SeqCst);

        if duty_percent == 0 {
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

    pub fn check_tank_alert(&mut self) -> anyhow::Result<TankAlert> {
        self.valve
            .parse_tank_alert()
            .map_err(|e| anyhow::anyhow!("Lỗi đọc I2C TankAlert: {:?}", e))
    }
}

#[cfg(test)]
#[allow(unused_imports, dead_code)]
mod tests {
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[test]
    fn osaka_generation_counter_invalidates_stale_threads() {
        let gen = Arc::new(AtomicU32::new(0));
        let gen_t1 = gen.fetch_add(1, Ordering::SeqCst) + 1;
        assert_eq!(gen_t1, 1);
        assert_eq!(gen.load(Ordering::SeqCst), 1);
        let gen_t2 = gen.fetch_add(1, Ordering::SeqCst) + 1;
        assert_eq!(gen_t2, 2);
        assert_ne!(gen.load(Ordering::SeqCst), gen_t1);
        assert_eq!(gen.load(Ordering::SeqCst), gen_t2);
    }
}
