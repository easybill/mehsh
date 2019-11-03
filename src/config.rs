use std::fs::File;
use std::io::Read;
use failure::Error;
use serde::Deserialize;
use std::path::PathBuf;
use std::collections::HashMap;
use std::collections::hash_map::Entry;

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
pub struct RawConfig {
    server: Vec<RawConfigServer>,
    group: Vec<RawConfigGroup>,
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

#[derive(Debug)]
pub struct ConfigServer {
    name: String,
    public_key: String,
    endpoint: String,
    v4: String,
    allow: Vec<ConfigAllow>,
}

#[derive(Debug)]
pub struct ConfigAllow {
    name: String,
    v4: String,
}

#[derive(Debug)]
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
        ) -> Vec<ConfigAllow> {
            if recusion_guard.contains(name) {
                return vec![];
            }

            recusion_guard.push(name.clone());

            if raw_servers.contains_key(name) && raw_groups.contains_key(name) {
                panic!("allow {:?} is ambigious (server, groups)", name);
            }

            if let Some(s) = raw_servers.get(name) {
                return vec![
                    ConfigAllow {
                        name: s.name.clone(),
                        v4: s.v4.clone(),
                    }
                ];
            }

            if let Some(s) = raw_groups.get(name) {
                let mut buf = vec![];
                for x in s.allow.iter() {
                    buf.extend(resolve_allow(&raw_servers, &raw_groups, x, recusion_guard));
                }

                let servers_in_group = raw_servers.iter().filter(|(_, s)|{
                    s.groups.contains(name)
                }).collect::<Vec<_>>();

                for (_, s) in servers_in_group.iter() {
                    buf.push(ConfigAllow {
                        name: s.name.clone(),
                        v4: s.v4.clone()
                    });
                }

                return buf;
            }

            vec![]
        }

        let server_config = raw.server.iter().map(|server| {
            let mut allows = vec![];

            let server_group = server.groups.iter().filter_map(|g|{
                raw_groups.get(g)
            }).collect::<Vec<_>>();

            for g in server_group {
                for ga in g.allow.iter() {
                    allows.extend(
                        resolve_allow(&raw_servers, &raw_groups, ga, &mut vec![server.name.clone()])
                    )
                }
            }

            ConfigServer {
                name: server.name.clone(),
                public_key: server.public_key.clone(),
                endpoint: server.endpoint.clone(),
                v4: server.v4.clone(),
                allow: allows
            }

        }).collect();


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
}


#[cfg(test)]
mod tests {
    use super::*;

    fn load_config(content : &[u8]) -> Config {
        Config::new_from_bytes(content).expect("could not load config")
    }

    #[test]
    fn test_add() {
        let c = load_config(include_bytes!("./../example/basic.toml"));
    }
}