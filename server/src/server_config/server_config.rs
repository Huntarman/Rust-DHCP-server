use serde::Deserialize;
use std::fs;
use std::error::Error;

#[derive(Deserialize, Clone)]
pub struct Config {
    pub server: ServerConfig,
    pub ip_pool: IpPoolConfig,
    pub restricted_ips: Vec<String>,
    pub options_extended: ExtendedConfig,
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
    pub log_file: String,
}

#[derive(Deserialize, Clone)]
pub struct IpPoolConfig {
    pub range_start: String,
    pub range_end: String,
}

#[derive(Deserialize, Clone)]
pub struct ExtendedConfig {
    pub subnet_mask: String,
    pub time_offset: u32,
    pub router: Vec<String>,
    pub time_server: Vec<String>,
    pub name_server: Vec<String>,
    pub domain_name_server: Vec<String>,
    pub log_server: Vec<String>,
    pub cookie_server: Vec<String>,
    pub lpr_server: Vec<String>,
    pub impress_server: Vec<String>,
    pub resource_location_server: Vec<String>,
    pub boot_file_size: u16,
    pub merit_dump_file: String,
    pub domain_name: String,
    pub swap_server: String,
    pub root_path: String,
    pub extensions_path: String,
    pub broadcast_address: String,
    pub network_time_protocol_servers: Vec<String>,
    //EXTEND HERE IF NEEDED
}

pub fn load_config(path: &str) -> Result<Config, Box<dyn Error>> {
    let file_content = fs::read_to_string(path)?;
    let config: Config = serde_json::from_str(&file_content)?;
    Ok(config)
}
