use crate::utility::types::DHCPMessage;
use crate::server_config::Config;
use std::net::Ipv4Addr;

//DHCP OPTIONS
pub const SUBNET_MASK: u8 = 1;
pub const TIME_OFFSET: u8 = 2;
pub const ROUTER: u8 = 3;
pub const TIME_SERVER: u8 = 4;
pub const NAME_SERVER: u8 = 5;
pub const DNS_SERVER: u8 = 6;
pub const LOG_SERVER: u8 = 7;
pub const COOKIE_SERVER: u8 = 8;
pub const LPR_SERVER: u8 = 9;
pub const IMPRESS_SERVER: u8 = 10;
pub const RESOURCE_LOCATION_SERVER: u8 = 11;
pub const HOST_NAME: u8 = 12;
pub const BOOT_FILE_SIZE: u8 = 13;
pub const MERIT_DUMP_FILE: u8 = 14;
pub const DOMAIN_NAME: u8 = 15;
pub const SWAP_SERVER: u8 = 16;
pub const ROOT_PATH: u8 = 17;
pub const EXTENSIONS_PATH: u8 = 18;

pub const BROADCAST_ADDRESS: u8 = 28;

pub const NETWORK_TIME_PROTOCOL_SERVERS: u8 = 42;

pub const REQUESTED_IP: u8 = 50;
pub const LEASE_TIME: u8 = 51;
pub const OPTION_OVERLOAD: u8 = 52;
pub const MESSAGE_TYPE: u8 = 53;
pub const SERVER_IDENTIFIER: u8 = 54;
pub const PARAMETER_REQUEST_LIST: u8 = 55;

pub const MAXIMUM_DHCP_MESSAGE_SIZE: u8 = 56;
pub const RENEWAL_TIME: u8 = 58;
pub const REBINDING_TIME: u8 = 59;
pub const CLIENT_IDENTIFIER: u8 = 61;

pub const END: u8 = 255;

//DHCP MESSAGE TYPE
pub const DHCPDISCOVER: u8 = 1;
pub const DHCPOFFER: u8 = 2;
pub const DHCPREQUEST: u8 = 3;
pub const DHCPDECLINE: u8 = 4;
pub const DHCPACK: u8 = 5;
pub const DHCPNAK: u8 = 6;
pub const DHCPRELEASE: u8 = 7;
pub const DHCPINFORM: u8 = 8;

 /*
 * FUNCTIONS FOR HANDLING OPTIONS VEC<U8> BUFFER
 */
pub fn create_options_buffer(message: &DHCPMessage, config: &Config, message_type: u8) -> Vec<u8> {
    let mut options_buf = Vec::new();
    options_buf.push(MESSAGE_TYPE);
    options_buf.push(1);
    options_buf.push(message_type);

    if message_type != DHCPNAK{
        options_buf.push(SUBNET_MASK);
        options_buf.push(4);
        options_buf.extend_from_slice(&config.server.subnet_mask.parse::<Ipv4Addr>().unwrap().octets());

        options_buf.push(ROUTER);
        options_buf.push(4);
        options_buf.extend_from_slice(&config.server.default_gateway.parse::<Ipv4Addr>().unwrap().octets());

        options_buf.push(DNS_SERVER);
        options_buf.push(4);
        options_buf.extend_from_slice(&config.server.dns_server.parse::<Ipv4Addr>().unwrap().octets());

        options_buf.push(DOMAIN_NAME);
        let domain_name = config.server.domain_name.clone().into_bytes();
        options_buf.push(domain_name.len() as u8);
        options_buf.extend_from_slice(&domain_name);

        if message.options_map.get(&53).and_then(|v| v.get(0)) != Some(&8){
            options_buf.push(LEASE_TIME);
            options_buf.push(4);
            options_buf.extend_from_slice(&config.server.lease_time.to_be_bytes());

            options_buf.push(RENEWAL_TIME);
            options_buf.push(4);
            options_buf.extend_from_slice(&config.server.renewal_time.to_be_bytes());
        }
    }

    options_buf.push(SERVER_IDENTIFIER);
    options_buf.push(4);
    options_buf.extend_from_slice(&config.server.ip_address.parse::<Ipv4Addr>().unwrap().octets());

    options_buf
}

pub fn inform_options_buf(parameter_request_list: Vec<u8>, config: &Config, client_mac: [u8; 16]) -> Vec<u8> {
    let mut options_buf = Vec::new();
    options_buf.push(SERVER_IDENTIFIER);
    options_buf.push(4);
    options_buf.extend_from_slice(&config.server.ip_address.parse::<Ipv4Addr>().unwrap().octets());
    options_buf.push(MESSAGE_TYPE);
    options_buf.push(1);
    options_buf.push(DHCPACK);

    for id in parameter_request_list {
        match id {
            SUBNET_MASK => {
                options_buf.push(SUBNET_MASK);
                options_buf.push(4);
                options_buf.extend_from_slice(
                &config.options_extended.subnet_mask.parse::<Ipv4Addr>().unwrap().octets());
            }
            TIME_OFFSET => {
                options_buf.push(TIME_OFFSET);
                options_buf.push(4);
                options_buf.extend_from_slice(&config.options_extended.time_offset.to_be_bytes());
            }
            ROUTER => {
                options_buf.push(ROUTER);
                options_buf.push(4 * config.options_extended.router.len() as u8);
                for router in &config.options_extended.router {
                options_buf.extend_from_slice(&router.parse::<Ipv4Addr>().unwrap().octets());
                }
            }
            TIME_SERVER => {
                options_buf.push(TIME_SERVER);
                options_buf.push(4 * config.options_extended.time_server.len() as u8);
                for time_server in &config.options_extended.time_server {
                options_buf.extend_from_slice(&time_server.parse::<Ipv4Addr>().unwrap().octets());
                }
            }
            NAME_SERVER => {
                options_buf.push(NAME_SERVER);
                options_buf.push(4 * config.options_extended.name_server.len() as u8);
                for name_server in &config.options_extended.name_server {
                options_buf.extend_from_slice(&name_server.parse::<Ipv4Addr>().unwrap().octets());
                }
            }
            DNS_SERVER => {
                options_buf.push(DNS_SERVER);
                options_buf.push(4 * config.options_extended.domain_name_server.len() as u8);
                for domain_name_server in &config.options_extended.domain_name_server {
                options_buf.extend_from_slice(&domain_name_server.parse::<Ipv4Addr>().unwrap().octets());
                }
            }
            LOG_SERVER => {
                options_buf.push(LOG_SERVER);
                options_buf.push(4 * config.options_extended.log_server.len() as u8);
                for log_server in &config.options_extended.log_server {
                options_buf.extend_from_slice(&log_server.parse::<Ipv4Addr>().unwrap().octets());
                }
            }
            COOKIE_SERVER => {
                options_buf.push(COOKIE_SERVER);
                options_buf.push(4 * config.options_extended.cookie_server.len() as u8);
                for cookie_server in &config.options_extended.cookie_server {
                options_buf.extend_from_slice(&cookie_server.parse::<Ipv4Addr>().unwrap().octets());
                }
            }
            LPR_SERVER => {
                options_buf.push(LPR_SERVER);
                options_buf.push(4 * config.options_extended.lpr_server.len() as u8);
                for lpr_server in &config.options_extended.lpr_server {
                options_buf.extend_from_slice(&lpr_server.parse::<Ipv4Addr>().unwrap().octets());
                }
            }
            IMPRESS_SERVER => {
                options_buf.push(IMPRESS_SERVER);
                options_buf.push(4 * config.options_extended.impress_server.len() as u8);
                for impress_server in &config.options_extended.impress_server {
                options_buf.extend_from_slice(&impress_server.parse::<Ipv4Addr>().unwrap().octets());
                }
            }
            RESOURCE_LOCATION_SERVER => {
                options_buf.push(RESOURCE_LOCATION_SERVER);
                options_buf.push(4 * config.options_extended.resource_location_server.len() as u8);
                for resource_location_server in &config.options_extended.resource_location_server {
                options_buf.extend_from_slice(&resource_location_server.parse::<Ipv4Addr>().unwrap().octets());
                }
            }
            BOOT_FILE_SIZE => {
                options_buf.push(BOOT_FILE_SIZE);
                options_buf.push(2);
                options_buf.extend_from_slice(&config.options_extended.boot_file_size.to_be_bytes());
            }
            MERIT_DUMP_FILE => {
                options_buf.push(MERIT_DUMP_FILE);
                let merit_dump_file = config.options_extended.merit_dump_file.clone().into_bytes();
                options_buf.push(merit_dump_file.len() as u8);
                options_buf.extend_from_slice(&merit_dump_file);
            }
            DOMAIN_NAME => {
                options_buf.push(DOMAIN_NAME);
                let domain_name = config.options_extended.domain_name.clone().into_bytes();
                options_buf.push(domain_name.len() as u8);
                options_buf.extend_from_slice(&domain_name);
            }
            SWAP_SERVER => {
                options_buf.push(SWAP_SERVER);
                let swap_server = config.options_extended.swap_server.clone().into_bytes();
                options_buf.push(swap_server.len() as u8);
                options_buf.extend_from_slice(&swap_server);
            }
            ROOT_PATH => {
                options_buf.push(ROOT_PATH);
                let root_path = config.options_extended.root_path.clone().into_bytes();
                options_buf.push(root_path.len() as u8);
                options_buf.extend_from_slice(&root_path);
            }
            EXTENSIONS_PATH => {
                options_buf.push(EXTENSIONS_PATH);
                let extensions_path = config.options_extended.extensions_path.clone().into_bytes();
                options_buf.push(extensions_path.len() as u8);
                options_buf.extend_from_slice(&extensions_path);
            }
            BROADCAST_ADDRESS => {
                options_buf.push(BROADCAST_ADDRESS);
                options_buf.push(4);
                options_buf.extend_from_slice(&config.options_extended.broadcast_address.parse::<Ipv4Addr>().unwrap().octets());
            }
            NETWORK_TIME_PROTOCOL_SERVERS => {
                options_buf.push(NETWORK_TIME_PROTOCOL_SERVERS);
                options_buf.push(4 * config.options_extended.network_time_protocol_servers.len() as u8);
                for network_time_protocol_server in &config.options_extended.network_time_protocol_servers {
                options_buf.extend_from_slice(&network_time_protocol_server.parse::<Ipv4Addr>().unwrap().octets());
            }
            }
            HOST_NAME => {
                options_buf.push(HOST_NAME);
                let mut host_name = client_mac.iter().map(|&c| format!("{:02x}", c)).collect::<Vec<String>>().join("");
                if host_name.len() >= 8 {
                host_name.truncate(host_name.len() - 20);
                }
                host_name = format!("user{}", host_name);
                options_buf.push(host_name.len() as u8);
                options_buf.extend_from_slice(&host_name.into_bytes());
            }
            /*
             * This match can be expanded to include other options as desired
             * To do so would require adding the option to the config.json, config type and here
             */
            _ => {},
        }
    }
    options_buf
}

//CHECK IF OPTIONS BUFFER IS TOO LARGE AND ADJUST
//OVERLOAD IF NECESSARY
pub fn adjust_options_buf(mut options_buf: Vec<u8>, max_message_size: u16,
                        file: &mut [u8; 128], sname: &mut [u8; 64]) -> Vec<u8> {
    let mut option_overload = 0;
    let required_size = max_message_size as usize - 236;

    if options_buf.len() > required_size {
        if options_buf.len() - required_size <= 64 {
            sname[..options_buf.len() - required_size].copy_from_slice(&options_buf[required_size..]);
            options_buf.truncate(required_size);
            option_overload = 2; 
        }
        else if options_buf.len() - required_size <= 128 {
            file[..options_buf.len() - required_size].copy_from_slice(&options_buf[required_size..]);
            options_buf.truncate(required_size);
            option_overload = 1;
        } 
        else if options_buf.len() - required_size <= 192 {
            let sname_end = required_size + 64;
            let file_end = required_size + 192;

            sname.copy_from_slice(&options_buf[required_size..sname_end]);
            file.copy_from_slice(&options_buf[sname_end..file_end]);
            options_buf.truncate(required_size);
            option_overload = 3;
        } 
        else {
            options_buf.truncate(required_size);
        }
    }

    if option_overload > 0 {
        options_buf.push(52);
        options_buf.push(1);
        options_buf.push(option_overload);
    }

    options_buf
}
