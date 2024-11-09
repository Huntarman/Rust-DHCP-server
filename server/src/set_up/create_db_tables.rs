use tokio_postgres::Error as TokioError;
use std::net::{IpAddr, Ipv4Addr};
use std::io::Error as StdError;
use std::fmt;

use crate::server_config::{load_config, Config, generate_ip_pool};

use crate::set_up::config_hash;

pub async fn create_db (client: &tokio_postgres::Client) -> Result<(), CustomError> {

    // create_clients_table(&client).await?;
    create_ip_address_table(&client).await?;
    // create_dhcp_leases_table(&client).await?;
    fill_ip_addresses_table(&client).await?;

    return Ok(());
}

// async fn create_clients_table (client: &tokio_postgres::Client) -> Result<(), TokioError>{
    
//     let table_exists_query = "
//         SELECT EXISTS (
//             SELECT FROM information_schema.tables 
//             WHERE table_name = 'dhcp_clients'
//         );
//     ";
//     let row = client.query_one(table_exists_query, &[]).await?;
//     let table_exists: bool = row.get(0);

//     if table_exists {
//         println!("Table dhcp_clients already exists - skipping creation");
//     } else {
//         let create_dhcp_clients_table_query = "
//             CREATE TABLE IF NOT EXISTS dhcp_clients (
//                 client_id SERIAL PRIMARY KEY,
//                 mac_address MACADDR NOT NULL UNIQUE,
//                 client_identifier VARCHAR(255) UNIQUE,
//                 hostname VARCHAR(255),
//                 created_at TIMESTAMP NOT NULL DEFAULT NOW(),
//                 updated_at TIMESTAMP NOT NULL DEFAULT NOW()
//             )
//         ";
//         client.execute(create_dhcp_clients_table_query, &[]).await?;
//         println!("Table dhcp_clients created successfully");
//     }
//     return Ok(());
// }

async fn create_ip_address_table (client: &tokio_postgres::Client) -> Result<(), TokioError>{
    let table_exists_query = "
        SELECT EXISTS (
            SELECT FROM information_schema.tables 
            WHERE table_name = 'ip_addresses'
        );
    ";
    let row = client.query_one(table_exists_query, &[]).await?;
    let table_exists: bool = row.get(0);

    if table_exists {
        println!("Table ip_addresses already exists - skipping creation");
    } else {
        let create_ip_addresses_table_query = "
            CREATE TABLE IF NOT EXISTS ip_addresses (
                ip_address INET PRIMARY KEY,
                allocated BOOLEAN NOT NULL DEFAULT FALSE,
                client_id VARCHAR(32) UNIQUE,
                lease_start TIMESTAMP,
                lease_end TIMESTAMP
            )
        ";
        client.execute(create_ip_addresses_table_query, &[]).await?;
        println!("Table ip_addresses created successfully");
    }
    return Ok(());
}

// async fn create_dhcp_leases_table (client: &tokio_postgres::Client) -> Result<(), TokioError>{
//     let table_exists_query = "
//         SELECT EXISTS (
//             SELECT FROM information_schema.tables 
//             WHERE table_name = 'dhcp_leases'
//         );
//     ";
//     let row = client.query_one(table_exists_query, &[]).await?;
//     let table_exists: bool = row.get(0);

//     if table_exists {
//         println!("Table dhcp_leases already exists - skipping creation");
//     } else {
//         let create_dhcp_leases_table_query = "
//             CREATE TABLE dhcp_leases (
//                 lease_id SERIAL PRIMARY KEY,
//                 ip_address INET REFERENCES ip_addresses(ip_address) ON DELETE CASCADE,
//                 client_id INTEGER REFERENCES dhcp_clients(client_id) ON DELETE CASCADE,
//                 lease_start TIMESTAMP NOT NULL,
//                 lease_end TIMESTAMP NOT NULL,
//                 lease_duration INTERVAL NOT NULL,
//                 lease_status VARCHAR(50) CHECK (lease_status IN ('ACTIVE', 'EXPIRED', 'RENEWING', 'RELEASED')),
//                 created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
//                 updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
//             );
//         ";
//         client.execute(create_dhcp_leases_table_query, &[]).await?;
//         println!("Table dhcp_leases created successfully");
//     }
//     return Ok(());
// }

async fn fill_ip_addresses_table (client: &tokio_postgres::Client) -> Result<(), CustomError> {
    if config_hash::check_config_changed("app/server-config.json").map_err(|e| CustomError::from(e))? {
        println!("Server configuration changed - updating IP addresses table");
        let new_hash = config_hash::calculate_config_hash("app/server-config.json").map_err(|e| CustomError::from(e))?;
        config_hash::store_hash(&new_hash).map_err(|e| CustomError::from(e))?;

        let config = load_config("app/server-config.json").expect("Failed to load configuration");
    
        let start_ip: Ipv4Addr = config.ip_pool.range_start.parse().expect("Invalid start IP");
        let end_ip: Ipv4Addr = config.ip_pool.range_end.parse().expect("Invalid end IP");

        let ip_pool = generate_ip_pool(start_ip, end_ip);

        let mut clear_table = "DELETE FROM ip_addresses";
        client.execute(clear_table, &[]).await?;

        let insert_ip_query = "
            INSERT INTO ip_addresses (ip_address, allocated)
            VALUES ($1, $2);
        ";

        for ip in ip_pool {
            let ip_addr = IpAddr::V4(ip);
            if !config.restricted_ips.contains(&ip.to_string()) {
                client.execute(insert_ip_query, &[&ip_addr, &false]).await?;
            }
        }
        println!("IP adresses table filled with IP pool")
    } else {
        println!("Server configuration unchanged - skipping IP addresses table update");
    }   

    return Ok(());
 }

 pub enum CustomError {
    PostgresError(TokioError),
    IoError(StdError),
}

impl From<TokioError> for CustomError {
    fn from(err: TokioError) -> Self {
        CustomError::PostgresError(err)
    }
}

impl From<StdError> for CustomError {
    fn from(err: StdError) -> Self {
        CustomError::IoError(err)
    }
}

impl fmt::Display for CustomError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CustomError::PostgresError(e) => write!(f, "Database error: {}", e),
            CustomError::IoError(e) => write!(f, "I/O error: {}", e),
        }
    }
}