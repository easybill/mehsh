use crate::BroadcastEvent;

pub struct AnalyzerEventSubscriberAnalysis {
    broadcast_recv: ::tokio::sync::broadcast::Receiver<BroadcastEvent>
}

impl AnalyzerEventSubscriberAnalysis {
    pub fn new(broadcast_recv: ::tokio::sync::broadcast::Receiver<BroadcastEvent>) -> Self {
        Self {
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
                    BroadcastEvent::UdpEchoAnalyzerEventServer(e) => self.on_udp_echo_analyzer_event_server(e),
                    _ => {}
                }
            }
        }
    }
}