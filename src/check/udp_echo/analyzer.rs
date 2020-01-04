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
    packet: Packet
}

impl AnalyzerEvent {
    pub fn new(remote_hostname : String, packet : Packet) -> Self {
        AnalyzerEvent {
            remote_hostname,
            packet
        }
    }
}

pub struct Analyzer {
    config: Config,
    receiver: Receiver<AnalyzerEvent>,
    sender: Sender<AnalyzerEvent>
}

impl Analyzer {
    pub fn new(config : Config) -> Self {
        let (sender, receiver) = channel(100);

        Analyzer {
            config,
            receiver,
            sender
        }
    }

    pub fn get_sender_handle(&self) -> Sender<AnalyzerEvent> {
        self.sender.clone()
    }

    pub async fn run(mut self) {
        let mut interval = time::interval(Duration::from_millis(250)).map(|x|Either::Left(x));
        let recv = self.receiver.map(|x|Either::Right(x));

        let mut sel = stream::select(interval, recv);

        loop {

            // todo, sliding window!

            match sel.next().await {
                Some(Either::Left(_)) => println!("interval!"),
                Some(Either::Right(p)) => {
                    println!("data {:?}", p);
                }
                None => {}
            };

        }
    }
}

struct AnalyzerStatsEntry {
    req_time: Option<SystemTime>,
    resp_time: Option<SystemTime>,
}

struct AnalyzerStats {
    map: HashMap<SystemTime, HashMap<(String, u64), AnalyzerStatsEntry>>,
}

impl AnalyzerStats {
    pub fn add_event(&mut self, event : AnalyzerEvent) {
        let now = SystemTime::now();

        let mut time_map = self.map.entry(now.clone()).or_insert(HashMap::new());

        match time_map.entry((event.remote_hostname, event.packet.get_id())) {
            Entry::Vacant(e) => {

                let stats_entry = match event.packet.get_type() {
                    &PacketType::Req => {
                        AnalyzerStatsEntry {
                            req_time: Some(now.clone()),
                            resp_time: None,
                        }
                    },
                    &PacketType::Resp => {
                        AnalyzerStatsEntry {
                            req_time: None,
                            resp_time: Some(now.clone()),
                        }
                    },
                };

                e.insert(stats_entry);
            },
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

    pub fn slice(&mut self) {

    }


}