use std::net::Ipv4Addr;

pub fn generate_ip_pool(start: Ipv4Addr, end: Ipv4Addr) -> Vec<Ipv4Addr> {
    let mut ips = Vec::new();
    let mut current = u32::from(start);
    let end = u32::from(end);

    while current <= end {
        ips.push(Ipv4Addr::from(current));
        current += 1;
    }

    ips
}