mod create_db_tables;
use tokio_postgres::NoTls;
use dotenvy::dotenv;
use std::env;
use std::error::Error;
use tokio::net::{UdpSocket};
use tokio::time::{timeout, Duration};
use tokio::task;

const DHCP_SERVER_PORT: u16 = 67;
const DHCP_CLIENT_PORT: u16 = 68;
const MAX_BUFFER_SIZE: usize = 1500;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();
    let db_url = env::var("POSTGRES_URL").expect("Failed to find POSTGRES_URL");
    let (client, connection) = tokio_postgres::connect(&db_url, NoTls).await?;
    
    tokio::spawn(async move{
        if let Err(e) = connection.await {
            eprintln!("Connection error: {}", e);
        }
    });
    println!("Connected to database");

    if let Err(e) = create_db_tables::create_db(&client).await {
        eprintln!("Error creating db: {}", e);
        return Ok(());
    }

    println!("Database created successfully");
    println!("Starting DHCP server");

    let dhcp_socket = UdpSocket::bind(("0.0.0.0", DHCP_SERVER_PORT)).await?;
    dhcp_socket.set_broadcast(true)?;
    println!("DHCP server listening on port {}", DHCP_SERVER_PORT);
    
    let mut buf = [0u8; MAX_BUFFER_SIZE];
    
    loop {
        match timeout(Duration::from_secs(10), dhcp_socket.recv_from(&mut buf)).await {
            Ok(Ok((size, addr))) => {
                println!("Received {} bytes from {}", size, addr);
                let dhcp_packet = buf[..size].to_vec();
                let cloned_packet = dhcp_packet.clone();
                task::spawn( async move { 
                    match cloned_packet[0] {
                        1 => println!("Received DHCP Discover"),
                        3 => println!("Received DHCP Request"),
                        7 => println!("Received DHCP Release"),
                        8 => println!("Received DHCP Inform"),
                        _ => println!("Received not valid DHCP message type")
                    }
                }
                 );

                if let Err(e) = dhcp_socket.send_to(&dhcp_packet, &addr).await {
                    eprintln!("Failed to send data to {}: {}", addr, e);
                } else {
                    println!("Sent message back to {}", addr);
                }
            },
            Ok(Err(e)) => eprintln!("Failed to receive data: {}", e),
            Err(_) => println!("Receive timed out"),
        }
    }

    return Ok(());
}