use bytes::{Buf, BufMut, BytesMut};
use std::mem::size_of;
use anyhow::anyhow;

const PACKAGE_MAGIC: u32 = 326134347;

const PACKAGE_SIZE: usize =
    size_of::<u32>() + size_of::<u32>() + size_of::<u64>() + size_of::<u16>();

#[derive(Debug, Clone, PartialEq)]
pub enum PacketType {
    Req,
    Resp,
}

#[derive(Debug, Clone)]
pub struct Packet {
    version: u32,
    id: u64,
    packet_type: PacketType,
}

impl Packet {
    pub fn new_req(id: u64) -> Self {
        Packet {
            version: 1,
            id,
            packet_type: PacketType::Req,
        }
    }

    pub fn new_resp(id: u64) -> Self {
        Packet {
            version: 1,
            id,
            packet_type: PacketType::Resp,
        }
    }

    pub fn to_bytes(self) -> Vec<u8> {
        let mut buf = BytesMut::with_capacity(PACKAGE_SIZE);
        buf.put_u32(PACKAGE_MAGIC);
        buf.put_u32(self.version);
        buf.put_u64(self.id);
        buf.put_u16(match self.packet_type {
            PacketType::Req => 1,
            PacketType::Resp => 2,
        });

        buf.to_vec()
    }

    pub fn new_from_raw(data: &[u8]) -> Result<Self, ::anyhow::Error> {
        if data.len() < PACKAGE_SIZE {
            return Err(anyhow!("invalid packet size"));
        }

        let mut buf = &data[..];

        let magic_byte = buf.get_u32();

        if magic_byte != PACKAGE_MAGIC {
            return Err(anyhow!("invalid packet magic"));
        }

        let version = buf.get_u32();
        let id = buf.get_u64();
        let packet_type = match buf.get_u16() {
            1 => PacketType::Req,
            2 => PacketType::Resp,
            _ => return Err(anyhow!("unknown packet type")),
        };

        Ok(Packet {
            version,
            id,
            packet_type,
        })
    }

    pub fn get_version(&self) -> u32 {
        self.version
    }

    pub fn get_id(&self) -> u64 {
        self.id
    }

    pub fn get_type(&self) -> &PacketType {
        &self.packet_type
    }
}
