use failure::Error;
use crate::udp_echo::analyzer::AnalyzerEvent;
use futures::channel::mpsc::Sender;
use mehsh_common::config::ConfigCheck;


struct HttpCheck {
 config: ConfigCheck
}

impl HttpCheck {
    pub fn new(config: ConfigCheck, client_analyzer_sender : Sender<AnalyzerEvent>) -> Self
    {
        Self {
            config
        }
    }

    pub async fn run() {
        loop {

        }
    }
}