use std::str::FromStr;
use env_logger::Builder;
use log::Level;

pub fn init_logs(log_level: &String) {
    Builder::new()
        .filter_level(Level::from_str(log_level.as_str()).unwrap().to_level_filter())
        .init();
}