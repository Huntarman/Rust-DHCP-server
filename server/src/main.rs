use tokio_postgres::NoTls;
use dotenvy::dotenv;
use std::env;
use std::error::Error;
use std::sync::Arc;
use tokio::net::{UdpSocket};

mod utility;
mod server_config;
mod set_up;

use crate::set_up::create_db_tables::create_db;
use crate::server_config::{load_config};

mod logger;
mod server;
use server::Server;

const DHCP_SERVER_PORT: u16 = 67;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    //SET UP ENV
    dotenv().ok();
    let db_url = env::var("POSTGRES_URL").expect("Failed to find POSTGRES_URL");
    let (client, connection) = tokio_postgres::connect(&db_url, NoTls).await?;
    
    println!("Detected local timezone: {}", chrono::Local::now());
    
    //SET UP DATABASE CONNECTION
    tokio::spawn(async move{
        if let Err(e) = connection.await {
            eprintln!("Connection error: {}", e);
        }
    });
    println!("Connected to database");

    //CREATE DATABASE TABLES
    if let Err(e) = create_db(&client).await {
        eprintln!("Error creating db: {}", e);
        return Ok(());
    }

    println!("Database created successfully");
    println!("Starting DHCP server");

    //OPEN SOCKET
    let dhcp_socket = UdpSocket::bind(("0.0.0.0", DHCP_SERVER_PORT)).await?;
    dhcp_socket.set_broadcast(true)?;
    println!("DHCP server listening on port {}", DHCP_SERVER_PORT);
    
    //LOAD CONFIG
    let config = load_config("app/server-config.json").expect("Failed to load configuration");

    //START SERVER
    let server = Server::new(config, dhcp_socket, client).await;
    Arc::new(server).start().await;

    return Ok(());
}