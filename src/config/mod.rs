use std::fs::File;
use std::io::Read;
use failure::Error;
use serde::Deserialize;
use std::path::PathBuf;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use crate::config::allow_addr::AllowAddr;

mod allow_addr;

#[derive(Deserialize, Debug, Clone)]
pub struct RawConfigServer {
    name: String,
    public_key: String,
    endpoint: String,
    v4: String,
    groups: Vec<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct RawConfigGroup {
    name: String,
    allow: Vec<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct RawConfigCheck {
    from: String,
    to: String,
    check: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct RawConfig {
    server: Vec<RawConfigServer>,
    group: Vec<RawConfigGroup>,
    check: Vec<RawConfigCheck>
}

impl RawConfig {
    pub fn new_from_bytes(content: &[u8]) -> Result<Self, Error> {
        Ok(toml::from_slice(content)?)
    }

    pub fn new_from_file(filename: PathBuf) -> Result<Self, Error> {
        let mut content = Vec::new();
        File::open(filename)?.read_to_end(&mut content)?;

        Self::new_from_bytes(&content)
    }
}

#[derive(Debug, Clone)]
pub struct ConfigServer {
    pub name: String,
    pub public_key: String,
    pub endpoint: AllowAddr,
    pub v4: AllowAddr,
    pub allow: Vec<ConfigAllow>,
    pub groups: Vec<ConfigServerGroup>,
    pub checks: Vec<ConfigServerCheck>
}

impl ConfigServer {
    pub fn get_group_names(&self) -> Vec<&str> {
        self.groups.iter().map(|g| g.name.as_str()).collect()
    }
}

enum ConfigServerCheckName {
    Ping
}

pub struct ConfigServerCheck {
    from: AllowAddr,
    to: AllowAddr,
    check: ConfigServerCheckName
}

#[derive(Debug, Clone)]
pub struct ConfigServerGroup {
    name: String
}

#[derive(Debug, Clone)]
pub struct ConfigAllow {
    name: String,
    v4: AllowAddr,
}

#[derive(Debug, Clone)]
pub struct Config {
    server: Vec<ConfigServer>,
}

impl Config {

    fn new_from_raw_config(raw: RawConfig) -> Result<Self, Error> {
        let raw_servers = {
            let mut m = HashMap::new();
            for server in raw.server.iter() {
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
            for group in raw.group.iter() {
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

        fn resolve_allow(
            raw_servers: &HashMap<String, RawConfigServer>,
            raw_groups: &HashMap<String, RawConfigGroup>,
            name: &String,
            recusion_guard: &mut Vec<String>
        ) -> Result<Vec<ConfigAllow>, Error> {
            if recusion_guard.contains(name) {
                return Ok(vec![]);
            }

            recusion_guard.push(name.clone());

            if raw_servers.contains_key(name) && raw_groups.contains_key(name) {
                panic!("allow {:?} is ambigious (server, groups)", name);
            }

            if let Some(s) = raw_servers.get(name) {
                return Ok(vec![
                    ConfigAllow {
                        name: s.name.clone(),
                        v4: AllowAddr::new_from_str(&s.v4)?,
                    }
                ]);
            }

            if let Some(s) = raw_groups.get(name) {
                let mut buf = vec![];
                for x in s.allow.iter() {
                    buf.extend(resolve_allow(&raw_servers, &raw_groups, x, recusion_guard)?);
                }

                let servers_in_group = raw_servers.iter().filter(|(_, s)|{
                    s.groups.contains(name)
                }).collect::<Vec<_>>();

                for (_, s) in servers_in_group.iter() {
                    buf.push(ConfigAllow {
                        name: s.name.clone(),
                        v4: AllowAddr::new_from_str(&s.v4)?
                    });
                }

                return Ok(buf);
            }

            Ok(vec![])
        }

        let server_config = raw.server.iter().map(|server| {
            let mut allows = vec![];

            let server_group = server.groups.iter().filter_map(|g|{
                raw_groups.get(g)
            }).collect::<Vec<_>>();

            for g in server_group.iter() {
                for ga in g.allow.iter() {
                    allows.extend(
                        resolve_allow(&raw_servers, &raw_groups, ga, &mut vec![server.name.clone()])?
                    )
                }
            }

            Ok(ConfigServer {
                name: server.name.clone(),
                public_key: server.public_key.clone(),
                endpoint: AllowAddr::new_from_str(&server.endpoint)?,
                v4: AllowAddr::new_from_str(&server.v4)?,
                allow: allows,
                groups: server_group.iter().map(|g| {
                    ConfigServerGroup {
                        name: g.name.clone()
                    }
                }).collect(),
            })

        }).collect::<Result<Vec<_>, Error>>()?;


        Ok(Config {
            server: server_config
        })
    }

    pub fn new_from_file(filename: PathBuf) -> Result<Self, Error> {
        Self::new_from_raw_config(RawConfig::new_from_file(filename)?)
    }

    pub fn new_from_bytes(content: &[u8]) -> Result<Self, Error> {
        Self::new_from_raw_config(RawConfig::new_from_bytes(content)?)
    }

    pub fn get_servers(&self) -> &Vec<ConfigServer> {
        &self.server
    }

    pub fn get_server_clone(&self, server_name : &str) -> Option<ConfigServer> {
        self.server.iter()
            .find(|s| { &s.name == server_name  })
            .map(|s| s.clone())
    }


}


#[cfg(test)]
mod tests {
    use super::*;

    fn load_config(content : &[u8]) -> Config {
        Config::new_from_bytes(content).expect("could not load config")
    }

    #[test]
    fn test_basic_servers() {
        let c = load_config(r#"
[[group]]
name = "servers"
allow = []

[[server]]
name = "server1"
public_key = "FUZFUFJHGUFU"
endpoint = "v4:126.0.0.1"
v4 = "v4:127.0.0.1"
groups = ["servers"]

[[server]]
name = "server2"
public_key = "xxxxx"
endpoint = "v4:127.0.0.2"
v4 = "v4:127.0.0.2"
groups = ["servers"]
        "#.as_bytes());

        assert_eq!(2, c.get_servers().len());
        let server1 = c.get_server_clone("server1").expect("1");
        assert_eq!("server1", server1.name);
        assert_eq!("FUZFUFJHGUFU", server1.public_key);
        assert_eq!(AllowAddr::new_from_str("v4:126.0.0.1").unwrap(), server1.endpoint);
        assert_eq!(AllowAddr::new_from_str("v4:127.0.0.1").unwrap(), server1.v4);
        assert_eq!(vec!["servers"], server1.get_group_names());

        let server2 = c.get_server_clone("server2").expect("2");
        assert_eq!("server2", server2.name);
        assert_eq!(vec!["servers"], server2.get_group_names());

    }

    #[test]
    fn test_server_in_multiple_groups() {
        let c = load_config(r#"
[[group]]
name = "g1"
allow = []

[[group]]
name = "g2"
allow = []

[[server]]
name = "server1"
public_key = "FUZFUFJHGUFU"
endpoint = "v4:126.0.0.1"
v4 = "v4:127.0.0.1"
groups = ["g1", "g2"]

        "#.as_bytes());

        let server1 = c.get_server_clone("server1").expect("1");
        assert_eq!(vec!["g1", "g2"], server1.get_group_names());

    }

    #[test]
    fn test_server_A_is_in_a_group_that_allows_server_B() {
        let c = load_config(r#"
[[group]]
name = "g1"
allow = ["serverB"]

[[server]]
name = "serverA"
public_key = "FUZFUFJHGUFU"
endpoint = "v4:126.0.0.1"
v4 = "v4:127.0.0.1"
groups = ["g1"]

[[server]]
name = "serverB"
public_key = "FUZFUFJHGUFU"
endpoint = "v4:126.0.0.1"
v4 = "v4:127.0.0.1"
groups = []

        "#.as_bytes());

        let serverA = c.get_server_clone("serverA").expect("1");
        assert_eq!(1, serverA.allow.len());

    }
}