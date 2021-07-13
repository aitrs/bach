use bach_bus::packet::*;
use bach_module::*;
use std::fs::OpenOptions;
use std::io::prelude::*;
use std::sync::{
    atomic::{AtomicBool, AtomicU8, Ordering},
    Arc,
};
use std::thread;
use std::time::{Duration, Instant};
extern crate bach_module_tests;
use bach_module_tests::*;

#[derive(BachModuleStdTests)]
pub struct Dummy {
    ctrl: Arc<AtomicU8>,
    out_alive: Arc<AtomicBool>,
}

impl Module for Dummy {
    fn name(&self) -> String {
        "Dummy".to_string()
    }

    fn init(&self) -> ModResult<()> {
        println!("Dummy Init");
        Ok(())
    }

    fn accept(&self, p: Packet) -> bool {
        match p {
            Packet::BackupCom(core) => match BackupCommand::from(core) {
                BackupCommand::Fire(Some(s)) => {
                    println!("Dummy received fire message : {}", s);
                    s.eq(&self.name())
                }
                _ => false,
            },
            Packet::Terminate => true,
            _ => false,
        }
    }

    fn inlet(&self, p: Packet) {
        println!("Dummy accepted message");
        match p {
            Packet::BackupCom(_) => {
                self.ctrl.store(1, Ordering::SeqCst);
            }
            Packet::Terminate => {
                self.ctrl.store(2, Ordering::SeqCst);
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
            println!("Dummy says he's alive");
            Some(Packet::new_alive("Dummy"))
        } else {
            None
        }
    }

    fn spawn(&self) -> thread::JoinHandle<ModResult<()>> {
        let ctrlc = self.ctrl.clone();
        let alivc = self.out_alive.clone();
        thread::spawn(move || -> ModResult<()> {
            let mut start = Instant::now();
            let mut run = true;
            println!("Dummy Spawned");
            while run {
                let c = ctrlc.load(Ordering::SeqCst);
                if c == 2 {
                    run = false;
                } else if c == 1 {
                    let mut f = OpenOptions::new()
                        .write(true)
                        .create(true)
                        .append(true)
                        .open("./dummyout.txt")?;
                    f.write_all(b"Dummy wrote\n")?;
                    ctrlc.store(0, Ordering::SeqCst);
                }
                thread::sleep(Duration::from_millis(500));
                if start.elapsed().gt(&Duration::from_secs(1)) {
                    alivc.store(true, Ordering::SeqCst);
                    start = Instant::now();
                }
            }
            println!("Dummy Stopped");
            Ok(())
        })
    }
}

impl Dummy {
    pub fn new(_config_filename: Option<String>) -> Self {
        Dummy {
            ctrl: Arc::new(AtomicU8::new(0)),
            out_alive: Arc::new(AtomicBool::new(false)),
        }
    }
}

#[cfg(feature = "modular")]
mk_create_module!(Dummy, Dummy::new);
