use futures::channel::mpsc::{Receiver, channel, Sender};
use std::time::{Duration, SystemTime};
use tokio::time;
use futures::stream::StreamExt;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use crate::udp_echo::packet::{Packet, PacketType};
use mehsh_common::config::{Config, ServerIdentifier};
use chrono::Local;

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

    pub async fn run(self) {
        let mut interval = time::interval(Duration::from_millis(5_000));
        let mut recv = self.receiver;

        let mut analyzer_stats = AnalyzerStats::new(self.config.clone());

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    let data = analyzer_stats.slice();
                    analyzer_stats.aggregate(data);
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
            map: HashMap::new()
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

    pub fn aggregate(&self, stats_entries: Vec<AnalyzerStatsEntry>)
    {
        let mut map = HashMap::new();
        for entry in stats_entries.into_iter() {

            match map.entry(entry.server_identifier.clone()) {
                Entry::Vacant(e) => {
                    let latency = entry.calculate_latency();
                    e.insert(AggregatedStatsEntry {
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

        for (_, item) in map.iter() {
            let loss = item.req_count - item.resp_count;
            let server_name = self.config.get_server_by_identifier(&item.remote_server_identifier)
                .and_then(|v|Some(format!("{} ({})", item.remote_server_identifier, v.ip)))
                .unwrap_or(item.remote_server_identifier.clone());

            println!("{} host: {}, req: {:?}, resp: {:?}, max_lat: {:?}, min_lat: {:?}, loss: {:?}, {}", Local::now().format("%Y-%m-%d %H:%M:%S").to_string(), server_name, item.req_count, item.resp_count, item.max_latency, item.min_latency, loss, if loss > 0 { "withloss" } else { "withoutloss"});
        }
    }
}

struct AggregatedStatsEntry {
    remote_server_identifier: ServerIdentifier,
    req_count: u16,
    resp_count: u16,
    min_latency: Option<u128>,
    max_latency: Option<u128>,
    // avg_latency: u64
}