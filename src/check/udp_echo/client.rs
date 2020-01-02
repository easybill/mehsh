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

                socket_send.send_to(&[counter as u8], &remote_socket).await;
                println!("client send ...");


                interval.tick().await;
            }
        });

        let recv_handle = task::spawn(async move {

            let mut interval = time::interval(Duration::from_millis(50));

            loop {
                let mut data = vec![0u8; 100];

                let len = socker_recv.recv(&mut data).await;
                println!("client recv ...");

                interval.tick().await;
            }
        });


        future::select(send_handle, recv_handle).await;

        Ok(())
    }
}