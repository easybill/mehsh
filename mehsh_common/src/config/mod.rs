use crate::config::allow_addr::AllowIp;
use serde::Deserialize;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use anyhow::Context;

mod allow_addr;

#[derive(Deserialize, Debug, Clone)]
pub struct RawConfigServer {
    #[serde(alias = "name")]
    pub identifier: ServerIdentifier,
    pub datacenter: Option<String>,
    pub ip: String,
    pub groups: Vec<String>,
    pub serverdensity_udp_agent: Option<bool>,
    pub extra1: Option<String>,
    pub extra2: Option<String>,
    pub extra3: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ConfigServer {
    pub identifier: ServerIdentifier,
    pub datacenter: Option<String>,
    pub datacenter_as_entries: Vec<String>,
    pub ip: String,
    pub groups: Vec<String>,
    pub serverdensity_udp_agent: bool,
    pub extra1: Option<String>,
    pub extra2: Option<String>,
    pub extra3: Option<String>,
}

impl ConfigServer {
    pub fn from_raw_config_server(raw: RawConfigServer) -> Self {
        let datacenter_as_entries = {
            let mut buf = vec![];

            let dc = raw
                .datacenter
                .clone()
                .unwrap_or("no-datacenter".to_string());

            let mut path_to_travel = dc;
            let mut path_traveled = "".to_string();
            loop {
                match path_to_travel.clone().split_once(".") {
                    Some((s1, s2)) => {
                        buf.push(
                            format!("{}{}", path_traveled, s1)
                                .trim_start_matches(".")
                                .to_string(),
                        );
                        path_to_travel = s2.to_string();
                        path_traveled.push_str(&format!("{}.", s1))
                    }
                    None => {
                        buf.push(
                            format!("{}{}", path_traveled, path_to_travel)
                                .trim_start_matches(".")
                                .to_string(),
                        );
                        break;
                    }
                };
            }

            buf
        };

        Self {
            identifier: raw.identifier,
            datacenter: raw.datacenter,
            datacenter_as_entries,
            ip: raw.ip,
            groups: raw.groups,
            serverdensity_udp_agent: raw.serverdensity_udp_agent.unwrap_or(false),
            extra1: raw.extra1,
            extra2: raw.extra2,
            extra3: raw.extra3,
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct RawConfigGroup {
    name: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct RawConfigAnalysis {
    name: String,
    from: String,
    to: String,
    min_loss: u32,
    command: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct RawConfigCheck {
    from: String,
    to: String,
    check: String,
    http_url: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct RawConfig {
    server: Vec<RawConfigServer>,
    group: Vec<RawConfigGroup>,
    check: Option<Vec<RawConfigCheck>>,
    analysis: Option<Vec<RawConfigAnalysis>>,
}

#[derive(Debug, Clone)]
pub struct Config {
    self_server_identifier: ServerIdentifier,
    servers_by_identifier: HashMap<ServerIdentifier, ConfigServer>,
    server_self: ConfigServer,
    server: Vec<ConfigServer>,
    group: Vec<RawConfigGroup>,
    check: Option<Vec<RawConfigCheck>>,
    analysis: Option<Vec<RawConfigAnalysis>>,
}

pub type ServerIdentifier = String;

#[derive(Clone)]
pub struct Ident {
    pub identifier: ServerIdentifier,
    pub ip: AllowIp,
}

#[derive(Clone)]
pub struct ConfigCheck {
    pub from: Ident,
    pub to: Ident,
    pub check: String,
    pub http_url: Option<String>,
}

#[derive(Clone)]
pub struct ConfigAnalysis {
    pub name: String,
    pub from: ConfigServer,
    pub to: ConfigServer,
    pub min_loss: u32,
    pub command: String,
}

impl Config {
    pub fn new_from_bytes(
        self_server_identifier: ServerIdentifier,
        content: &[u8],
    ) -> Result<Self, ::anyhow::Error> {
        let raw_config = toml::from_str::<RawConfig>(
            String::from_utf8(content.to_vec()).context("could not read toml, invalid utf8")?.as_str()
        )?;

        let servers = raw_config
            .server
            .iter()
            .map(|s| ConfigServer::from_raw_config_server(s.clone()))
            .collect::<Vec<_>>();

        let servers_by_identifiers = {
            let mut map = HashMap::new();
            for s in servers.iter() {
                map.insert(s.identifier.clone(), s.clone());
            }

            map
        };

        let server_self = servers
            .iter()
            .find(|v| v.identifier == self_server_identifier)
            .map(|v| v.clone())
            .expect(&format!(
                "could not find server {} in config",
                self_server_identifier.as_str()
            ));

        Ok(Config {
            self_server_identifier,
            servers_by_identifier: servers_by_identifiers,
            server_self,
            server: servers,
            check: raw_config.check,
            group: raw_config.group,
            analysis: raw_config.analysis,
        })
    }

    pub fn all_analyisis(&self) -> Result<Vec<ConfigAnalysis>, ::anyhow::Error> {
        let mut buf = HashMap::new();
        match &self.analysis {
            None => {}
            Some(analysis) => {
                for analysis_entry in analysis {
                    for from in &self.resolve_idents(analysis_entry.from.clone())? {
                        for to in &self.resolve_idents(analysis_entry.to.clone())? {
                            let key = (
                                from.identifier.clone(),
                                to.identifier.clone(),
                                analysis_entry.name.clone(),
                            );
                            if buf.contains_key(&key) {
                                eprintln!("warning, you defined the same analysis multiple times. from: {}, to: {}, analysis: {}", from.identifier.clone(), to.identifier.clone(), analysis_entry.name);
                                continue;
                            }
                            
                            buf.insert(
                                key,
                                ConfigAnalysis {
                                    from: self.get_server_by_identifier(&from.identifier).expect("invalid server in analysis from, should never happen.").clone(),
                                    to: self.get_server_by_identifier(&to.identifier).expect("invalid server in analysis to, should never happen.").clone(),
                                    name: analysis_entry.name.clone(),
                                    command: analysis_entry.command.clone(),
                                    min_loss: analysis_entry.min_loss.clone(),
                                },
                            );
                        }
                    }
                }
            }
        };

        Ok(buf.into_iter().map(|(_k, v)| v).collect::<Vec<_>>())
    }

    pub fn all_checks(&self) -> Result<Vec<ConfigCheck>, ::anyhow::Error> {
        let mut buf = HashMap::new();
        match &self.check {
            None => {}
            Some(checks) => {
                for check in checks {
                    for from in &self.resolve_idents(check.from.clone())? {
                        for to in &self.resolve_idents(check.to.clone())? {
                            let key = (
                                from.identifier.clone(),
                                to.identifier.clone(),
                                check.check.clone(),
                                check.http_url.clone(),
                            );
                            if buf.contains_key(&key) {
                                eprintln!("warning, you defined the same check multiple times. from: {}, to: {}, check: {}", from.identifier.clone(), to.identifier.clone(), check.check.clone());
                            }

                            buf.insert(
                                key,
                                ConfigCheck {
                                    from: from.clone(),
                                    to: to.clone(),
                                    check: check.check.clone(),
                                    http_url: check.http_url.clone(),
                                },
                            );
                        }
                    }
                }
            }
        };

        Ok(buf.into_iter().map(|(_k, v)| v).collect::<Vec<_>>())
    }

    pub fn new_from_file(
        self_server_identifier: ServerIdentifier,
        filename: PathBuf,
    ) -> Result<Self, ::anyhow::Error> {
        let mut content = Vec::new();
        File::open(filename)?.read_to_end(&mut content)?;

        Self::new_from_bytes(self_server_identifier, &content)
    }

    pub fn is_server_or_is_in_group(&self, server_or_group_identifier: &str) -> bool {
        self.server_self.identifier == server_or_group_identifier
            || self
                .server_self
                .groups
                .contains(&server_or_group_identifier.to_string())
    }

    pub fn get_server_by_identifier(&self, identifier: &ServerIdentifier) -> Option<&ConfigServer> {
        self.servers_by_identifier.get(identifier)
    }

    pub fn get_server_self(&self) -> &ConfigServer {
        &self.server_self
    }

    pub fn resolve_idents<I>(&self, raw_identifier: I) -> Result<Vec<Ident>, ::anyhow::Error>
    where
        I: AsRef<str> + Sized,
    {
        let identifier: &str = raw_identifier.as_ref();
        let raw_servers = {
            let mut m = HashMap::new();
            for server in self.server.iter() {
                match m.entry(server.identifier.clone()) {
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
            return Ok(vec![Ident {
                identifier: s.identifier.clone(),
                ip: AllowIp::V4(s.ip.parse()?),
            }]);
        }

        if let Some(_) = raw_groups.get(identifier) {
            let mut buf = vec![];

            let servers_in_group = raw_servers
                .iter()
                .filter(|(_, s)| s.groups.contains(&identifier.to_string()))
                .collect::<Vec<_>>();

            for (_, s) in servers_in_group.iter() {
                buf.push(Ident {
                    identifier: s.identifier.clone(),
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
        Config::new_from_bytes("server1".to_string(), content).expect("could not load config")
    }

    #[test]
    fn test_basic_servers() {
        let c = load_config(
            r#"
[[group]]
name = "g1"

[[group]]
name = "g2"

[[server]]
name = "server1"
ip = "127.0.0.1"
datacenter = "fra.dc11.foo.xyz"
groups = ["g1"]

[[server]]
name = "server2"
ip = "127.0.0.2"
groups = ["g1", "g2"]
        "#
            .as_bytes(),
        );

        assert_eq!(vec!["server1", "server2"], {
            let mut v = c
                .resolve_idents("g1")
                .unwrap()
                .iter()
                .map(|x| x.identifier.clone())
                .collect::<Vec<_>>();
            v.sort();
            v
        });

        assert_eq!(
            vec!["server2"],
            c.resolve_idents("g2")
                .unwrap()
                .iter()
                .map(|x| x.identifier.clone())
                .collect::<Vec<_>>()
        );

        assert_eq!(
            vec!["fra", "fra.dc11", "fra.dc11.foo", "fra.dc11.foo.xyz"],
            c.get_server_by_identifier(&"server1".to_string())
                .expect("server must exists")
                .datacenter_as_entries
        );
    }
}
