use std::fs::File;
use std::io::Read;
use failure::Error;
use serde::Deserialize;
use std::path::PathBuf;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use crate::config::allow_addr::{AllowAddr, AllowIp};

mod allow_addr;

#[derive(Deserialize, Debug, Clone)]
pub struct RawConfigServer {
    name: String,
    ip: String,
    groups: Vec<String>,
}


#[derive(Deserialize, Debug, Clone)]
pub struct RawConfigGroup {
    name: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct RawConfigCheck {
    from: String,
    to: String,
    check: String
}

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    server: Vec<RawConfigServer>,
    group: Vec<RawConfigGroup>,
    check: Option<Vec<RawConfigCheck>>,
}

type Identifier = String;

#[derive(Clone)]
pub struct Ident {
    pub identifier: Identifier,
    pub ip: AllowIp,
}

pub struct ConfigCheck {
    pub from: Ident,
    pub to: Ident,
    pub check: String
}

impl Config {
    pub fn new_from_bytes(content: &[u8]) -> Result<Self, Error> {
        Ok(toml::from_slice(content)?)
    }

    pub fn all_checks(&self) -> Result<Vec<ConfigCheck>, Error> {
        let mut buf = vec![];
        match &self.check {
            None => {},
            Some(checks) => {
                for check in checks {
                    for from in &self.resolve_idents(check.from.clone())? {
                        for to in &self.resolve_idents(check.to.clone())? {
                            buf.push(ConfigCheck {
                                from: from.clone(),
                                to: to.clone(),
                                check: check.check.clone()
                            });
                        }
                    }
                }
            }
        };

        Ok(buf)
    }

    pub fn new_from_file(filename: PathBuf) -> Result<Self, Error> {
        let mut content = Vec::new();
        File::open(filename)?.read_to_end(&mut content)?;

        Self::new_from_bytes(&content)
    }

    pub fn resolve_idents<I>(&self, raw_identifier: I) -> Result<Vec<Ident>, Error>
        where I : AsRef<str> + Sized
    {
        let identifier : &str = raw_identifier.as_ref();
        let raw_servers = {
            let mut m = HashMap::new();
            for server in self.server.iter() {
                match m.entry(server.name.clone()) {
                    Entry::Occupied(v) => {
                        panic!("server {:?} already registered", v.get());
                    }
                    Entry::Vacant(v) => {
                        v.insert(server.clone());
                    }
                }
            }
            m
        };

        let raw_groups = {
            let mut m = HashMap::new();
            for group in self.group.iter() {
                match m.entry(group.name.clone()) {
                    Entry::Occupied(v) => {
                        panic!("group {:?} already registered", v.get());
                    }
                    Entry::Vacant(v) => {
                        v.insert(group.clone());
                    }
                }
            }
            m
        };

        if raw_servers.contains_key(identifier) && raw_groups.contains_key(identifier) {
            panic!("allow {:?} is ambigious (server, groups)", identifier);
        }

        if let Some(s) = raw_servers.get(identifier) {
            return Ok(vec![
                Ident {
                    identifier: s.name.clone(),
                    ip: AllowIp::V4(s.ip.parse()?),
                }
            ]);
        }

        if let Some(s) = raw_groups.get(identifier) {
            let mut buf = vec![];

            let servers_in_group = raw_servers.iter().filter(|(_, s)| {
                s.groups.contains(&identifier.to_string())
            }).collect::<Vec<_>>();

            for (_, s) in servers_in_group.iter() {
                buf.push(Ident {
                    identifier: s.name.clone(),
                    ip: AllowIp::V4(s.ip.parse()?),
                });
            }

            return Ok(buf);
        }

        Ok(vec![])

    }
}


#[cfg(test)]
mod tests {
    use super::*;

    fn load_config(content: &[u8]) -> Config {
        Config::new_from_bytes(content).expect("could not load config")
    }

    #[test]
    fn test_basic_servers() {
        let c = load_config(r#"
[[group]]
name = "g1"

[[group]]
name = "g2"

[[server]]
name = "server1"
ip = "127.0.0.1"
groups = ["g1"]

[[server]]
name = "server2"
ip = "127.0.0.2"
groups = ["g1", "g2"]
        "#.as_bytes());

        assert_eq!(
            vec!["server1", "server2"],
            {
                let mut v = c.resolve_idents("g1").unwrap().iter().map(|x| x.identifier.clone()).collect::<Vec<_>>();
                v.sort();
                v
            }
        );

        assert_eq!(
            vec!["server2"],
            c.resolve_idents("g2").unwrap().iter().map(|x|x.identifier.clone()).collect::<Vec<_>>()
        );

    }
}