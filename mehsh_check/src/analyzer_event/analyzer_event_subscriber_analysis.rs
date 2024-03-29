use crate::udp_echo::analyzer_event::UdpEchoAnalyzerEventServer;
use crate::{BroadcastEvent, ExecuteAnalysisCommandHandler};
use chrono::{DateTime, Duration, Utc};
use mehsh_common::config::ConfigAnalysis;
use crate::maintenance_mode::MaintenanceMode;

pub struct AnalyzerEventSubscriberAnalysis {
    do_not_collect_until: DateTime<Utc>,
    config_analysis: ConfigAnalysis,
    broadcast_recv: ::tokio::sync::broadcast::Receiver<BroadcastEvent>,
    execute_analysis_command_handler: ExecuteAnalysisCommandHandler,
}

impl AnalyzerEventSubscriberAnalysis {
    pub fn new(
        config_analysis: ConfigAnalysis,
        broadcast_recv: ::tokio::sync::broadcast::Receiver<BroadcastEvent>,
    ) -> Self {
        Self {
            do_not_collect_until: Utc::now() + Duration::seconds(20),
            execute_analysis_command_handler: ExecuteAnalysisCommandHandler::new(
                config_analysis.clone(),
            ),
            config_analysis,
            broadcast_recv,
        }
    }

    pub async fn run(mut self) {
        loop {
            match self.broadcast_recv.recv().await {
                Err(e) => {
                    eprintln!("warning, broadcast std out issue: {}", e);
                }
                Ok(event) => match event {
                    BroadcastEvent::UdpEchoAnalyzerEventServer(e) => {
                        self.on_udp_echo_analyzer_event_server(e).await
                    }
                    _ => {}
                },
            }
        }
    }

    pub async fn on_udp_echo_analyzer_event_server(&mut self, event: UdpEchoAnalyzerEventServer) {
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

        if Utc::now() < self.do_not_collect_until {
            println!("analysis {} - skip already runned", &self.config_analysis.name);
            return;
        }

        if MaintenanceMode::is_active().await {
            println!("analysis {} - skip MAINTANCE MODE", &self.config_analysis.name);
            return;
        }

        self.execute_analysis_command_handler.run_if_not_running();

        self.do_not_collect_until = Utc::now() + Duration::seconds(120);
    }
}
