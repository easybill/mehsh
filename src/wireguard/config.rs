use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct RawConfigWireguard {
    from: String,
    to: String,
}