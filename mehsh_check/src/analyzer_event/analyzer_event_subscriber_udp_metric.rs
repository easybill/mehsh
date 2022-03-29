use crate::udp_echo::analyzer_event::{UdpEchoAnalyzerEventDatacenter, UdpEchoAnalyzerEventServer};
use crate::BroadcastEvent;
use anyhow::anyhow;
use serverdensity_udpserver_lib::create_package_sum;
use std::net::SocketAddrV4;
use tokio::net::UdpSocket;

const UDPSERVER_ENDPOINT: &str = "127.0.0.1:1113";

pub struct AnalyzerEventSubscriberUdpMetric {
    broadcast_recv: ::tokio::sync::broadcast::Receiver<BroadcastEvent>,
}

impl AnalyzerEventSubscriberUdpMetric {
    pub fn new(broadcast_recv: ::tokio::sync::broadcast::Receiver<BroadcastEvent>) -> Self {
        Self { broadcast_recv }
    }

    pub async fn run(mut self) {
        let mut sock = UdpSocket::bind("0.0.0.0:0")
            .await
            .expect("could not register Metric Udp Socker. should never happen.");

        loop {
            match self.broadcast_recv.recv().await {
                Err(e) => {
                    eprintln!("warning, broadcast std out issue: {}", e);
                }
                Ok(event) => match event {
                    BroadcastEvent::UdpEchoAnalyzerEventServer(e) => {
                        match self.on_udp_echo_analyzer_event_server(&mut sock, e).await {
                            Ok(_) => {}
                            Err(e) => {
                                eprintln!("mehsh could not send udp metrics: {}", e);
                            }
                        }
                    }
                    BroadcastEvent::UdpEchoAnalyzerEventDatacenter(e) => {
                        self.on_udp_echo_analyzer_event_datacenter(&mut sock, e);
                    }
                },
            };
        }
    }

    pub async fn on_udp_echo_analyzer_event_server(
        &self,
        sock: &mut UdpSocket,
        event: UdpEchoAnalyzerEventServer,
    ) -> Result<(), ::anyhow::Error> {
        let loss = event.req_count - event.resp_count;
        let target: SocketAddrV4 = "127.0.0.1:1113".parse()?;

        sock.send_to(
            create_package_sum("mehsh.loss", loss as i32)
                .map_err(|e| anyhow!(e))?
                .as_slice(),
            target,
        )
        .await?;
        sock.send_to(
            create_package_sum(
                format!("mehsh.sendloss.{}", &event.server_from),
                loss as i32,
            )
            .map_err(|e| anyhow!(e))?
            .as_slice(),
            target,
        )
        .await?;
        sock.send_to(
            create_package_sum(format!("mehsh.recvloss.{}", &event.server_to), loss as i32)
                .map_err(|e| anyhow!(e))?
                .as_slice(),
            target,
        )
        .await?;

        Ok(())
    }

    pub fn on_udp_echo_analyzer_event_datacenter(
        &self,
        _sock: &mut UdpSocket,
        _event: UdpEchoAnalyzerEventDatacenter,
    ) {
    }
}
