use futures::channel::mpsc::{Receiver, channel, Sender};
use crate::check::udp_echo::packet::Packet;
use crate::config::Config;
use std::time::{Duration, SystemTime};
use tokio::time;
use futures::future;
use futures::stream::StreamExt;
use futures::stream;
use futures::future::Either;

#[derive(Debug)]
pub struct AnalyzerEvent {
    remote_hostname: String,
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

