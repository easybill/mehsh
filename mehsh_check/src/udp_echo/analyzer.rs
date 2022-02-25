use std::cmp::min;
use futures::channel::mpsc::{Receiver, channel, Sender};
use std::time::{Duration, SystemTime};
use tokio::time;
use futures::stream::StreamExt;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use crate::udp_echo::packet::{Packet, PacketType};
use mehsh_common::config::{Config, ServerIdentifier};
use chrono::Local;
use crate::BroadcastEvent;
use crate::udp_echo::analyzer_event::{UdpEchoAnalyzerEventDatacenter, UdpEchoAnalyzerEventServer};

type RemoteHost = String;

#[derive(Debug)]
pub struct AnalyzerEvent {
    server_identifier: String,
    packet: Packet,
}

impl AnalyzerEvent {
    pub fn new(server_identifier : ServerIdentifier, packet: Packet) -> Self {
        AnalyzerEvent {
            server_identifier,
            packet,
        }
    }
}

pub struct Analyzer {
    config: Config,
    receiver: Receiver<AnalyzerEvent>,
    sender: Sender<AnalyzerEvent>,
}

impl Analyzer {
    pub fn new(config: Config) -> Self {
        let (sender, receiver) = channel(1000);

        Analyzer {
            config,
            receiver,
            sender,
        }
    }

    pub fn get_sender_handle(&self) -> Sender<AnalyzerEvent> {
        self.sender.clone()
    }

    pub async fn run(self, mut broadcast : ::tokio::sync::broadcast::Sender<BroadcastEvent>) {
        let mut interval = time::interval(Duration::from_millis(5_000));
        let mut recv = self.receiver;

        let mut analyzer_stats = AnalyzerStats::new(self.config.clone());

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    let data = analyzer_stats.slice();
                    analyzer_stats.aggregate(data, &mut broadcast);
                }
                p = recv.next() => {
                    if let Some(msg) = p {
                        analyzer_stats.add_event(msg);
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
struct AnalyzerStatsEntry {
    server_identifier: ServerIdentifier,
    req_time: SystemTime,
    resp_time: Option<SystemTime>,
}

impl AnalyzerStatsEntry {
    pub fn calculate_latency(&self) -> Option<u128> {
        match self.resp_time {
            Some(resp)  => {
                Some(resp.duration_since(self.req_time).expect("could not calculate duration").as_micros())
            },
            _ => None
        }
    }

}

struct AnalyzerStats {
    config: Config,
    map: HashMap<(String, u64), AnalyzerStatsEntry>,
}

impl AnalyzerStats {
    pub fn new(config: Config) -> AnalyzerStats {
        AnalyzerStats {
            config,
            map: HashMap::new(),
        }
    }

    pub fn add_event(&mut self, event: AnalyzerEvent) {
        let now = SystemTime::now();

        match self.map.entry((event.server_identifier.clone(), event.packet.get_id())) {
            Entry::Vacant(e) => {
                let stats_entry = match event.packet.get_type() {
                    &PacketType::Req => {
                        AnalyzerStatsEntry {
                            server_identifier: event.server_identifier,
                            req_time: now.clone(),
                            resp_time: None,
                        }
                    }
                    &PacketType::Resp => {
                        // got a response without a request. ignore ...
                        return;
                    }
                };

                e.insert(stats_entry);
            }
            Entry::Occupied(mut e) => {
                match event.packet.get_type() {
                    &PacketType::Req => {
                        // got request twice? doesnt make sense.
                        return;
                    }
                    &PacketType::Resp => {
                        e.get_mut().resp_time = Some(now);
                    }
                }
            }
        }
    }

    pub fn slice(&mut self) -> Vec<AnalyzerStatsEntry> {
        let mut data = vec![];
        let now = SystemTime::now();

        let mut old_map = HashMap::new();

        ::std::mem::swap(&mut self.map, &mut old_map);

        for (k, m) in old_map.into_iter() {
            let dur = match now.duration_since(m.req_time) {
                Err(_) => { continue; }
                Ok(d) => { d }
            };

            if dur.as_secs() < 1 {
                self.map.insert(k, m);
                continue;
            }

            data.push(m);
        }

        data
    }

    pub fn aggregate(&self, stats_entries: Vec<AnalyzerStatsEntry>, broadcast: &mut ::tokio::sync::broadcast::Sender<BroadcastEvent>)
    {
        let mut map = HashMap::new();
        for entry in stats_entries.into_iter() {

            match map.entry(entry.server_identifier.clone()) {
                Entry::Vacant(e) => {
                    let latency = entry.calculate_latency();
                    e.insert(AggregatedServerStatsEntry {
                        remote_server_identifier: entry.server_identifier,
                        req_count: 1,
                        resp_count: if entry.resp_time.is_some() { 1 } else { 0 },
                        min_latency: latency,
                        max_latency: latency,
                    });
                }
                Entry::Occupied(mut e) => {

                    let mut_entry = e.get_mut();

                    if entry.resp_time.is_some() {
                        mut_entry.resp_count += 1;
                    }

                    mut_entry.req_count += 1;

                    let latency = entry.calculate_latency();

                    match (mut_entry.min_latency, latency) {
                        (None, None) => {},
                        (None, Some(new)) => { mut_entry.min_latency = Some(new) }
                        (Some(curr), Some(new)) if new < curr => { mut_entry.min_latency = Some(new) }
                        _ => {}
                    };

                    match (mut_entry.max_latency, latency) {
                        (None, None) => {},
                        (None, Some(new)) => { mut_entry.min_latency = Some(new) }
                        (Some(curr), Some(new)) if new > curr => { mut_entry.max_latency = Some(new) }
                        _ => {}
                    };
                }
            }
        }

        // losses by server
        let server_self = self.config.get_server_self();
        for (_, item) in map.iter() {
            let server_info = self.config.get_server_by_identifier(&item.remote_server_identifier).expect("could not find server, should never happen");
            let server_ip = server_info.ip.clone();

            match broadcast.send(BroadcastEvent::UdpEchoAnalyzerEventServer(UdpEchoAnalyzerEventServer {
                date_time: Local::now(),
                server_from: server_self.identifier.to_string(),
                server_to: item.remote_server_identifier.to_string(),
                server_to_ip: server_ip,
                req_count : item.req_count,
                resp_count : item.resp_count,
                max_latency : item.max_latency,
                min_latency : item.min_latency,
            })) {
                Ok(_) => {},
                Err(e) => {
                    eprintln!("warning, issue with broadcasting server event: {:?}", e)
                }
            };
        }

        let datacenter_map = {
            let mut buffer : HashMap<String, AggregatedDatacenterStatsEntry> = HashMap::new();

            // losses by dc
            for (_, item) in map.iter() {
                let server_info = self.config.get_server_by_identifier(&item.remote_server_identifier).expect("could not find server, should never happen");

                for datacenter in &server_info.datacenter_as_entries {
                    match buffer.entry(datacenter.to_string()) {
                        Entry::Vacant(e) => {
                            e.insert(AggregatedDatacenterStatsEntry {
                                datacenter: datacenter.to_string(),
                                req_count: item.req_count,
                                resp_count: item.resp_count,
                                min_latency: item.min_latency,
                                max_latency: item.max_latency,
                            });
                        }
                        Entry::Occupied(mut e) => {
                            let mut_entry = e.get_mut();

                            mut_entry.req_count += item.req_count;
                            mut_entry.resp_count += item.resp_count;
                            mut_entry.min_latency = min(item.min_latency, mut_entry.min_latency);
                            mut_entry.max_latency = min(item.max_latency, mut_entry.max_latency);
                        }
                    };
                }
            }

            buffer
        };

        let datacenter_self = self.config.get_server_self().datacenter.clone().unwrap_or("".to_string());
        for (_, item) in datacenter_map.iter() {
            match broadcast.send(BroadcastEvent::UdpEchoAnalyzerEventDatacenter(UdpEchoAnalyzerEventDatacenter {
                date_time: Local::now(),
                server_from: server_self.identifier.to_string(),
                datacenter_from: datacenter_self.to_string(),
                datacenter_to: item.datacenter.to_string(),
                req_count : item.req_count,
                resp_count : item.resp_count,
                max_latency : item.max_latency,
                min_latency : item.min_latency,
            })) {
                Ok(_) => {},
                Err(e) => {
                    eprintln!("warning, issue with broadcasting datacenter event: {:?}", e)
                }
            };
        }

    }
}

struct AggregatedServerStatsEntry {
    remote_server_identifier: ServerIdentifier,
    req_count: u16,
    resp_count: u16,
    min_latency: Option<u128>,
    max_latency: Option<u128>,
    // avg_latency: u64
}

struct AggregatedDatacenterStatsEntry {
    datacenter: String,
    req_count: u16,
    resp_count: u16,
    min_latency: Option<u128>,
    max_latency: Option<u128>,
    // avg_latency: u64
}