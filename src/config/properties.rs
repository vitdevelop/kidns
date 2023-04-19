use crate::util::Result;

static DNS_SERVER_PUBLIC: &str = "DNS_SERVER_PUBLIC";
static DNS_SERVER_PORT: &str = "DNS_SERVER_PORT";
static DNS_SERVER_HOST: &str = "DNS_SERVER_HOST";
pub static LOG_LEVEL: &str = "LOG_LEVEL";

pub struct Properties {
    pub dns_server_public: String,
    pub dns_server_port: u16,
    pub dns_server_host: String,
    pub log_level: String,
}

pub fn parse_properties() -> Result<Properties> {
    dotenv::from_filename("config.env").ok();

    return Ok(Properties {
        dns_server_public: get_env_var(DNS_SERVER_PUBLIC),
        dns_server_port: get_optional_env_var(DNS_SERVER_PORT, "53").parse::<u16>()?,
        dns_server_host: get_optional_env_var(DNS_SERVER_HOST, "0.0.0.0"),
        log_level: get_env_var(LOG_LEVEL),
    });
}

fn get_optional_env_var(var: &str, default: &str) -> String {
    return match std::env::var(var) {
        Ok(val) => val,
        Err(_) => default.to_string(),
    };
}

fn get_env_var(var: &str) -> String {
    return std::env::var(var)
        .expect((var.to_owned() + " var not defined").as_str());
}