use env_logger::Builder;

pub fn init_logs() {
    let mut builder = Builder::from_env(super::properties::LOG_LEVEL);
    builder.init();
}