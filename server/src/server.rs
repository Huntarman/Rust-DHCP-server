use crate::utility::types::DHCPMessage;
use crate::utility::options::*;
use crate::server_config::Config;

use tokio::net::UdpSocket;
use tokio::time::{sleep, timeout, Duration};
use tokio::task;

use tokio_postgres::Client;
use std::net::{Ipv4Addr, IpAddr};
use std::sync::Arc;

use crate::logger::Logger;

pub struct Server {
    config: Config,
    socket: UdpSocket,
    db: Client,
    logger: Logger,
}

impl Server {
    pub async fn new(config: Config, socket: UdpSocket, db: Client) -> Self {
        Server {
            config: config.clone(),
            socket,
            db,
            logger: Logger::new(&config.server.log_file),
        }
    }

    //STARTING THE SERVER
    pub async fn start(self: Arc<Self>) {
        let mut iterator = 0;

        let mut buf = vec![0; 1500];
        self.logger.log("[INFO] Server starting").await;
        loop {
            //TRYING TO RECEIVE RESPONSE FOR 60 SECONDS
            match timeout(Duration::from_secs(60), self.socket.recv_from(&mut buf)).await {
                Ok(Ok((size, addr))) => {
                    println!("Received {} bytes from {}", size, addr);
                    if let Ok(dhcp_message) = DHCPMessage::from_buffer(&buf[..size].to_vec()) {
                        let this = Arc::clone(&self);
                        task::spawn(
                            async move {
                                this.handle_message(dhcp_message, this.config.clone(), &this.db).await
                            }
                        );
                    } else {
                        println!("Failed to parse DHCP message.");
                    }
                }
                Ok(Err(e)) => eprintln!("Failed to receive data: {}", e),
                Err(_) => println!("Receive timed out"),
            }
        }
        sleep(Duration::from_millis(10)).await;
    }

    async fn handle_message(&self, dhcp_message: DHCPMessage, config: Config, db: &Client) {
        //UPDATE DATABASE BEFORE PROCESSING EVERY MESSAGE
        self.update_db(db).await;
        println!("Handling message: {:?}", dhcp_message);
        match dhcp_message.options_map.get(&MESSAGE_TYPE).and_then(|v| v.get(0)) {
            
            //DHCPDISCOVER
            Some(&DHCPDISCOVER) => {
                println!("Received DHCP Discover");
                if let Some(response) = self.build_offer_response(&dhcp_message, &config, &db).await {
                    println!("Sending DHCP Offer for address: {:?}", response.yiaddr);
                    self.logger.log(&format!("[INFO] DHCP Discover from client: {:?} offered IP address: {:?}", 
                    dhcp_message.chaddr.iter().map(|byte| format!("{:02X}", byte)).collect::<Vec<_>>().join(":")//MACADDR IN HEXADECIMAL
                    , response.yiaddr)).await;
                    self.send_response(response, &dhcp_message.ciaddr).await;
                }
            }

            //DHCPREQUEST
            Some(&DHCPREQUEST) => {
                println!("Received DHCP Request");
                self.logger.log(&format!("[INFO] DHCP Request from client: {:?}",
                dhcp_message.chaddr.iter().map(|byte| format!("{:02X}", byte)).collect::<Vec<_>>().join(":"))).await;
                if  self.should_nak(&dhcp_message, &config, &db).await {
                    if let Some(response) = self.build_nak_response(&dhcp_message, &config).await {
                        println!("Sending DHCP Nak");
                        self.logger.log(&format!("[INFO] Sending DHCP Nak to client: {:?}",
                        dhcp_message.chaddr.iter().map(|byte| format!("{:02X}", byte)).collect::<Vec<_>>().join(":"))).await;
                        self.send_response(response, &dhcp_message.ciaddr).await;
                    }
                    return;
                }
                if let Some(response) = self.build_ack_response_request(&dhcp_message, &config, &db).await {
                    println!("Sending DHCP Ack for address: {:?}", response.yiaddr);
                    self.logger.log(&format!("[INFO] DHCP Ack for client: {:?} for IP address: {:?}",
                    dhcp_message.chaddr.iter().map(|byte| format!("{:02X}", byte)).collect::<Vec<_>>().join(":"),
                    response.yiaddr)).await;
                    self.send_response(response, &dhcp_message.ciaddr).await;
                }
            }

            //DHCPDECLINE
            Some(&DHCPDECLINE) => {
                if !Server::for_this_server(&dhcp_message, &config) {return;} 
                println!("Received DHCP Decline");
                self.logger.log(&format!("[WARN] DHCP Decline from client: {:?}",
                dhcp_message.chaddr.iter().map(|byte| format!("{:02X}", byte)).collect::<Vec<_>>().join(":"))).await;
                if self.handle_decline(dhcp_message, db).await {
                    println!("Declined IP address successfully");
                    self.logger.log("[WARN] Declined IP address marked as unavailable for lease for an hour").await;
                }
                else {
                    eprintln!("Failed to decline IP address");
                    self.logger.log("[ERROR] Failed to handle decline message!!!").await;
                }
            }

            //DHCPRELEASE
            Some(&DHCPRELEASE) => {
                if !Server::for_this_server(&dhcp_message, &config) {return;} 
                println!("Received DHCP Release");
                if self.handle_release(dhcp_message, db).await {
                    println!("Released IP address successfully");
                    self.logger.log("[INFO] Released IP address marked as available for lease").await;
                }
                else {
                    eprintln!("Failed to decline IP address");
                    self.logger.log("[ERROR] Failed to handle release message!!!").await;
                }
            }

            //DHCPINFORM
            Some(&DHCPINFORM) => {
                println!("Received DHCP Inform");
                 if let Some(response) = self.build_ack_response_inform(&dhcp_message, &config).await {
                    println!("Sending DHCP Ack for DHCPINFORM");
                    self.logger.log(&format!("[INFO] DHCP Inform from client: {:?}",
                    dhcp_message.chaddr.iter().map(|byte| format!("{:02X}", byte)).collect::<Vec<_>>().join(":"))).await;
                    self.send_response(response, &dhcp_message.ciaddr).await;
                }
            }

            //DEFAULT
            _ => {
                println!("Received invalid DHCP message type");
                self.logger.log(&format!("[WARN] Server received invalid DHCP message type")).await;
            }
        }
    }

    //SENDING RESPONSE TO CLIENT
    async fn send_response(&self, response: DHCPMessage, addr: &Ipv4Addr) {
        //CHECK IF BROADCAST FLAG IS SET
        //CHECK IF CLIENT HAS IP ADDRESS
        //IF NOT SEND TO BROADCAST
        println!("Sending response to client: {:?}", response);
        let dest_addr: std::net::SocketAddr = if response.flags & 0x8000 != 0 {
            "255.255.255.255:68".parse().unwrap()
        } else if *addr == Ipv4Addr::new(0, 0, 0, 0) {
            "255.255.255.255:68".parse().unwrap()
        } else {
            //format!("{}:68", addr).parse().unwrap()
            "255.255.255.255:68".parse().unwrap()
        };

        let mut response_buffer = response.to_buffer();
        
        const MIN_DHCP_PAYLOAD_SIZE: usize = 548;
        //MINIMUM DHCP MESSAGE SIZE = 576 BYTES
        //INCLUDING IP AND UDP HEADERS
        //20 BYTES - IP HEADER
        //8 BYTES - UDP HEADER
        //MINIMUM DHCP PAYLOAD SIZE = 548 BYTES
        if response_buffer.len() < MIN_DHCP_PAYLOAD_SIZE {
            response_buffer.resize(MIN_DHCP_PAYLOAD_SIZE, 0);
        }
        
        let response_buffer = response.to_buffer();
        if let Err(e) = self.socket.send_to(&response_buffer, dest_addr).await {
            eprintln!("Failed to send DHCP message to {}: {}", dest_addr, e);
            self.logger.log(&format!("[ERROR] Failed to send DHCP message to {:?}: {}", dest_addr, e)).await;
        } else {
            println!("Sent DHCP message to {}", dest_addr);
        }
    }

    //UPDATE ADDRESSES TO CHECK IF SOME LEASES HAVE EXPIRED
    async fn update_db(&self, db: &Client) {
        println!("Updating database");
        let update_query = "UPDATE ip_addresses
                            SET allocated = false,
                                client_id = NULL,
                                lease_start = NULL,
                                lease_end = NULL
                            WHERE lease_end < NOW()";
        if let Err(e) = db.execute(update_query, &[]).await {
            eprintln!("Database query error: {}", e);
            self.logger.log(&format!("[ERROR] Database query error: {}", e)).await;
        }
    }

    //HANDLING BUILDING DHCPOFFER RESPONSE TO DHCPDISCOVER
    //SEARCH FOR FIRST AVAILABLE IP ADDRESS IN DATABASE
    //AND CREATE OFFER MESSAGE TO CLIENT
    //RETURNS MESSAGE
    async fn build_offer_response(&self, message: &DHCPMessage, config: &Config, db: &Client) -> Option<DHCPMessage> {
        
        //CHECK IF CLIENT REQUESTED SPECIFIC IP ADDRESS
        let requested_ip_address: IpAddr = IpAddr::V4(message.options_map.get(&REQUESTED_IP)
        .and_then(|v| if v.len() == 4 { Some([v[0], v[1], v[2], v[3]]) } else { None })
        .map(Ipv4Addr::from)
        .unwrap_or(Ipv4Addr::new(0, 0, 0, 0)));
        let check_requested_ip = "SELECT ip_address
                                FROM IP_addresses
                                WHERE ip_address = $1
                                AND allocated = false
                                LIMIT 1";
        let mut row = match db.query_opt(check_requested_ip, &[&requested_ip_address]).await {
            Ok(Some(row)) => Some(row),
            Ok(None) => None,
            Err(e) => {
                eprintln!("Database query error: {}", e);
                self.logger.log(&format!("[ERROR] Database query error: {}", e)).await;
                return None;
            }
        };

        //CHECK IF CLIENT HAS ALREADY ALLOCATED IP ADDRESS
        if row.is_none() {
            let search_client_id = "
            SELECT ip_address
            FROM IP_addresses
            WHERE client_id = $1
            LIMIT 1";
            let client_id: String = message.chaddr.iter().map(|&c| format!("{:02x}", c)).collect::<Vec<String>>().join("");
            row = match db.query_opt(search_client_id, &[&client_id]).await {
                Ok(Some(row)) => Some(row),
                Ok(None) => None,
                Err(e) => {
                    eprintln!("Database query error: {}", e);
                    self.logger.log(&format!("[ERROR] Database query error: {}", e)).await;
                    return None;
                }
            };
        }

        //OTHERWISE SEARCH FOR FIRST AVAILABLE IP ADDRESS
        if row.is_none() {
                row = match db
                .query_opt("SELECT ip_address FROM IP_addresses WHERE allocated = false LIMIT 1 FOR UPDATE", &[])
                .await
            {
                Ok(Some(row)) => Some(row),
                Ok(None) => None,
                Err(e) => {
                    eprintln!("Database query error: {}", e);
                    self.logger.log(&format!("[ERROR] Database query error: {}", e)).await;
                    return None;
                }
            };
        }
        if row.is_none() {
            eprintln!("No available IP addresses");
            return None;
        }
        
        let mut options_buf = create_options_buffer(message, config, DHCPOFFER);

        let ip_address_db: IpAddr = row?.get::<usize, IpAddr>(0);
        let ip_address: Ipv4Addr;
        match ip_address_db {
            IpAddr::V4(ipv4_addr) => ip_address = ipv4_addr,
            _ => {
                eprintln!("Invalid IP address type");
                return None;
            }
        }
        let bootstrap_server_ip = Ipv4Addr::new(0,0,0,0);

        let client_max_message_size = message.options_map.get(&MAXIMUM_DHCP_MESSAGE_SIZE);
        let max_message_size = match client_max_message_size {
            Some(v) if v.len() == 2 => {
                u16::from_be_bytes([v[0], v[1]])
            },
            _ => 1500,
        };
        let mut file = [0u8; 128];
        let mut sname = [0u8; 64];
        options_buf = adjust_options_buf(options_buf, max_message_size, &mut file, &mut sname);

        Some(DHCPMessage::new(
            2,
            message.htype,
            message.hlen,
            message.hops,
            message.xid,
            message.secs,
            message.flags,
            message.ciaddr,
            ip_address,
            bootstrap_server_ip,
            message.giaddr,
            message.chaddr,
            [0u8; 64],
            [0u8; 128],
            options_buf,
        ))
    }

    //HANDLING BUILDING DHCPACK RESPONSE TO DHCPREQUEST
    async fn build_ack_response_request(&self, message: &DHCPMessage, config: &Config, db: &Client) -> Option<DHCPMessage> {
        let mut options_buf = create_options_buffer(message, config, DHCPACK);

        let mut ip_address: Ipv4Addr = message.options_map.get(&REQUESTED_IP)
        .and_then(|v| if v.len() == 4 { Some([v[0], v[1], v[2], v[3]]) } else { None })
        .map(Ipv4Addr::from)
        .unwrap_or(Ipv4Addr::new(0, 0, 0, 0));
        
        let client_id: String = message.chaddr.iter().map(|&c| format!("{:02x}", c)).collect::<Vec<String>>().join("");

        let lease_duration: String = config.server.lease_time.to_string();

        let inet_type_ip: IpAddr;
        let query;
        let log_query;
        let log_message;
        //CHECK IF CLIENT WANTS TO EXTEND LEASE
        if ip_address == Ipv4Addr::new(0, 0, 0, 0) && message.ciaddr != Ipv4Addr::new(0, 0, 0, 0) {
            println!("Renewing IP address");
            inet_type_ip = IpAddr::V4(message.ciaddr);
            ip_address = message.ciaddr;
            query = "UPDATE ip_addresses
                    SET allocated = true,
                        client_id = $2,
                        lease_start = NOW(),
                        lease_end = NOW() + ($3 || ' seconds')::INTERVAL
                    WHERE ip_address = $1
                    AND client_id = $2";
            
            log_query = "INSERT INTO lease_history (ip_address,
                                                     client_id,
                                                      lease_start,
                                                       lease_end,
                                                        server_response,
                                                         lease_type)
                        VALUES ($1,
                                 $2,
                                  NOW(),
                                   NOW() + ($3 || ' seconds')::INTERVAL,
                                    'ACK',
                                     'RENEWING')";

            log_message = format!("[INFO] Renewing IP address {:?} for client {:?}", ip_address,
            message.chaddr.iter().map(|byte| format!("{:02X}", byte)).collect::<Vec<_>>().join(":"));
        }   
        //OTHERWISE LEASE THE REQUESTED IP ADDRESS
        else{
            if !Server::for_this_server(message, config) {return None;}
            println!("Leasing new IP address");
            inet_type_ip = IpAddr::V4(ip_address);
            query = "UPDATE ip_addresses
                    SET allocated = true,
                        client_id = $2,
                        lease_start = NOW(),
                        lease_end = NOW() + ($3 || ' seconds')::INTERVAL
                    WHERE ip_address = $1
                    AND allocated = false";
            
            log_query = "INSERT INTO lease_history (ip_address,
                                                     client_id,
                                                      lease_start,
                                                       lease_end,
                                                        server_response,
                                                         lease_type)
                        VALUES ($1,
                                 $2,
                                  NOW(),
                                   NOW() + ($3 || ' seconds')::INTERVAL,
                                    'ACK',
                                     'INITIAL')";
            log_message = format!("[INFO] Leasing new IP address {:?} for client {:?}", ip_address,
            message.chaddr.iter().map(|byte| format!("{:02X}", byte)).collect::<Vec<_>>().join(":"));
        }
        if let Err(e) = db.execute(query, &[&inet_type_ip, &client_id, &lease_duration]).await {
            eprintln!("Database query error: {}", e);
            self.logger.log(&format!("[ERROR] Database query error: {}", e)).await;
            return None;
        }
        if let Err(e) = db.execute(log_query, &[&inet_type_ip, &client_id, &lease_duration]).await {
            eprintln!("Database query error: {}", e);
            self.logger.log(&format!("[ERROR] Database query error: {}", e)).await;
            return None;
        }

        self.logger.log(&log_message).await;
        let client_max_message_size = message.options_map.get(&MAXIMUM_DHCP_MESSAGE_SIZE);
        let max_message_size = match client_max_message_size {
            Some(v) if v.len() == 2 => {
                u16::from_be_bytes([v[0], v[1]])
            },
            _ => 1500,
        };
        let mut file = [0u8; 128];
        let mut sname = [0u8; 64];
        options_buf = adjust_options_buf(options_buf, max_message_size, &mut file, &mut sname);

        Some(DHCPMessage::new(
            2,
            message.htype,
            message.hlen,
            message.hops,
            message.xid,
            message.secs,
            message.flags,
            message.ciaddr,
            ip_address,
            Ipv4Addr::new(0,0,0,0),
            message.giaddr,
            message.chaddr,
            [0u8; 64],
            [0u8; 128],
            options_buf,
        ))
    }

    //BUILDING DHCPACK RESPONSE TO DHCPINFORM
    async fn build_ack_response_inform(&self, message: &DHCPMessage, config: &Config) -> Option<DHCPMessage> {
        //IF CLIENT DID NOT REQUEST ANY PARAMETERS
        //SEND SOME DEFAULT PARAMETERS
        if message.options_map.get(&PARAMETER_REQUEST_LIST).is_none() {
            let mut options_buf = create_options_buffer(message, config, DHCPACK);
            let mut file = [0u8; 128];
            let mut sname = [0u8; 64];
            let client_max_message_size = message.options_map.get(&MAXIMUM_DHCP_MESSAGE_SIZE);
            let max_message_size = match client_max_message_size {
                Some(v) if v.len() == 2 => {
                    u16::from_be_bytes([v[0], v[1]])
                },
                _ => 1500,
            };
            options_buf = adjust_options_buf(options_buf, max_message_size, &mut file, &mut sname);
            Some(DHCPMessage::new(
                2,
                message.htype,
                message.hlen,
                0,
                message.xid,
                0,
                message.flags,
                message.ciaddr,
                message.yiaddr,
                Ipv4Addr::new(0,0,0,0),
                message.giaddr,
                message.chaddr,
                [0u8; 64],
                [0u8; 128],
                options_buf,
            ))
        }
        //OTHERWISE SEND THE REQUESTED PARAMETERS
        else{
            let mut options_buf = inform_options_buf(
                message.options_map.get(&PARAMETER_REQUEST_LIST).unwrap().to_vec(),
                config,
                message.chaddr
            );
            let client_max_message_size = message.options_map.get(&MAXIMUM_DHCP_MESSAGE_SIZE);
            let max_message_size = match client_max_message_size {
                Some(v) if v.len() == 2 => {
                    u16::from_be_bytes([v[0], v[1]])
                },
                _ => 1500,
            };
            let mut file = [0u8; 128];
            let mut sname = [0u8; 64];
            options_buf = adjust_options_buf(options_buf, max_message_size, &mut file, &mut sname);

            Some(DHCPMessage::new(
                2,
                message.htype,
                message.hlen,
                0,
                message.xid,
                0,
                message.flags,
                message.ciaddr,
                message.yiaddr,
                Ipv4Addr::new(0,0,0,0),
                message.giaddr,
                message.chaddr,
                [0u8; 64],
                [0u8; 128],
                options_buf,
            ))
        }
    }

    //BUILDING DHCPNAK RESPONSE TO DHCPREQUEST
    async fn build_nak_response(&self, message: &DHCPMessage, config: &Config) -> Option<DHCPMessage> {
        let requested_ip = match message.options_map.get(&REQUESTED_IP) {
            Some(v) if v.len() == 4 => {
                Ipv4Addr::new(v[0], v[1], v[2], v[3])
            },
            _ => message.ciaddr,
        };
        let log_nak = "INSERT INTO lease_history (ip_address,
                                                  client_id,
                                                   lease_start,
                                                    lease_end,
                                                     server_response,
                                                      lease_type)
                    VALUES ($1,
                            $2,
                             NULL,
                              NULL,
                               'NAK',
                                'DECLINED')";
        let client_id: String = message.chaddr.iter().map(|&c| format!("{:02x}", c)).collect::<Vec<String>>().join("");
        if let Err(e) = self.db.execute(log_nak, &[&IpAddr::V4(requested_ip), &client_id]).await {
            eprintln!("Database query error: {}", e);
            self.logger.log(&format!("[ERROR] Database query error: {}", e)).await;
            return None;
        }
        let options_buf = create_options_buffer(message, config, DHCPNAK);
        Some(DHCPMessage::new(
            2,
            message.htype,
            message.hlen,
            0,
            message.xid,
            0,
            message.flags,
            Ipv4Addr::new(0, 0, 0, 0),
            Ipv4Addr::new(0, 0, 0, 0),
            Ipv4Addr::new(0, 0, 0, 0),
            message.giaddr,
            message.chaddr,
            [0u8; 64],
            [0u8; 128],
            options_buf,
        ))
    }

    //HANDLING DECLINE MESSAGE
    //NO RESPONSE NECESSARY
    async fn handle_decline(&self, message: DHCPMessage, db: &Client) -> bool {
        let declined_ip_address: IpAddr = IpAddr::V4(message.options_map.get(&REQUESTED_IP)
        .and_then(|v| if v.len() == 4 { Some([v[0], v[1], v[2], v[3]]) } else { None })
        .map(Ipv4Addr::from)
        .unwrap_or(Ipv4Addr::new(0, 0, 0, 0)));
        
        let query = "UPDATE ip_addresses
                    SET allocated = true,
                        client_id = NULL,
                        lease_start = NOW(),
                        lease_end = NOW() + '1 hour'::INTERVAL
                    WHERE ip_address = $1";
        
        if let Err(e) = db.execute(query, &[&declined_ip_address]).await {
            eprintln!("Database query error: {}", e);
            self.logger.log(&format!("[ERROR] Database query error: {}", e)).await;
            return false;
        }
        self.logger.log(&format!("[WARN] IP address {:?} declined", declined_ip_address)).await;
        return true;
    }

    //HANDLING RELEASE MESSAGE
    //NO RESPONSE NECESSARY
    async fn handle_release(&self, message: DHCPMessage, db: &Client) -> bool {
        let released_ip_address: IpAddr = IpAddr::V4(message.options_map.get(&REQUESTED_IP)
        .and_then(|v| if v.len() == 4 { Some([v[0], v[1], v[2], v[3]]) } else { None })
        .map(Ipv4Addr::from)
        .unwrap_or(Ipv4Addr::new(0, 0, 0, 0)));

        let query = "UPDATE ip_addresses
                    SET allocated = false,
                        client_id = NULL,
                        lease_start = NULL,
                        lease_end = NULL
                    WHERE ip_address = $1";
        
        if let Err(e) = db.execute(query, &[&released_ip_address]).await {
            eprintln!("Database query error: {}", e);
            self.logger.log(&format!("[ERROR] Database query error: {}", e)).await;
            return false;
        }
        self.logger.log(&format!("[INFO] IP address {:?} released", released_ip_address)).await;
        return true;
    }


    /*
     FUNCTIONS FOR CONTROLLING THE SERVER BEHAVIOR
     */
    async fn should_nak(&self, message: &DHCPMessage, config: &Config, db: &Client) -> bool {
        let mut requested_ip = match message.options_map.get(&REQUESTED_IP) {
            Some(v) if v.len() == 4 => {
                Ipv4Addr::new(v[0], v[1], v[2], v[3])
            },
            _ => Ipv4Addr::new(0, 0, 0, 0),
        };

        let renewing: bool = (requested_ip == Ipv4Addr::new(0, 0, 0, 0) && message.ciaddr != Ipv4Addr::new(0, 0, 0, 0));
        if requested_ip == Ipv4Addr::new(0, 0, 0, 0) && !renewing {
            println!("Client requested lease of IP address without requested IP option");
            self.logger.log(&format!("[INFO] Client {:?} requested lease of IP address without requested IP option",
            message.chaddr.iter().map(|byte| format!("{:02X}", byte)).collect::<Vec<_>>().join(":"))).await;
            return true;
        }

        requested_ip = if requested_ip == Ipv4Addr::new(0, 0, 0, 0) {message.ciaddr} else {requested_ip};

        if (u32::from(requested_ip) < u32::from(config.ip_pool.range_start.parse::<Ipv4Addr>().unwrap()) ||
            u32::from(requested_ip) > u32::from(config.ip_pool.range_end.parse::<Ipv4Addr>().unwrap())) {
            println!("Requested IP is outside the server's pool");
            self.logger.log(&format!("[INFO] Client {:?} requested lease of IP address outside the server's pool",
            message.chaddr.iter().map(|byte| format!("{:02X}", byte)).collect::<Vec<_>>().join(":"))).await;
            return true;
        }

        if config.restricted_ips.contains(&requested_ip.to_string()) {
            println!("Requested IP is restricted");
            return true;
        }
        let client_id: String = message.chaddr.iter().map(|&c| format!("{:02x}", c)).collect::<Vec<String>>().join("");
        
        let query = "SELECT ip_address
                    FROM ip_addresses
                    WHERE ip_address = $1
                    AND (allocated = false
                    OR client_id = $2)
                    LIMIT 1";
        
        let _row = match db.query_opt(query, &[&IpAddr::V4(requested_ip), &client_id]).await {
            Ok(Some(_row)) => Some(_row),
            Ok(None) => {
                println!("Requested IP is already allocated");
                self.logger.log(&format!("[INFO] Client {:?} requested lease of IP address that was already allocated",
                message.chaddr.iter().map(|byte| format!("{:02X}", byte)).collect::<Vec<_>>().join(":"))).await;
                return true;
            }
            Err(e) => {
                eprintln!("Database query error: {}", e);
                self.logger.log(&format!("[ERROR] Database query error: {}", e)).await;
                return true;
            }
        };

        if let Some(server_identifier) = message.options_map.get(&SERVER_IDENTIFIER) {
            let server_ip = &config.server.ip_address.parse::<Ipv4Addr>().unwrap().octets();
            if server_identifier != &server_ip {
                println!("Mismatched Server Identifier");
                self.logger.log(&format!("[INFO] Client {:?} requested lease of IP from a different server",
                message.chaddr.iter().map(|byte| format!("{:02X}", byte)).collect::<Vec<_>>().join(":"))).await;
                return true;
            }
        } else if !renewing {
            println!("Server Identifier not present in options");
            self.logger.log(&format!("[INFO] Client {:?} requested lease of IP address without Server Identifier option",
            message.chaddr.iter().map(|byte| format!("{:02X}", byte)).collect::<Vec<_>>().join(":"))).await;
            return true;
        }

        false
    }   

    fn for_this_server(message: &DHCPMessage, config: &Config) -> bool {
        if let Some(server_identifier) = message.options_map.get(&SERVER_IDENTIFIER) {
            let server_ip = &config.server.ip_address.parse::<Ipv4Addr>().unwrap().octets();
            if server_identifier != &server_ip {
                println!("Mismatched Server Identifier");
                return false;
            }
            return true;
        } else {
            println!("Server Identifier not present in options");
            return false;
        }
    }
}