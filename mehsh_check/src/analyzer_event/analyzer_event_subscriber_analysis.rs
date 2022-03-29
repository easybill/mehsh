use mehsh_common::config::ConfigAnalysis;
use crate::{BroadcastEvent, ExecuteAnalysisCommandHandler};
use crate::udp_echo::analyzer_event::UdpEchoAnalyzerEventServer;

pub struct AnalyzerEventSubscriberAnalysis {
    config_analysis: ConfigAnalysis,
    broadcast_recv: ::tokio::sync::broadcast::Receiver<BroadcastEvent>,
    execute_analysis_command_handler: ExecuteAnalysisCommandHandler,
}

impl AnalyzerEventSubscriberAnalysis {
    pub fn new(config_analysis: ConfigAnalysis, broadcast_recv: ::tokio::sync::broadcast::Receiver<BroadcastEvent>) -> Self {
        Self {
            execute_analysis_command_handler: ExecuteAnalysisCommandHandler::new(config_analysis.clone()),
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
        if event.server_to != self.config_analysis.to.identifier {
            return;
        }
        if event.server_from != self.config_analysis.from.identifier {
            return;
        }

        let loss = event.req_count - event.resp_count;

        if (loss as u32) < self.config_analysis.min_loss {
            return;
        }

        self.execute_analysis_command_handler.run_if_not_running();
    }
}