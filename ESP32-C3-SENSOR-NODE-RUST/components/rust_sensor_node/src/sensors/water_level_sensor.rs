use crate::ffi;

pub struct WaterLevelSensor {
    tank_height_cm: f32,
    last_distance: f32,
    last_level: f32,
}

impl WaterLevelSensor {
    pub fn new(tank_height_cm: f32) -> Self {
        Self {
            tank_height_cm,
            last_distance: f32::NAN,
            last_level: f32::NAN,
        }
    }

    pub fn set_tank_height(&mut self, h: f32) {
        self.tank_height_cm = h;
    }

    pub fn last_distance(&self) -> f32 {
        self.last_distance
    }
    pub fn last_level(&self) -> f32 {
        self.last_level
    }

    pub fn read(&mut self) -> f32 {
        let dist = unsafe { ffi::hcsr04_read_cm() };
        if dist == 0.0 {
            self.last_distance = f32::NAN;
            self.last_level = f32::NAN;
            return f32::NAN;
        }
        self.last_distance = dist;
        let level = (self.tank_height_cm - dist).max(0.0);
        self.last_level = level;
        level
    }
}
