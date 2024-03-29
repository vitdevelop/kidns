use env_logger::Builder;
use log::Level;
use std::str::FromStr;

pub fn init_logs(log_level: &String) {
    let mut builder = Builder::new();
    builder.filter_level(
        Level::from_str(log_level.as_str())
            .unwrap()
            .to_level_filter(),
    );

    if cfg!(not(debug_assertions)) {
        builder.format_target(false);
    }

    builder.init();
}
