use std::net::{SocketAddr};
use std::{env, io};
use tokio;
use tokio::net::UdpSocket;
use failure::Error;
use std::net::{Ipv4Addr, SocketAddrV4};
use crate::check::udp_echo::packet::Packet;
use rand::Rng;

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

    pub async fn run(mut self) -> Result<(), Error> {

        loop {
            match self.run_loop().await {
                Err(e) => { eprintln!("server err: {:?}", e) },
                _ => ()
            };
        }
    }

    async fn run_loop(&mut self) -> Result<(), Error>  {

        let (size, target) : (usize, SocketAddr) = self.socket.recv_from(&mut self.buf).await?;

        let recv_packet = Packet::new_from_raw(&self.buf[0..size]).expect("could not read package");
        let send_package = Packet::new_resp(recv_packet.get_id()).to_bytes();

        let mut send_size = 0;

        {
            let mut rng = rand::thread_rng();
            if rng.gen_range(0, 10) > 5 {
                return Ok(());
            }
        }

        while send_size < size {
            match self.socket.send_to(&send_package, target).await {
                Ok(s) => {
                    send_size = send_size + s;
                }
                Err(e) => return Err(e.into())
            };
        }

        Ok(())
    }
}
