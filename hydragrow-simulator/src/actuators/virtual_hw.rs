#[derive(Debug, Clone, Default)]
pub struct VirtualPump {
    pub on: bool,
    pub pwm: u8,
}

impl VirtualPump {
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Debug, Clone, Default)]
pub struct VirtualHardwareState {
    pub pump_a: VirtualPump,
    pub pump_b: VirtualPump,
    pub pump_ph_up: VirtualPump,
    pub pump_ph_down: VirtualPump,
    pub water_pump_in: VirtualPump,
    pub water_pump_out: VirtualPump,
    pub mist_valve: bool,
    pub osaka_pwm: u8,
}
