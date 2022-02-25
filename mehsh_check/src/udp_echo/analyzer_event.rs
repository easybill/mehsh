use chrono::{DateTime, Local};

pub struct UdpEchoAnalyzerEventServer {
    date_time: DateTime<Local>,
    server_from: String,
    server_to: String,
    server_to_ip: String,
    req_count : u16,
    resp_count : u16,
    max_latency : Option<u128>,
    min_latency : Option<u128>,
}


pub struct UdpEchoAnalyzerEventDatacenter {
    date_time: DateTime<Local>,
    server_from: String,
    datacenter_from: String,
    datacenter_to: String,
    server_to_ip: String,
    req_count : u16,
    resp_count : u16,
    max_latency : Option<u128>,
    min_latency : Option<u128>,
}