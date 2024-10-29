use tokio_postgres::Error;

pub async fn create_db (client: &tokio_postgres::Client) -> Result<(), Error> {

    create_clients_table(&client).await?;
    create_ip_address_table(&client).await?;
    create_dhcp_leases_table(&client).await?;
    
    return Ok(());
}

async fn create_clients_table (client: &tokio_postgres::Client) -> Result<(), Error>{
    
    let table_exists_query = "
        SELECT EXISTS (
            SELECT FROM information_schema.tables 
            WHERE table_name = 'dhcp_clients'
        );
    ";
    let row = client.query_one(table_exists_query, &[]).await?;
    let table_exists: bool = row.get(0);

    if table_exists {
        println!("Table dhcp_clients already exists - skipping creation");
    } else {
        let create_dhcp_clients_table_query = "
            CREATE TABLE IF NOT EXISTS dhcp_clients (
                client_id SERIAL PRIMARY KEY,
                mac_address MACADDR NOT NULL UNIQUE,
                client_identifier VARCHAR(255) UNIQUE,
                hostname VARCHAR(255),
                created_at TIMESTAMP NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMP NOT NULL DEFAULT NOW()
            )
        ";
        client.execute(create_dhcp_clients_table_query, &[]).await?;
        println!("Table dhcp_clients created successfully");
    }
    return Ok(());
}

async fn create_ip_address_table (client: &tokio_postgres::Client) -> Result<(), Error>{
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
                subnet_mask INET NOT NULL,
                allocated BOOLEAN NOT NULL DEFAULT FALSE,
                allocated_to_client_id INTEGER REFERENCES dhcp_clients(client_id),
                lease_start TIMESTAMP,
                lease_end TIMESTAMP
            )
        ";
        client.execute(create_ip_addresses_table_query, &[]).await?;
        println!("Table ip_addresses created successfully");
    }
    return Ok(());
}

async fn create_dhcp_leases_table (client: &tokio_postgres::Client) -> Result<(), Error>{
    let table_exists_query = "
        SELECT EXISTS (
            SELECT FROM information_schema.tables 
            WHERE table_name = 'dhcp_leases'
        );
    ";
    let row = client.query_one(table_exists_query, &[]).await?;
    let table_exists: bool = row.get(0);

    if table_exists {
        println!("Table dhcp_leases already exists - skipping creation");
    } else {
        let create_dhcp_leases_table_query = "
            CREATE TABLE dhcp_leases (
                lease_id SERIAL PRIMARY KEY,
                ip_address INET REFERENCES ip_addresses(ip_address) ON DELETE CASCADE,
                client_id INTEGER REFERENCES dhcp_clients(client_id) ON DELETE CASCADE,
                lease_start TIMESTAMP NOT NULL,
                lease_end TIMESTAMP NOT NULL,
                lease_duration INTERVAL NOT NULL,
                lease_status VARCHAR(50) CHECK (lease_status IN ('ACTIVE', 'EXPIRED', 'RENEWING', 'RELEASED')),
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );
        ";
        client.execute(create_dhcp_leases_table_query, &[]).await?;
        println!("Table dhcp_leases created successfully");
    }
    return Ok(());
}