use crate::udp_echo::analyzer_event::{UdpEchoAnalyzerEventDatacenter, UdpEchoAnalyzerEventServer};
use crate::BroadcastEvent;
use crate::maintenance_mode::MaintenanceMode;

pub struct AnalyzerEventSubscriverStout {
    broadcast_recv: ::tokio::sync::broadcast::Receiver<BroadcastEvent>,
}

impl AnalyzerEventSubscriverStout {
    pub fn new(broadcast_recv: ::tokio::sync::broadcast::Receiver<BroadcastEvent>) -> Self {
        Self { broadcast_recv }
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
                    BroadcastEvent::UdpEchoAnalyzerEventDatacenter(e) => {
                        self.on_udp_echo_analyzer_event_datacenter(e).await
                    }
                },
            }
        }
    }

    pub async fn get_mode_info() -> &'static str {
        match MaintenanceMode::is_active().await {
            true => "MAINTENANCE",
            false => "normal",
        }
    }

    pub async fn on_udp_echo_analyzer_event_server(&self, event: UdpEchoAnalyzerEventServer) {

        let loss = event.req_count - event.resp_count;
        println!(
            "{} server: {}, ip: {}, req: {:?}, resp: {:?}, max_lat: {:?}, min_lat: {:?}, mode: {}, loss: {:?}, {}",
            event.date_time.format("%Y-%m-%d %H:%M:%S").to_string(),
            format!("{} -> {}", &event.server_from, &event.server_to),
            event.server_to_ip,
            event.req_count,
            event.resp_count,
            event.max_latency,
            event.min_latency,
            Self::get_mode_info().await,
            loss, if loss > 0 { "withloss" } else { "withoutloss"}
        );
    }
    pub async fn on_udp_echo_analyzer_event_datacenter(&self, event: UdpEchoAnalyzerEventDatacenter) {
        let loss = event.req_count - event.resp_count;
        println!(
            "{} datacenter: {}, req: {:?}, resp: {:?}, max_lat: {:?}, min_lat: {:?}, mode: {}, loss: {:?}, {}",
            event.date_time.format("%Y-%m-%d %H:%M:%S").to_string(),
            format!("{} -> {}", event.datacenter_from, event.datacenter_to),
            event.req_count,
            event.resp_count,
            event.max_latency,
            event.min_latency,
            Self::get_mode_info().await,
            loss, if loss > 0 { "withloss" } else { "withoutloss"}
        );
    }
}
