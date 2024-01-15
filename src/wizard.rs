use ipnet::Ipv4Net;
use socket2::{Domain, Protocol, Socket, Type};

use std::io::Write;

use local_ip_address::local_ip;
use std::sync::{Arc, Mutex};
use std::thread;
use std::{mem::MaybeUninit, net::SocketAddr, time::Duration};

use interprocess::local_socket::LocalSocketStream;

use crate::program::Action;
use crate::{
    bulb::Bulb,
    daemon::{Msg, DAEMONNAME},
    pilot::{Method, Pilot},
};
pub const WIZARD_PORT: u16 = 38899;
pub struct Wizard {
    socket: Arc<Mutex<Socket>>,
    pub daemon: Arc<Mutex<Option<LocalSocketStream>>>,
    pub bulbs: Arc<Mutex<Vec<Bulb>>>,
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

        Wizard {
            socket: Arc::new(Mutex::new(socket)),
            daemon: Arc::new(Mutex::new(LocalSocketStream::connect(DAEMONNAME).ok())),
            bulbs: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn daemon_connect(&self) {
        let daemon = self.daemon.clone();

        *daemon.lock().unwrap() = LocalSocketStream::connect(DAEMONNAME).ok();
    }

    pub fn daemon_shutdown(&self) {
        let daemon = self.daemon.clone();

        let daemon = daemon.lock().unwrap().take();

        match daemon {
            Some(mut daemon) => {
                let msg = Msg::Stop;
                let data = serde_json::to_string(&msg).unwrap();
                let _ = daemon.write_all(data.as_bytes());
            }
            None => {}
        }
    }

    pub fn daemon_run_program(&self, program: Vec<Action>, bulb_ip: String) {
        let daemon = self.daemon.clone();

        let mut daemon = daemon.lock().unwrap();

        if let Some(daemon) = daemon.as_mut() {
            let msg = Msg::Run(program, bulb_ip);
            let data = serde_json::to_string(&msg).unwrap();
            let _ = daemon.write_all(data.as_bytes());
        }
    }

    pub fn set_pilot(&self, bulb: Bulb, pilot: Pilot) {
        let data = pilot.build();
        let addr: SocketAddr = format!("{}:{}", bulb.ip, WIZARD_PORT).parse().unwrap();
        let _ = self
            .socket
            .lock()
            .unwrap()
            .send_to(data.to_string().as_bytes(), &addr.into());
    }

    pub fn cleanup(&self) {
        let _ = self
            .socket
            .lock()
            .unwrap()
            .shutdown(std::net::Shutdown::Both);
    }

    pub fn discover(&mut self) {
        let localip = local_ip().unwrap();
        let network = Ipv4Net::new(localip.to_string().parse().unwrap(), 24).unwrap();
        let broadcast_addr = network.broadcast().to_string();

        let nbulbs = self.bulbs.clone();
        let nsocket = self.socket.clone();
        thread::spawn(move || {
            let pilot = Pilot::new(Method::GetDevInfo);
            let addr: SocketAddr = format!("{}:{}", broadcast_addr, WIZARD_PORT)
                .parse()
                .unwrap();
            nsocket
                .lock()
                .unwrap()
                .send_to(pilot.build().as_bytes(), &addr.into())
                .unwrap();

            let mut bulbs: Vec<Bulb> = Vec::new();

            let mut buf = [MaybeUninit::new(0u8); 1024];
            loop {
                match nsocket.lock().unwrap().recv_from(&mut buf) {
                    Ok((_amt, src)) => {
                        let src_ip = src.as_socket_ipv4().unwrap();
                        if src_ip.ip() == &localip {
                            continue;
                        }
                        let pbuf = buf.map(|c| unsafe { c.assume_init() });
                        let data = String::from_utf8(pbuf.to_vec()).unwrap();
                        let bulb = Bulb::parse(src_ip.ip().to_string(), &data);
                        if let Some(bulb) = bulb {
                            bulbs.push(bulb);
                        }
                    }
                    Err(_e) => {
                        break;
                    }
                }
            }

            let mut map = std::collections::HashMap::new();
            for bulb in nbulbs.lock().unwrap().iter() {
                map.insert(bulb.mac.clone(), bulb.name.clone());
            }

            for bulb in bulbs.iter_mut() {
                if let Some(name) = map.get(&bulb.mac) {
                    bulb.name = name.clone();
                }
            }

            *nbulbs.lock().unwrap() = bulbs;
        });
    }
}
