use std::net::{SocketAddr};
use std::{env, io};
use tokio;
use tokio::net::UdpSocket;
use failure::Error;
use std::net::{Ipv4Addr, SocketAddrV4};

pub struct Server {
    socket: UdpSocket,
    buf: Vec<u8>,
}

impl Server {

    pub async fn new() -> Result<Self, Error>
    {
        let socket : SocketAddrV4 = "0.0.0.0:4232".parse()?;
        Ok(Server {
            socket: UdpSocket::bind(socket).await?,
            buf: vec![0; 1024],
        })
    }

    pub async fn run(self) -> Result<(), Error> {

        let mut socket = self.socket;
        let mut buf = self.buf;

        loop {

            let (size, target) : (usize, SocketAddr) = socket.recv_from(&mut buf).await?;

            let reply = &buf[0..size];
            let mut send_size = 0;

            while send_size < size {
                match socket.send_to(&buf[send_size..size], target).await {
                    Ok(s) => {
                        send_size = send_size + s;
                    }
                    Err(e) => {
                        break;
                    }
                };
            }
        }
    }
}
