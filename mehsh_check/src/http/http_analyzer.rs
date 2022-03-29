use futures::channel::mpsc::{channel, Receiver, Sender};
use futures::StreamExt;
use mehsh_common::config::Config;
use reqwest::StatusCode;
use tokio::time::Duration;

pub struct HttpAnalyzerEvent {
    status: Result<StatusCode, String>,
}

impl HttpAnalyzerEvent {
    pub fn new(status: Result<StatusCode, String>) -> Self {
        HttpAnalyzerEvent { status }
    }
}

pub struct HttpAnalyzer {
    config: Config,
    receiver: Receiver<HttpAnalyzerEvent>,
    sender: Sender<HttpAnalyzerEvent>,
}

impl HttpAnalyzer {
    pub fn new(config: Config) -> Self {
        let (sender, receiver) = channel(1000);

        Self {
            config,
            receiver,
            sender,
        }
    }

    pub fn get_sender_handle(&self) -> Sender<HttpAnalyzerEvent> {
        self.sender.clone()
    }

    pub async fn run(self) {
        let mut interval = ::tokio::time::interval(Duration::from_millis(5_000));
        let mut recv = self.receiver;

        loop {
            ::tokio::select! {
                _ = interval.tick() => {
                    // ...
                }
                _p = recv.next() => {
                    // ..
                }
            }
        }
    }
}
