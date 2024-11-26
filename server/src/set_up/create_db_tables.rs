use tokio_postgres::Error as TokioError;
use std::net::{IpAddr, Ipv4Addr};
use std::io::Error as StdError;
use std::fmt;

use crate::server_config::{load_config, generate_ip_pool};

use crate::set_up::config_hash;

pub async fn create_db (client: &tokio_postgres::Client) -> Result<(), CustomError> {

    create_ip_address_table(&client).await?;
    create_lease_history_table(&client).await?;
    fill_ip_addresses_table(&client).await?;

    return Ok(());
}

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

async fn create_lease_history_table (client: &tokio_postgres::Client) -> Result<(), TokioError>{
    let table_exists_query = "
        SELECT EXISTS (
            SELECT FROM information_schema.tables 
            WHERE table_name = 'lease_history'
        );
    ";
    let row = client.query_one(table_exists_query, &[]).await?;
    let table_exists: bool = row.get(0);

    if table_exists {
        println!("Table lease_history already exists - skipping creation");
    } else {
        let check_server_response_type = "
            SELECT EXISTS (
                SELECT 1
                FROM pg_type t
                JOIN pg_namespace n ON n.oid = t.typnamespace
                WHERE t.typname = 'server_response'
            )";
        let check_server_resp_type_row = client.query_one(check_server_response_type, &[]).await?;
        let server_response_type_exists: bool = check_server_resp_type_row.get(0);
        if !server_response_type_exists {
            let create_enum = "
                CREATE TYPE server_response AS ENUM (
                    'ACK',
                    'NAK'
                )";
            client.execute(create_enum, &[]).await?;
        }

        let check_lease_type_enum = "
            SELECT EXISTS (
                SELECT 1
                FROM pg_type t
                JOIN pg_namespace n ON n.oid = t.typnamespace
                WHERE t.typname = 'lease_type'
            )";
        let check_lease_type_enum_row = client.query_one(check_lease_type_enum, &[]).await?;
        let lease_type_enum_exists: bool = check_lease_type_enum_row.get(0);
        if !lease_type_enum_exists {
            let create_lease_type_enum = "
                CREATE TYPE lease_type AS ENUM (
                    'RENEWING',
                    'INITIAL',
                    'DECLINED'
                )";
            client.execute(create_lease_type_enum, &[]).await?;
        }

        let create_lease_history_table_query = "
            CREATE TABLE IF NOT EXISTS lease_history (
                id SERIAL PRIMARY KEY,
                ip_address INET NOT NULL,
                client_id VARCHAR(32) NOT NULL,
                lease_start TIMESTAMP,
                lease_end TIMESTAMP,
                server_response server_response NOT NULL,
                lease_type lease_type NOT NULL
            )
        ";
        client.execute(create_lease_history_table_query, &[]).await?;
        println!("Table lease_history created successfully");
    }
    return Ok(());
}
async fn fill_ip_addresses_table (client: &tokio_postgres::Client) -> Result<(), CustomError> {
    //CHECK IF CONFIG FILE HAS CHANGED
    //IF CONFIG FILE HAS CHANGED, DELETE ALL ENTRIES IN IP ADDRESSES TABLE AND FILL IT WITH NEW IP POOL
    //OTHERWISE LEAVE THE TABLE AS IT IS

    if config_hash::check_config_changed("app/server-config.json").map_err(|e| CustomError::from(e))? {
        println!("Server configuration changed - updating IP addresses table");
        let new_hash = config_hash::calculate_config_hash("app/server-config.json").map_err(|e| CustomError::from(e))?;
        config_hash::store_hash(&new_hash).map_err(|e| CustomError::from(e))?;

        let config = load_config("app/server-config.json").expect("Failed to load configuration");
    
        let start_ip: Ipv4Addr = config.ip_pool.range_start.parse().expect("Invalid start IP");
        let end_ip: Ipv4Addr = config.ip_pool.range_end.parse().expect("Invalid end IP");

        let ip_pool = generate_ip_pool(start_ip, end_ip);

        let clear_table = "DELETE FROM ip_addresses";
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