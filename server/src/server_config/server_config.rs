use serde::Deserialize;
use std::fs;
use std::error::Error;

#[derive(Deserialize, Clone)]
pub struct Config {
    pub server: ServerConfig,
    pub ip_pool: IpPoolConfig,
    pub restricted_ips: Vec<String>,
}

#[derive(Deserialize, Clone)]
pub struct ServerConfig {
    pub lease_time: u32,
    pub renewal_time: u32,
    pub subnet_mask: String,
    pub default_gateway: String,
    pub dns_server: String,
    pub domain_name: String,
    pub ip_address: String,
}

#[derive(Deserialize, Clone)]
pub struct IpPoolConfig {
    pub range_start: String,
    pub range_end: String,
}

pub fn load_config(path: &str) -> Result<Config, Box<dyn Error>> {
    let file_content = fs::read_to_string(path)?;
    let config: Config = serde_json::from_str(&file_content)?;
    Ok(config)
}
