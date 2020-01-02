use std::net::{SocketAddr};
use std::{env, io};
use tokio;
use tokio::net::UdpSocket;
use failure::Error;
use std::net::{Ipv4Addr, SocketAddrV4};
use crate::check::udp_echo::packet::Packet;

pub struct Server {
    socket: UdpSocket,
    buf: Vec<u8>,
}

impl Server {

    pub async fn new(host : &str) -> Result<Self, Error>
    {
        let socket : SocketAddrV4 = host.parse()?;
        Ok(Server {
            socket: UdpSocket::bind(socket).await?,
            buf: vec![0; 100],
        })
    }

    pub async fn run(self) -> Result<(), Error> {

        let mut socket = self.socket;
        let mut buf = self.buf;

        loop {

            let (size, target) : (usize, SocketAddr) = socket.recv_from(&mut buf).await?;


            let reply = &buf[0..size];


            let recv_packet = Packet::new_from_raw(&buf[0..size]).expect("could not read package");
            let send_package = Packet::new_resp(recv_packet.get_id()).to_bytes();

            let mut send_size = 0;

            while send_size < size {
                match socket.send_to(&send_package, target).await {
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
