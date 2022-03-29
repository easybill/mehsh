use crate::udp_echo::analyzer_event::{UdpEchoAnalyzerEventDatacenter, UdpEchoAnalyzerEventServer};

#[derive(Clone, Debug)]
pub enum BroadcastEvent {
    UdpEchoAnalyzerEventServer(UdpEchoAnalyzerEventServer),
    UdpEchoAnalyzerEventDatacenter(UdpEchoAnalyzerEventDatacenter),
}
