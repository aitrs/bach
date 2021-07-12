use ansi_term::Colour::Green;
use ansi_term::Colour::Red;
use ansi_term::Colour::Yellow;
use bach_bus::packet::*;
use bach_module::*;
use std::sync::{
    atomic::{AtomicBool, AtomicU8, Ordering},
    Arc,
};
use std::thread;
use std::time::{Duration, Instant};

pub struct StdLogger {
    ctrl: Arc<AtomicU8>,
    out_alive: Arc<AtomicBool>,
}

impl Module for StdLogger {
    fn name(&self) -> String {
        "StdLogger".to_string()
    }

    fn init(&self) -> ModResult<()> {
        Ok(())
    }

    fn accept(&self, p: Packet) -> bool {
        matches!(p,
            Packet::NotifyGood(_) |
            Packet::NotifyWarn(_) |
            Packet::NotifyErr(_) |
            Packet::LoggerCom(_) |
            Packet::Terminate 
        )
    }

    fn inlet(&self, p: Packet) {
        match p {
            Packet::NotifyGood(_) => {
                println!("{}", Green.paint(Notification::from(p).to_string()));
            }
            Packet::NotifyWarn(_) => {
                println!("{}", Yellow.paint(Notification::from(p).to_string()));
            }
            Packet::NotifyErr(_) => {
                println!("{}", Red.paint(Notification::from(p).to_string()));
            }
            Packet::LoggerCom(e) => {
                if let LoggerCommand::Write(s) = LoggerCommand::from(e) {
                    println!("{}", s);
                }
            }
            Packet::Terminate => {
                self.ctrl.store(1, Ordering::SeqCst);
            }
            _ => (),
        }
    }

    fn destroy(&self) -> ModResult<()> {
        Ok(())
    }

    fn outlet(&self) -> Option<Packet> {
        if self.out_alive.load(Ordering::SeqCst) {
            self.out_alive.store(false, Ordering::SeqCst);
            Some(Packet::new_alive("StdLogger"))
        } else {
            None
        }
    }

    fn spawn(&self) -> thread::JoinHandle<ModResult<()>> {
        let ctrlc = self.ctrl.clone();
        let alivc = self.out_alive.clone();

        thread::spawn(move || -> ModResult<()> {
            let mut start = Instant::now();
            loop {
                let c = ctrlc.load(Ordering::SeqCst);
                if c == 1 {
                    break;
                } else {
                    thread::sleep(Duration::from_millis(50));
                    if start.elapsed().gt(&Duration::from_secs(1)) {
                        alivc.store(true, Ordering::SeqCst);
                        start = Instant::now();
                    }
                }
            }
            Ok(())
        })
    }
}

impl StdLogger {
    pub fn new(_config_filename: Option<String>) -> Self {
        StdLogger {
            ctrl: Arc::new(AtomicU8::new(0)),
            out_alive: Arc::new(AtomicBool::new(false)),
        }
    }
}

#[cfg(feature = "modular")]
mk_create_module!(StdLogger, StdLogger::new);
