use futures::channel::mpsc::{Receiver, channel, Sender};
use crate::check::udp_echo::packet::{Packet, PacketType};
use crate::config::Config;
use std::time::{Duration, SystemTime};
use tokio::time;
use futures::future;
use futures::stream::StreamExt;
use futures::stream;
use futures::future::Either;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::alloc::System;

type RemoteHost = String;

#[derive(Debug)]
pub struct AnalyzerEvent {
    remote_hostname: RemoteHost,
    packet: Packet,
}

impl AnalyzerEvent {
    pub fn new(remote_hostname: String, packet: Packet) -> Self {
        AnalyzerEvent {
            remote_hostname,
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
        let (sender, receiver) = channel(100);

        Analyzer {
            config,
            receiver,
            sender,
        }
    }

    pub fn get_sender_handle(&self) -> Sender<AnalyzerEvent> {
        self.sender.clone()
    }

    pub async fn run(mut self) {
        let mut interval = time::interval(Duration::from_millis(60_000)).map(|x| Either::Left(x));
        let recv = self.receiver.map(|x| Either::Right(x));

        let mut sel = stream::select(interval, recv);
        let mut analyzer_stats = AnalyzerStats::new();

        loop {
            match sel.next().await {
                Some(Either::Left(_)) => {
                    let data = analyzer_stats.slice();
                    AnalyzerStats::aggrrgate(data);
                    // println!("data: {:?}", data);
                }
                Some(Either::Right(p)) => {
                    analyzer_stats.add_event(p);
                }
                None => {}
            };
        }
    }
}

#[derive(Debug)]
struct AnalyzerStatsEntry {
    remote_host: String,
    req_time: Option<SystemTime>,
    resp_time: Option<SystemTime>,
    packet_type: PacketType
}

impl AnalyzerStatsEntry {
    pub fn calculate_latency(&self) -> Option<u128> {
        match (self.req_time, self.resp_time) {
            (Some(req), Some(resp))  => {
                Some(resp.duration_since(req).expect("could not calculate duration").as_micros())
            },
            _ => None
        }
    }

}

struct AnalyzerStats {
    map: HashMap<SystemTime, HashMap<(String, u64), AnalyzerStatsEntry>>,
}

impl AnalyzerStats {
    pub fn new() -> AnalyzerStats {
        AnalyzerStats {
            map: HashMap::new()
        }
    }

    pub fn add_event(&mut self, event: AnalyzerEvent) {
        let now = SystemTime::now();

        let mut time_map = self.map.entry(now.clone()).or_insert(HashMap::new());

        match time_map.entry((event.remote_hostname.clone(), event.packet.get_id())) {
            Entry::Vacant(e) => {
                let stats_entry = match event.packet.get_type() {
                    &PacketType::Req => {
                        AnalyzerStatsEntry {
                            remote_host: event.remote_hostname,
                            req_time: Some(now.clone()),
                            resp_time: None,
                            packet_type: PacketType::Req
                        }
                    }
                    &PacketType::Resp => {
                        AnalyzerStatsEntry {
                            remote_host: event.remote_hostname,
                            req_time: None,
                            resp_time: Some(now.clone()),
                            packet_type: PacketType::Resp
                        }
                    }
                };

                e.insert(stats_entry);
            }
            Entry::Occupied(mut e) => {
                match event.packet.get_type() {
                    &PacketType::Req => {
                        e.get_mut().req_time = Some(now);
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
        let mut now = SystemTime::now();

        let mut old_map = HashMap::new();

        ::std::mem::swap(&mut self.map, &mut old_map);

        for (time, m) in old_map.into_iter() {
            let dur = match now.duration_since(time) {
                Err(_) => { continue; }
                Ok(d) => { d }
            };

            if dur.as_secs() < 10 {
                self.map.insert(time, m);
                continue;
            }

            for (_, stats_entry) in m.into_iter() {
                data.push(stats_entry);
            }
        }

        data
    }

    pub fn aggrrgate(stats_entries: Vec<AnalyzerStatsEntry>)
    {
        let mut map = HashMap::new();
        for entry in stats_entries.into_iter() {
            match map.entry(entry.remote_host.clone()) {
                Entry::Vacant(e) => {
                    let latency = entry.calculate_latency();
                    e.insert(AggregatedStatsEntry {
                        remote_host: entry.remote_host,
                        req_count: if entry.packet_type == PacketType::Req { 1 } else { 0 },
                        resp_count: if entry.packet_type == PacketType::Resp { 1 } else { 0 },
                        min_latency: latency,
                        max_latency: latency,
                    });
                }
                Entry::Occupied(mut e) => {

                    let mut_entry = e.get_mut();

                    if entry.packet_type == PacketType::Req {
                        mut_entry.req_count += 1;
                    }
                    if entry.packet_type == PacketType::Resp {
                        mut_entry.resp_count += 1;
                    }

                    let latency = entry.calculate_latency();

                    match (latency, mut_entry.min_latency) {
                        (None, None) => {},
                        (None, Some(new)) => { mut_entry.min_latency = Some(new) }
                        (Some(curr), Some(new)) if new < curr => { mut_entry.min_latency = Some(new) }
                        _ => {}
                    };

                    match (latency, mut_entry.max_latency) {
                        (None, None) => {},
                        (None, Some(new)) => { mut_entry.min_latency = Some(new) }
                        (Some(curr), Some(new)) if new > curr => { mut_entry.max_latency = Some(new) }
                        _ => {}
                    };
                }
            }
        }

        for (_, item) in map.iter() {
            println!("host: {}, req: {:?}, resp: {:?}, max_lat: {:?}, min_lat: {:?}", item.remote_host, item.req_count, item.resp_count, item.max_latency, item.min_latency);
        }
    }
}

struct AggregatedStatsEntry {
    remote_host: String,
    req_count: u16,
    resp_count: u16,
    min_latency: Option<u128>,
    max_latency: Option<u128>,
    // avg_latency: u64
}