use std::net::{SocketAddr};
use std::{env, io};
use tokio;
use tokio::net::UdpSocket;
use failure::Error;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::time::Duration;
use tokio::net::udp::{RecvHalf, SendHalf};
use tokio::time;
use tokio::runtime::Runtime;
use tokio::task;
use futures::future;
use crate::check::udp_echo::packet::Packet;


pub struct Client {
    remote_socket: SocketAddr
}

impl Client {

    pub async fn new(host : &str) -> Result<Self, Error>
    {
        let remote_socket : SocketAddr = host.parse()?;
        Ok(Client {
            remote_socket
        })
    }

    pub async fn run(self) -> Result<(), Error> {

        let mut remote_socket = self.remote_socket;

        let local_socket : SocketAddr = "0.0.0.0:0".parse()?;


        let mut socket = UdpSocket::bind(local_socket).await?;

        let (mut socker_recv, mut socket_send) : (RecvHalf, SendHalf) = socket.split();

        let send_handle = task::spawn(async move {

            let mut interval = time::interval(Duration::from_millis(50));

            let mut counter : u64 = 0;

            loop {
                counter = counter + 1;

                let packet = Packet::new_req(counter);

                println!("client send {:?}", &packet);

                socket_send.send_to(&packet.to_bytes(), &remote_socket).await;

                interval.tick().await;
            }
        });

        let recv_handle = task::spawn(async move {

            let mut interval = time::interval(Duration::from_millis(50));
            let mut data = vec![0u8; 100];

            loop {

                let len = socker_recv.recv(&mut data).await.expect("...");

                let package = Packet::new_from_raw(&data[0..len]).expect("could not parse");

                println!("client recv {:?}", &package);

                interval.tick().await;
            }
        });


        future::select(send_handle, recv_handle).await;

        Ok(())
    }
}