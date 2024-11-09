use std::net::Ipv4Addr;
use std::collections::HashMap;

const BOOT_FILENAME_SIZE: usize = 128;
const SERVER_NAME_SIZE: usize = 64;
const MAGIC_COOKIE: [u8; 4] = [99, 130, 83, 99];

#[derive(Debug, Clone)]
pub struct DHCPMessage {
    pub op: u8,
    pub htype: u8,
    pub hlen: u8,
    pub hops: u8,
    pub xid: u32,
    pub secs: u16,
    pub flags: u16,
    pub ciaddr: Ipv4Addr,
    pub yiaddr: Ipv4Addr,
    pub siaddr: Ipv4Addr,
    pub giaddr: Ipv4Addr,
    pub chaddr: [u8; 16],
    pub sname: [u8; SERVER_NAME_SIZE],
    pub file: [u8; BOOT_FILENAME_SIZE],
    pub options: Vec<u8>,
    pub options_map: HashMap<u8, Vec<u8>>,
}

impl DHCPMessage {
    pub fn from_buffer(buf: &[u8]) -> Result<Self, &'static str> {
        if buf.len() < 240 {
            return Err("Buffer size is too small to be a valid DHCP message");
        }
        
        if &buf[236..240] != MAGIC_COOKIE {
            return Err("Invalid DHCP magic cookie");
        }

        let options = buf[236..].to_vec();

        let mut options_map = HashMap::new();
        let mut i = 4;

        while i < options.len() {
            if options[i] == 0 {
                break;
            }
            let len = *options.get(i + 1).ok_or("Options length out of bounds")? as usize;
            let option_end = i + 2 + len;
            if option_end > options.len() {
                println!("{}",len);
                return Err("Options data truncated");
            }
            options_map.insert(options[i], options[i + 2..option_end].to_vec());
            i = option_end;
        }

        Ok(DHCPMessage {
            op: buf[0],
            htype: buf[1],
            hlen: buf[2],
            hops: buf[3],
            xid: u32::from_be_bytes([buf[4], buf[5], buf[6], buf[7]]),
            secs: u16::from_be_bytes([buf[8], buf[9]]),
            flags: u16::from_be_bytes([buf[10], buf[11]]),
            ciaddr: Ipv4Addr::new(buf[12], buf[13], buf[14], buf[15]),
            yiaddr: Ipv4Addr::new(buf[16], buf[17], buf[18], buf[19]),
            siaddr: Ipv4Addr::new(buf[20], buf[21], buf[22], buf[23]),
            giaddr: Ipv4Addr::new(buf[24], buf[25], buf[26], buf[27]),
            chaddr: {
                let mut chaddr = [0u8; 16];
                chaddr.copy_from_slice(&buf[28..44]);
                chaddr
            },
            sname: {
                let mut sname = [0u8; SERVER_NAME_SIZE];
                sname.copy_from_slice(&buf[44..44 + SERVER_NAME_SIZE]);
                sname
            },
            file: {
                let mut file = [0u8; BOOT_FILENAME_SIZE];
                file.copy_from_slice(&buf[108..108 + BOOT_FILENAME_SIZE]);
                file
            },
            options: options,
            options_map: options_map,
        })
    }

    pub fn new(
        op: u8,
        htype: u8,
        hlen: u8,
        hops: u8,
        xid: u32,
        secs: u16,
        flags: u16,
        ciaddr: Ipv4Addr,
        yiaddr: Ipv4Addr,
        siaddr: Ipv4Addr,
        giaddr: Ipv4Addr,
        chaddr: [u8; 16],
        sname: [u8; SERVER_NAME_SIZE],
        file: [u8; BOOT_FILENAME_SIZE],
        options: Vec<u8>,
    ) -> Self {
        let mut full_options = MAGIC_COOKIE.to_vec();
        full_options.extend_from_slice(&options);

        let mut options_map = HashMap::new();
        let mut i = 4;
        while i < full_options.len() {
            if full_options[i] == 0 {
                break;
            }
            let len = full_options[i + 1] as usize;
            if i + len + 2 > full_options.len() {
                break;
            }
            options_map.insert(full_options[i], full_options[i + 2..i + len + 2].to_vec());
            i += len + 2;
        }
        options_map.insert(255, Vec::new());
        full_options.push(255);

        DHCPMessage {
            op,
            htype,
            hlen,
            hops,
            xid,
            secs,
            flags,
            ciaddr,
            yiaddr,
            siaddr,
            giaddr,
            chaddr,
            sname,
            file,
            options: full_options,
            options_map,
        }
    }

    pub fn to_buffer(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(236 + self.options.len());

        buf.push(self.op);
        buf.push(self.htype);
        buf.push(self.hlen);
        buf.push(self.hops);
        buf.extend_from_slice(&self.xid.to_be_bytes());
        buf.extend_from_slice(&self.secs.to_be_bytes());
        buf.extend_from_slice(&self.flags.to_be_bytes());
        buf.extend_from_slice(&self.ciaddr.octets());
        buf.extend_from_slice(&self.yiaddr.octets());
        buf.extend_from_slice(&self.siaddr.octets());
        buf.extend_from_slice(&self.giaddr.octets());
        buf.extend_from_slice(&self.chaddr);
        buf.extend_from_slice(&self.sname);
        buf.extend_from_slice(&self.file);
        buf.extend_from_slice(&self.options);

        buf
    }
}
