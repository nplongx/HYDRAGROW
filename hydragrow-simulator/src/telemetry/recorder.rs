use std::fs::File;
use std::io::{self, Write};

pub struct Recorder {
    file: File,
}

impl Recorder {
    pub fn new(path: &str) -> io::Result<Self> {
        let mut file = File::create(path)?;
        writeln!(file, "time,phase,ec,ph,temp,level,pump_a,pump_b")?;
        Ok(Self { file })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn record(
        &mut self,
        time: u64,
        phase: &str,
        ec: f32,
        ph: f32,
        temp: f32,
        level: f32,
        pa: bool,
        pb: bool,
    ) -> io::Result<()> {
        writeln!(
            self.file,
            "{},{},{},{},{},{},{},{}",
            time, phase, ec, ph, temp, level, pa, pb
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_record_csv_line() {
        let mut recorder = Recorder::new("test_out.csv").unwrap();
        recorder
            .record(100, "Idle", 1.2, 6.0, 25.0, 50.0, true, false)
            .unwrap();
        drop(recorder);
        let content = fs::read_to_string("test_out.csv").unwrap();
        assert!(content.contains("100,Idle,1.2,6,25,50,true,false"));
        fs::remove_file("test_out.csv").unwrap();
    }

    #[test]
    fn recorder_returns_io_errors_instead_of_panicking() {
        let result = Recorder::new("/definitely/missing/dir/out.csv");
        assert!(result.is_err());
    }
}
