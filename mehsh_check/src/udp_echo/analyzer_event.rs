use chrono::{DateTime, Local};

#[derive(Clone, Debug)]
pub struct UdpEchoAnalyzerEventServer {
    pub date_time: DateTime<Local>,
    pub server_from: String,
    pub server_to: String,
    pub server_to_ip: String,
    pub req_count: u16,
    pub resp_count: u16,
    pub max_latency: Option<u128>,
    pub min_latency: Option<u128>,
}

#[derive(Clone, Debug)]
pub struct UdpEchoAnalyzerEventDatacenter {
    pub date_time: DateTime<Local>,
    pub server_from: String,
    pub datacenter_from: String,
    pub datacenter_to: String,
    pub req_count: u16,
    pub resp_count: u16,
    pub max_latency: Option<u128>,
    pub min_latency: Option<u128>,
}
