use tokio_postgres::NoTls;
use dotenvy::dotenv;
use std::env;
use std::error::Error;
use std::sync::Arc;
use tokio::net::{UdpSocket};
use tokio_postgres::Client;
use tokio::time::{timeout, Duration};
use tokio::task;

mod utility;
mod server_config;
mod set_up;

use crate::utility::types::DHCPMessage;
use crate::set_up::create_db_tables::create_db;
use crate::server_config::{load_config, Config};

mod server;
use server::Server;

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

    if let Err(e) = create_db(&client).await {
        eprintln!("Error creating db: {}", e);
        return Ok(());
    }

    println!("Database created successfully");
    println!("Starting DHCP server");

    let dhcp_socket = UdpSocket::bind(("0.0.0.0", DHCP_SERVER_PORT)).await?;
    dhcp_socket.set_broadcast(true)?;
    println!("DHCP server listening on port {}", DHCP_SERVER_PORT);
    
    let config = load_config("app/server-config.json").expect("Failed to load configuration");

    let server = Server::new(config, dhcp_socket, client).await;
    Arc::new(server).start().await;

    return Ok(());
}