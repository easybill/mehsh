use futures::channel::mpsc::{Receiver, channel, Sender};
use crate::check::udp_echo::packet::Packet;
use tokio::stream::StreamExt;
use crate::config::Config;

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

    pub async fn run(&mut self) {
        loop {
            match self.receiver.next().await {
                None => {
                    continue;
                },
                Some(d) => {
                    println!("foo {:?}", d)
                }
            }
        }
    }
}

