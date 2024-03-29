use crate::http::http_analyzer::HttpAnalyzerEvent;
use futures::channel::mpsc::Sender;
use futures::SinkExt;
use mehsh_common::config::ConfigCheck;
use std::time::Duration;
use tokio::time::sleep;

pub struct HttpCheck {
    config: ConfigCheck,
    http_analyzer_sender: Sender<HttpAnalyzerEvent>,
}

impl HttpCheck {
    pub fn new(config: ConfigCheck, http_analyzer_sender: Sender<HttpAnalyzerEvent>) -> Self {
        Self {
            config,
            http_analyzer_sender,
        }
    }

    pub async fn run(mut self) {
        loop {
            sleep(Duration::from_millis(1000)).await;

            let client = reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .expect("could not build http client");

            let res = client
                .get(
                    self.config
                        .http_url
                        .clone()
                        .expect("http check needs an http_url."),
                )
                .send()
                .await;

            let msg = match res {
                Ok(s) => HttpAnalyzerEvent::new(Ok(s.status())),
                Err(e) => HttpAnalyzerEvent::new(Err(format!("{}", e))),
            };

            match self.http_analyzer_sender.send(msg).await {
                Ok(_k) => {}
                Err(e) => println!("http sender error: {}", e),
            };
        }
    }
}
