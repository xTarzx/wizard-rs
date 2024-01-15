use interprocess::local_socket::LocalSocketListener;
use socket2::{Domain, Protocol, Socket, Type};

use std::{
    io::Read,
    net::SocketAddr,
    sync::{
        atomic::AtomicBool,
        mpsc,
        mpsc::{Receiver, Sender},
        Arc,
    },
    thread,
};

use wizard_rs::daemon::{Msg, DAEMONNAME};
use wizard_rs::program::Action;
use wizard_rs::wizard::WIZARD_PORT;

fn worker(rx: Receiver<Msg>) {
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP)).unwrap();
    let mut bulb_ip: Option<String> = None;

    let mut program: Vec<Action> = Vec::new();
    let mut idx: usize = 0;

    let mut run = true;

    while run {
        if let Ok(msg) = rx.try_recv() {
            match msg {
                Msg::Stop => {
                    program.clear();
                    idx = 0;
                    run = false;
                }

                Msg::Run(prog, ip) => {
                    program = prog;
                    bulb_ip = Some(ip);
                    idx = 0;
                }

                _ => {
                    println!("{:?}", msg);
                }
            }
        }

        if !program.is_empty() {
            if idx >= program.len() {
                idx = 0;
            }

            let action = &program[idx];

            match action {
                Action::Sleep(s) => {
                    thread::sleep(std::time::Duration::from_secs(*s));
                }
                Action::SetPilot(pilot) => {
                    let data = pilot.build();
                    let addr: SocketAddr = format!("{}:{}", bulb_ip.clone().unwrap(), WIZARD_PORT)
                        .parse()
                        .unwrap();

                    let _ = socket.send_to(data.as_bytes(), &addr.into());
                }
            }

            idx += 1;
        }
    }
}

fn main() {
    let run = Arc::new(AtomicBool::new(true));
    let (tx, rx): (Sender<Msg>, Receiver<Msg>) = mpsc::channel();

    let tx_clone = tx.clone();
    let r_clone = run.clone();
    ctrlc::set_handler(move || {
        let _ = tx_clone.send(Msg::Stop);
        r_clone.store(false, std::sync::atomic::Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    let listener = LocalSocketListener::bind(DAEMONNAME).unwrap();
    listener
        .set_nonblocking(true)
        .expect("could not set nonblocking");

    let t = thread::spawn(move || {
        worker(rx);
    });

    while run.load(std::sync::atomic::Ordering::SeqCst) {
        match listener.accept() {
            Ok(mut stream) => {
                let mut buf = [0u8; 1024];
                if let Ok(size) = stream.read(&mut buf) {
                    let msg: Msg = serde_json::from_slice(&buf[..size]).unwrap_or(Msg::Ignore);

                    match msg {
                        Msg::Stop => {
                            let _ = tx.send(msg);
                            run.store(false, std::sync::atomic::Ordering::SeqCst);
                        }
                        Msg::Run(_, _) => {
                            let _ = tx.send(msg);
                        }
                        Msg::Ignore => {
                            // println!("could not parse action");
                            // println!("{:?}", msg);
                        }
                    }
                };
            }
            Err(e) => {
                if e.kind() != std::io::ErrorKind::WouldBlock {
                    println!("Error: {}", e);
                }
            }
        }
    }

    let _ = std::fs::remove_file(DAEMONNAME);

    println!("waiting for worker thread to finish");
    t.join().unwrap();
}
