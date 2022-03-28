use mehsh_common::config::ConfigAnalysis;
use crate::BroadcastEvent;
use crate::udp_echo::analyzer_event::UdpEchoAnalyzerEventServer;

pub struct AnalyzerEventSubscriberAnalysis {
    config_analysis: ConfigAnalysis,
    broadcast_recv: ::tokio::sync::broadcast::Receiver<BroadcastEvent>
}

impl AnalyzerEventSubscriberAnalysis {
    pub fn new(config_analysis: ConfigAnalysis, broadcast_recv: ::tokio::sync::broadcast::Receiver<BroadcastEvent>) -> Self {
        Self {
            config_analysis,
            broadcast_recv
        }
    }

    pub async fn run(mut self) {
        loop {
            match self.broadcast_recv.recv().await {
                Err(e) => {
                    eprintln!("warning, broadcast std out issue: {}", e);
                },
                Ok(event) => match event {
                    BroadcastEvent::UdpEchoAnalyzerEventServer(e) => self.on_udp_echo_analyzer_event_server(e).await,
                    _ => {}
                }
            }
        }
    }

    pub async fn on_udp_echo_analyzer_event_server(&self, event: UdpEchoAnalyzerEventServer) {

    }
}