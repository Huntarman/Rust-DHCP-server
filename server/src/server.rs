use crate::utility::types::DHCPMessage;
use crate::server_config::Config;

use tokio::net::UdpSocket;
use tokio::time::{timeout, Duration};
use tokio::task;

use tokio_postgres::Client;
use std::net::{Ipv4Addr, IpAddr};
use std::sync::Arc;

pub struct Server {
    config: Config,
    socket: UdpSocket,
    db: Client,
}

impl Server {
    pub async fn new(config: Config, socket: UdpSocket, db: Client) -> Self {
        Server {
            config,
            socket,
            db,
        }
    }

    pub async fn start(self: Arc<Self>) {
        let mut buf = vec![0; 1500];
        loop {
            match timeout(Duration::from_secs(10), self.socket.recv_from(&mut buf)).await {
                Ok(Ok((size, addr))) => {
                    println!("Received {} bytes from {}", size, addr);
                    let dhcp_packet = buf[..size].to_vec();
                    if let Ok(dhcp_message) = DHCPMessage::from_buffer(&dhcp_packet) {
                        let this = Arc::clone(&self);
                        task::spawn(
                            async move {
                                println!("Received DHCP packet: {:?}", dhcp_message);
                                this.handle_message(dhcp_message, addr, this.config.clone(), &this.db).await
                            }
                        );
                    } else {
                        println!("Failed to parse DHCP message");
                    }
                }
                Ok(Err(e)) => eprintln!("Failed to receive data: {}", e),
                Err(_) => println!("Receive timed out"),
            }
        }
    }

    async fn handle_message(&self, dhcp_message: DHCPMessage, addr: std::net::SocketAddr, config: Config, db: &Client) {
        self.update_db(db).await;
        match dhcp_message.options_map.get(&53).and_then(|v| v.get(0)) {
            Some(1) => {
                println!("Received DHCP Discover");
                if let Some(response) = Server::build_offer_response(&dhcp_message, &config, &db).await {
                    println!("Sending DHCP Offer: {:?}", response);
                    self.send_response(response, &addr).await;
                }
            }
            Some(3) => {
                println!("Received DHCP Request");
                if let Some(response) = Server::build_ack_response_request(&dhcp_message, &config, &db).await {
                    println!("Sending DHCP Ack: {:?}", response);
                    self.send_response(response, &addr).await;
                }
            }
            Some(4) => println!("Received DHCP Decline"),
            Some(7) => println!("Received DHCP Release"),
            Some(8) => println!("Received DHCP Inform"),
            _ => println!("Received invalid DHCP message type"),
        }
    }

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
        }
    }

    async fn build_offer_response(message: &DHCPMessage, config: &Config, db: &Client) -> Option<DHCPMessage> {
        
        let mut requested_ip_address: IpAddr = IpAddr::V4(message.options_map.get(&50)
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
                return None;
            }
        };

        if row.is_none() {
            let search_client_id = "
            SELECT ip_address
            FROM IP_addresses
            WHERE client_id = $1
            LIMIT 1";
            let client_id: String = message.chaddr.iter().map(|&c| format!("{:02x}", c)).collect::<Vec<String>>().join("");
            row = match db.query_opt(search_client_id, &[&client_id]).await {
                Ok(Some(row)) => Some(row),
                Ok(None) => None, // No matching row found
                Err(e) => {
                    eprintln!("Database query error: {}", e);
                    return None;
                }
            };
        }

        if row.is_none() {
                row = match db
                .query_opt("SELECT ip_address FROM IP_addresses WHERE allocated = false LIMIT 1", &[])
                .await
            {
                Ok(Some(row)) => Some(row),
                Ok(None) => None, // No matching row found
                Err(e) => {
                    eprintln!("Database query error: {}", e);
                    return None;
                }
            };
        }
        if row.is_none() {
            eprintln!("No available IP addresses");
            return None;
        }
        
        let mut options_buf = Self::create_options_buffer(message, config, 2);

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

    async fn build_ack_response_request(message: &DHCPMessage, config: &Config, db: &Client) -> Option<DHCPMessage> {
        let mut options_buf = Self::create_options_buffer(message, config, 5);

        let mut ip_address: Ipv4Addr = message.options_map.get(&50)
        .and_then(|v| if v.len() == 4 { Some([v[0], v[1], v[2], v[3]]) } else { None })
        .map(Ipv4Addr::from)
        .unwrap_or(Ipv4Addr::new(0, 0, 0, 0));
        
        let client_id: String = message.chaddr.iter().map(|&c| format!("{:02x}", c)).collect::<Vec<String>>().join("");

        let lease_duration: String = config.server.lease_time.to_string();

        let bootstrap_server_ip = Ipv4Addr::new(0,0,0,0);

        let mut inet_type_ip: IpAddr;

        if ip_address == Ipv4Addr::new(0, 0, 0, 0) && message.ciaddr != Ipv4Addr::new(0, 0, 0, 0) {
            eprintln!("Renewing IP address");
            inet_type_ip = IpAddr::V4(message.ciaddr);
            ip_address = message.ciaddr;
        }
        else{
            eprintln!("Allocating new IP address");
            inet_type_ip = IpAddr::V4(ip_address);
        }
        let query = "UPDATE ip_addresses
                    SET allocated = true,
                        client_id = $2,
                        lease_start = NOW(),
                        lease_end = NOW() + ($3 || ' seconds')::INTERVAL
                    WHERE ip_address = $1";
        if let Err(e) = db.execute(query, &[&inet_type_ip, &client_id, &lease_duration]).await {
            eprintln!("Database query error: {}", e);
            return None;
        }

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

    async fn send_response(&self, response: DHCPMessage, addr: &std::net::SocketAddr) {
        let dest_addr: std::net::SocketAddr = if response.flags & 0x8000 != 0 {
            "255.255.255.255:68".parse().unwrap()
        } else {
            "255.255.255.255:68".parse().unwrap()
        };

        let response_buffer = response.to_buffer();
        if let Err(e) = self.socket.send_to(&response_buffer, dest_addr).await {
            eprintln!("Failed to send DHCP message to {}: {}", dest_addr, e);
        } else {
            println!("Sent DHCP message to {}", dest_addr);
        }
    }

    async fn handle_decline(&self, message: DHCPMessage, db: &Client) {
        let allocated_ip: IpAddr = IpAddr::V4(message.yiaddr);
        let query = "UPDATE ip_addresses
                    SET allocated = true,
                        client_id = NULL,
                        lease_start = NOW(),
                        lease_end = NOW() + '1 hour'::INTERVAL
                    WHERE ip_address = $1";
        if let Err(e) = db.execute(query, &[&allocated_ip]).await {
            eprintln!("Database query error: {}", e);
        }
    }

    fn create_options_buffer (message: &DHCPMessage, config: &Config, message_type: u8) -> Vec<u8> {
        let mut options_buf = Vec::new();

        if message_type != 6{
            options_buf.push(53);
            options_buf.push(1);
            options_buf.push(message_type);

            options_buf.push(1);
            options_buf.push(4);
            options_buf.extend_from_slice(&config.server.subnet_mask.parse::<Ipv4Addr>().unwrap().octets());

            options_buf.push(3);
            options_buf.push(4);
            options_buf.extend_from_slice(&config.server.default_gateway.parse::<Ipv4Addr>().unwrap().octets());

            options_buf.push(6);
            options_buf.push(4);
            options_buf.extend_from_slice(&config.server.dns_server.parse::<Ipv4Addr>().unwrap().octets());

            options_buf.push(15);
            let domain_name = config.server.domain_name.clone().into_bytes();
            options_buf.push(domain_name.len() as u8);
            options_buf.extend_from_slice(&domain_name);

            if message.options_map.get(&53).and_then(|v| v.get(0)) != Some(&8){
                options_buf.push(51);
                options_buf.push(4);
                options_buf.extend_from_slice(&config.server.lease_time.to_be_bytes());

                options_buf.push(58);
                options_buf.push(4);
                options_buf.extend_from_slice(&config.server.renewal_time.to_be_bytes());
            }
        }

        options_buf.push(54);
        options_buf.push(4);
        options_buf.extend_from_slice(&config.server.ip_address.parse::<Ipv4Addr>().unwrap().octets());

        options_buf
    }
}
