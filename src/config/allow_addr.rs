use failure::Error;
use std::net::Ipv4Addr;

enum AllowAddrPort
{
    Port(usize),
    Range(usize, usize),
    AnyPort,
}

enum AllowAddr {
    V4(Ipv4Addr, AllowAddrPort),
    V6(String, AllowAddrPort),
    Server(String, AllowAddrPort),
    Group(String, AllowAddrPort),
}

impl AllowAddr {

    fn parse_v4(data : &str) -> Result<Self, Error> {

        let parts : Vec<String> = data.trim_start_matches("v4:").split(':').map(|s|s.to_string()).collect();

        if parts.len() == 1 {
            return Ok(AllowAddr::V4(parts.first().unwrap().parse()?, AllowAddrPort::AnyPort))
        }

        Err(format_err!("nöpe"))
    }

    pub fn from_str(data : &str) -> Result<Self, Error> {
        if data.starts_with("v4:") {
            return Self::parse_v4(data);
        }

        Err(format_err!("could not parse '{}' the identifier should start with 'v4:', 'v6:', 'server:' or 'group:'", data))
    }
}