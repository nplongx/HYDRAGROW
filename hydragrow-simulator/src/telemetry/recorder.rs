use std::fs::File;
use std::io::Write;

pub struct Recorder {
    file: File,
}

impl Recorder {
    pub fn new(path: &str) -> Self {
        let mut file = File::create(path).unwrap();
        writeln!(file, "time,phase,ec,ph,temp,level,pump_a,pump_b").unwrap();
        Self { file }
    }

    pub fn record(&mut self, time: u64, phase: &str, ec: f32, ph: f32, temp: f32, level: f32, pa: bool, pb: bool) {
        writeln!(self.file, "{},{},{},{},{},{},{},{}", time, phase, ec, ph, temp, level, pa, pb).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_record_csv_line() {
        let mut recorder = Recorder::new("test_out.csv");
        recorder.record(100, "Idle", 1.2, 6.0, 25.0, 50.0, true, false);
        drop(recorder);
        let content = fs::read_to_string("test_out.csv").unwrap();
        assert!(content.contains("100,Idle,1.2,6,25,50,true,false"));
        fs::remove_file("test_out.csv").unwrap();
    }
}
