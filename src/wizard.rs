use socket2::{Domain, Protocol, Socket, Type};

use local_ip_address::local_ip;
use std::{mem::MaybeUninit, net::SocketAddr, time::Duration};

use crate::{
    bulb::Bulb,
    pilot::{Method, Pilot},
};
const WIZARD_PORT: u16 = 38899;
pub struct Wizard {
    socket: Socket,
}

impl Wizard {
    pub fn new() -> Wizard {
        let addr: SocketAddr = format!("0.0.0.0:{}", WIZARD_PORT).parse().unwrap();

        let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP)).unwrap();
        socket.set_reuse_address(true).unwrap();
        socket.set_broadcast(true).unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        socket.bind(&addr.into()).unwrap();

        Wizard { socket }
    }

    pub fn set_pilot(&self, bulb: Bulb, pilot: Pilot) {
        let data = pilot.build();
        let addr: SocketAddr = format!("{}:{}", bulb.ip, WIZARD_PORT).parse().unwrap();
        self.socket
            .send_to(data.to_string().as_bytes(), &addr.into())
            .unwrap();
    }

    pub fn cleanup(&self) {
        self.socket.shutdown(std::net::Shutdown::Both).unwrap();
    }

    pub fn discover(&self) -> Vec<Bulb> {
        let pilot = Pilot::new(Method::GetDevInfo);
        let addr: SocketAddr = format!("192.168.1.255:{}", WIZARD_PORT).parse().unwrap();
        self.socket
            .send_to(pilot.build().as_bytes(), &addr.into())
            .unwrap();

        let local_ip = local_ip().unwrap();

        let mut bulbs: Vec<Bulb> = Vec::new();

        let mut buf = [MaybeUninit::new(0u8); 1024];
        loop {
            match self.socket.recv_from(&mut buf) {
                Ok((_amt, src)) => {
                    let src_ip = src.as_socket_ipv4().unwrap();
                    if src_ip.ip() == &local_ip {
                        continue;
                    }
                    let pbuf = buf.map(|c| unsafe { c.assume_init() });
                    let data = String::from_utf8(pbuf.to_vec()).unwrap();
                    let bulb = Bulb::parse(src_ip.ip().to_string(), &data);
                    bulbs.push(bulb);
                }
                Err(_e) => {
                    // println!("Error: {}", e);
                    break;
                }
            }
        }

        bulbs
    }
}
