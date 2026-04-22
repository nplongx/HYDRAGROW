use esp_idf_hal::delay::Ets;
use esp_idf_hal::gpio::{Input, Output, PinDriver};

pub struct JsnSr04t<'a> {
    trig: PinDriver<'a, Output>,
    echo: PinDriver<'a, Input>,
}

impl<'a> JsnSr04t<'a> {
    pub fn new(
        mut trig: PinDriver<'a, Output>,
        echo: PinDriver<'a, Input>,
    ) -> anyhow::Result<Self> {
        trig.set_low()?;
        Ok(Self { trig, echo })
    }

    pub fn get_distance_cm(&mut self) -> Option<f32> {
        self.trig.set_high().ok()?;
        Ets::delay_us(20);
        self.trig.set_low().ok()?;

        let mut timeout = 0;
        while self.echo.is_low() {
            Ets::delay_us(1);
            timeout += 1;
            if timeout > 30000 {
                return None;
            }
        }

        let mut duration = 0;
        while self.echo.is_high() {
            Ets::delay_us(1);
            duration += 1;
            if duration > 30000 {
                return None;
            }
        }

        let distance = (duration as f32) * 0.0343 / 2.0;
        if (2.0..=400.0).contains(&distance) {
            Some(distance)
        } else {
            None
        }
    }
}
