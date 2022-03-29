use failure::Error;
use std::net::Ipv4Addr;

#[derive(Debug, Clone, PartialEq)]
pub enum AllowAddrPort {
    Port(usize),
    Range(usize, usize),
    AnyPort,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AllowAddr {
    V4(Ipv4Addr, AllowAddrPort),
    V6(String, AllowAddrPort),
    Server(String, AllowAddrPort),
    Group(String, AllowAddrPort),
}

#[derive(Debug, Clone, PartialEq)]
pub enum AllowIp {
    V4(Ipv4Addr),
    V6(String),
}

impl AllowIp {
    pub fn to_string(&self) -> String {
        match &self {
            AllowIp::V4(v) => v.to_string(),
            AllowIp::V6(v) => v.to_string(),
        }
    }
}

impl AllowAddrPort {
    pub fn new_from_str(s: &str) -> Result<Self, Error> {
        if s == "*" {
            return Ok(AllowAddrPort::AnyPort);
        }

        if let Ok(k) = s.parse::<usize>() {
            return Ok(AllowAddrPort::Port(k));
        }

        let parts: Vec<_> = s.split('-').map(|s| s.to_string()).collect();

        if parts.len() != 2 {
            return Err(format_err!("could not decode addr port {}", s));
        }

        let min = {
            if let Ok(k) = parts[0].parse() {
                Ok(k)
            } else {
                Err(format_err!("could not decode (min) addr port {}", s))
            }
        }?;

        let max = {
            if let Ok(k) = parts[1].parse() {
                Ok(k)
            } else {
                Err(format_err!("could not decode (max) addr port {}", s))
            }
        }?;

        if parts[0] > parts[1] {
            return Err(format_err!("range mismatch {}", s));
        }

        Ok(AllowAddrPort::Range(min, max))
    }
}

impl AllowAddr {
    fn parse_v4(data: &str) -> Result<Self, Error> {
        let parts: Vec<_> = data
            .trim_start_matches("v4:")
            .split(':')
            .map(|s| s.to_string())
            .collect();

        if parts.len() == 1 {
            return Ok(AllowAddr::V4(parts[0].parse()?, AllowAddrPort::AnyPort));
        }

        if parts.len() == 2 {
            return Ok(AllowAddr::V4(
                parts.first().unwrap().parse()?,
                AllowAddrPort::new_from_str(&parts[1])?,
            ));
        }

        Err(format_err!("nöpe"))
    }

    pub fn new_from_str(data: &str) -> Result<Self, Error> {
        if data.starts_with("v4:") {
            return Self::parse_v4(data);
        }

        Err(format_err!("could not parse '{}' the identifier should start with 'v4:', 'v6:', 'server:' or 'group:'", data))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_allow_addr() {
        assert_eq!(
            AllowAddr::V4(
                Ipv4Addr::from_str("127.0.0.1").unwrap(),
                AllowAddrPort::AnyPort
            ),
            AllowAddr::new_from_str("v4:127.0.0.1").unwrap()
        );

        assert_eq!(
            AllowAddr::V4(
                Ipv4Addr::from_str("127.0.0.1").unwrap(),
                AllowAddrPort::Port(2121)
            ),
            AllowAddr::new_from_str("v4:127.0.0.1:2121").unwrap()
        );

        assert_eq!(
            AllowAddr::V4(
                Ipv4Addr::from_str("127.0.0.1").unwrap(),
                AllowAddrPort::Range(20, 25)
            ),
            AllowAddr::new_from_str("v4:127.0.0.1:20-25").unwrap()
        );

        assert_eq!(
            AllowAddr::V4(
                Ipv4Addr::from_str("127.0.0.1").unwrap(),
                AllowAddrPort::AnyPort
            ),
            AllowAddr::new_from_str("v4:127.0.0.1:*").unwrap()
        );
    }

    #[test]
    fn test_allow_addr_port() {
        assert_eq!(
            AllowAddrPort::Port(80),
            AllowAddrPort::new_from_str("80").unwrap()
        );
        assert_eq!(
            AllowAddrPort::Port(8080),
            AllowAddrPort::new_from_str("8080").unwrap()
        );
        assert_eq!(
            AllowAddrPort::Range(80, 88),
            AllowAddrPort::new_from_str("80-88").unwrap()
        );
        assert_eq!(
            AllowAddrPort::AnyPort,
            AllowAddrPort::new_from_str("*").unwrap()
        );
    }
}
