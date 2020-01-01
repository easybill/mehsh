use std::net::{SocketAddr};
use std::{env, io};
use tokio;
use tokio::net::UdpSocket;
use failure::Error;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::time::Duration;
use tokio::time;


pub struct Client {
    remote_socket: SocketAddrV4
}

impl Client {

    pub async fn new(host : &str) -> Result<Self, Error>
    {
        let remote_socket : SocketAddrV4 = host.parse()?;
        Ok(Client {
            remote_socket
        })
    }

    pub async fn run(self) -> Result<(), Error> {

        let mut remote_socket = self.remote_socket;

        let local_socket : SocketAddrV4 = "0.0.0.0:0".parse()?;


        let mut socket = UdpSocket::bind(local_socket).await?;

        let mut interval = time::interval(Duration::from_millis(50));

        loop {
            println!("send ...");

            let data: Vec<u8> = vec![1, 2, 3];

            socket.send_to(&data, &remote_socket).await?;
            let mut data = vec![0u8; 100];

            let len = socket.recv(&mut data).await?;

            interval.tick().await;
        }

        Ok(())
    }
}