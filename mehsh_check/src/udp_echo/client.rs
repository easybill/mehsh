use std::net::{SocketAddr};
use tokio;
use tokio::net::UdpSocket;
use failure::Error;
use std::time::Duration;
use tokio::time;
use tokio::task;
use futures::future;
use futures::channel::mpsc::Sender;
use crate::udp_echo::analyzer::AnalyzerEvent;
use crate::udp_echo::packet::Packet;
use std::sync::Arc;
use mehsh_common::config::ConfigCheck;


pub struct Client {
    check: ConfigCheck,
    remote_socket: SocketAddr,
    client_analyzer_sender: Sender<AnalyzerEvent>,
    host: String
}

impl Client {

    pub async fn new(check: ConfigCheck, client_analyzer_sender : Sender<AnalyzerEvent>) -> Result<Self, Error>
    {
        let host = format!("{}:4232", check.to.ip.to_string());
        let remote_socket : SocketAddr = host.parse()?;
        Ok(Client {
            check,
            remote_socket,
            client_analyzer_sender,
            host
        })
    }

    pub async fn run(self) -> Result<(), Error> {

        let remote_socket = self.remote_socket;
        let local_socket : SocketAddr = "0.0.0.0:0".parse()?;

        let socket = UdpSocket::bind(local_socket).await?;

        let socket_recv = Arc::new(socket);
        let socket_send = socket_recv.clone();

        let mut send_client_analyzer_sender = self.client_analyzer_sender.clone();
        let server_ident = self.check.to.identifier.clone();
        let send_handle = task::spawn(async move {

            let mut interval = time::interval(Duration::from_millis(25));

            let mut counter : u64 = 0;

            loop {
                counter = counter + 1;

                let packet = Packet::new_req(counter);

                match send_client_analyzer_sender.try_send(AnalyzerEvent::new(server_ident.clone(), packet.clone())) {
                    Ok(_) => {},
                    Err(_e) => eprintln!("issue with the client_send_handle")
                };

                match socket_send.send_to(&packet.to_bytes(), &remote_socket).await {
                    Ok(_) => {},
                    Err(_e) => eprintln!("client: could not send package to {:?}", &remote_socket)
                }

                interval.tick().await;
            }
        });

        let mut recv_client_analyzer_sender = self.client_analyzer_sender.clone();
        let recv_ident = self.check.to.identifier.clone();
        let recv_handle = task::spawn(async move {

            let mut data = vec![0u8; 100];

            loop {

                let len = match socket_recv.recv(&mut data).await {
                    Ok(l) => l,
                    Err(_e) => {
                        eprintln!("could not recv socket {:?}", &socket_recv);
                        continue;
                    }
                };

                let packet = match Packet::new_from_raw(&data[0..len]) {
                    Ok(p) => p,
                    Err(_) => {
                        eprintln!("could not parse package {:?}, {:?}", &socket_recv, &data[0..len]);
                        continue;
                    }
                };

                match recv_client_analyzer_sender.try_send(AnalyzerEvent::new(recv_ident.clone(),packet.clone())) {
                    Ok(_) => {},
                    Err(_e) => eprintln!("issue with the client_send_handle")
                };

                // println!("client recv {:?}", &packet);
            }
        });


        future::select(send_handle, recv_handle).await;

        Ok(())
    }
}