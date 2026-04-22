use esp_idf_hal::gpio::{AnyOutputPin, Output, PinDriver};
use esp_idf_hal::ledc::LedcDriver;
use log::{info, warn};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PumpType {
    NutrientA,
    NutrientB,
    PhUp,
    PhDown,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WaterDirection {
    In,
    Out,
    Stop,
}

pub struct PumpController {
    pump_a: LedcDriver<'static>,
    pump_b: LedcDriver<'static>,
    pump_ph_up: LedcDriver<'static>,
    pump_ph_down: LedcDriver<'static>,

    valve_mist: PinDriver<'static, Output>,

    // Bơm nước vào/ra chuyển sang Bật/Tắt (Digital Out)
    water_pump_in: PinDriver<'static, Output>,
    water_pump_out: PinDriver<'static, Output>,

    osaka_en: PinDriver<'static, Output>,
    osaka_rpwm: Arc<Mutex<LedcDriver<'static>>>,
    // osaka_lpwm: Arc<Mutex<LedcDriver<'static>>>,
    cancel_soft_start: Arc<AtomicBool>,
}

impl PumpController {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        mut pump_a: LedcDriver<'static>,
        mut pump_b: LedcDriver<'static>,
        mut pump_ph_up: LedcDriver<'static>,
        mut pump_ph_down: LedcDriver<'static>,
        mut valve_mist: PinDriver<'static, Output>,
        mut water_pump_in: PinDriver<'static, Output>,
        mut water_pump_out: PinDriver<'static, Output>,
        mut osaka_en: PinDriver<'static, Output>,
        mut osaka_rpwm: LedcDriver<'static>,
        // mut osaka_lpwm: LedcDriver<'static>,
    ) -> anyhow::Result<Self> {
        pump_a.set_duty(0)?;
        pump_b.set_duty(0)?;
        pump_ph_up.set_duty(0)?;
        pump_ph_down.set_duty(0)?;

        valve_mist.set_low()?;

        // Đổi khởi tạo bơm nước thành set_low
        water_pump_in.set_low()?;
        water_pump_out.set_low()?;

        osaka_en.set_low()?;
        osaka_rpwm.set_duty(0)?;
        // osaka_lpwm.set_duty(0)?;

        info!("✅ Đã khởi tạo PumpController (Bơm nước In/Out dạng Relay Bật/Tắt).");

        Ok(Self {
            pump_a,
            pump_b,
            pump_ph_up,
            pump_ph_down,
            valve_mist,
            water_pump_in,
            water_pump_out,
            osaka_en,
            osaka_rpwm: Arc::new(Mutex::new(osaka_rpwm)),
            // osaka_lpwm: Arc::new(Mutex::new(osaka_lpwm)),
            cancel_soft_start: Arc::new(AtomicBool::new(false)),
        })
    }

    pub fn set_dosing_pump_pwm(
        &mut self,
        pump: PumpType,
        state: bool,
        percent: u32,
    ) -> anyhow::Result<()> {
        let safe_percent = percent.clamp(0, 100);
        match pump {
            PumpType::NutrientA => {
                let max = self.pump_a.get_max_duty();
                self.pump_a.set_duty(if state {
                    (max as f32 * safe_percent as f32 / 100.0) as u32
                } else {
                    0
                })?;
            }
            PumpType::NutrientB => {
                let max = self.pump_b.get_max_duty();
                self.pump_b.set_duty(if state {
                    (max as f32 * safe_percent as f32 / 100.0) as u32
                } else {
                    0
                })?;
            }
            PumpType::PhUp => {
                let max = self.pump_ph_up.get_max_duty();
                self.pump_ph_up.set_duty(if state {
                    (max as f32 * safe_percent as f32 / 100.0) as u32
                } else {
                    0
                })?;
            }
            PumpType::PhDown => {
                let max = self.pump_ph_down.get_max_duty();
                self.pump_ph_down.set_duty(if state {
                    (max as f32 * safe_percent as f32 / 100.0) as u32
                } else {
                    0
                })?;
            }
        }
        Ok(())
    }

    pub fn set_pump_state(&mut self, pump: PumpType, state: bool) -> anyhow::Result<()> {
        self.set_dosing_pump_pwm(pump, state, 100)
    }

    pub fn set_water_pump(&mut self, direction: WaterDirection) -> anyhow::Result<()> {
        match direction {
            WaterDirection::In => {
                // Tắt bơm ra trước để đảm bảo an toàn, sau đó bật bơm vào
                self.water_pump_out.set_low()?;
                thread::sleep(Duration::from_millis(100)); // Delay nhỏ tránh sụt áp đột ngột
                self.water_pump_in.set_high()?;
            }
            WaterDirection::Out => {
                // Tắt bơm vào trước, sau đó bật bơm ra
                self.water_pump_in.set_low()?;
                thread::sleep(Duration::from_millis(100));
                self.water_pump_out.set_high()?;
            }
            WaterDirection::Stop => {
                // Tắt cả hai bơm
                self.water_pump_in.set_low()?;
                self.water_pump_out.set_low()?;
            }
        }
        Ok(())
    }

    pub fn set_mist_valve(&mut self, state: bool) -> anyhow::Result<()> {
        if state {
            self.valve_mist.set_high()?;
        } else {
            self.valve_mist.set_low()?;
        }
        Ok(())
    }

    pub fn start_osaka_pump_soft(&mut self, target_pwm_percent: u32) -> anyhow::Result<()> {
        info!(
            "🌊 Bắt đầu khởi động mềm bơm Osaka lên {}%...",
            target_pwm_percent
        );
        self.osaka_en.set_high()?;
        self.cancel_soft_start.store(false, Ordering::SeqCst);

        let rpwm_clone = Arc::clone(&self.osaka_rpwm);
        let cancel_flag = Arc::clone(&self.cancel_soft_start);
        let safe_percent = target_pwm_percent.clamp(0, 100);

        thread::spawn(move || {
            let max_duty = {
                let pump = rpwm_clone.lock().unwrap();
                pump.get_max_duty()
            };
            let target_duty = (max_duty as f32 * safe_percent as f32 / 100.0) as u32;
            let steps = 30;
            let step_delay = Duration::from_millis(100);

            for i in 1..=steps {
                if cancel_flag.load(Ordering::SeqCst) {
                    warn!("🛑 Đã hủy tiến trình khởi động mềm bơm Osaka!");
                    if let Ok(mut pump) = rpwm_clone.lock() {
                        let _ = pump.set_duty(0);
                    }
                    return;
                }
                let current_duty = target_duty * i / steps;
                if let Ok(mut pump) = rpwm_clone.lock() {
                    let _ = pump.set_duty(current_duty);
                }
                thread::sleep(step_delay);
            }
            info!("🌊 Bơm Osaka đã chạy ổn định ở {}%!", safe_percent);
        });
        Ok(())
    }

    pub fn set_osaka_pump_pwm(&mut self, duty_percent: u32) -> anyhow::Result<()> {
        if duty_percent == 0 {
            self.cancel_soft_start.store(true, Ordering::SeqCst);
            self.osaka_en.set_low()?;
            let mut pump = self.osaka_rpwm.lock().unwrap();
            pump.set_duty(0)?;
        } else {
            let percent = duty_percent.min(100);
            self.osaka_en.set_high()?;
            self.cancel_soft_start.store(true, Ordering::SeqCst);

            let mut pump = self.osaka_rpwm.lock().unwrap();
            let max_duty = pump.get_max_duty();
            let target_duty = ((max_duty as f32 * percent as f32) / 100.0) as u32;
            pump.set_duty(target_duty)?;
        }
        Ok(())
    }

    pub fn set_osaka_pump(&mut self, state: bool) -> anyhow::Result<()> {
        if state {
            self.set_osaka_pump_pwm(100)
        } else {
            self.set_osaka_pump_pwm(0)
        }
    }

    pub fn stop_all(&mut self) -> anyhow::Result<()> {
        warn!("🚨 CẢNH BÁO: Kích hoạt ngắt khẩn cấp toàn bộ hệ thống!");
        self.cancel_soft_start.store(true, Ordering::SeqCst);

        self.pump_a.set_duty(0)?;
        self.pump_b.set_duty(0)?;
        self.pump_ph_up.set_duty(0)?;
        self.pump_ph_down.set_duty(0)?;
        self.valve_mist.set_low()?;

        self.osaka_en.set_low()?;
        if let Ok(mut rpwm) = self.osaka_rpwm.lock() {
            let _ = rpwm.set_duty(0);
        }
        // if let Ok(mut lpwm) = self.osaka_lpwm.lock() {
        //     let _ = lpwm.set_duty(0);
        // }

        self.set_water_pump(WaterDirection::Stop)?;
        Ok(())
    }
}
