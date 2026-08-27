use esp_idf_svc::log::EspLogger;
use log::LevelFilter;

pub fn init(debug: bool) {
    EspLogger::initialize_default();
    let level = if debug {
        LevelFilter::Debug
    } else {
        LevelFilter::Info
    };
    log::set_max_level(level);
}
