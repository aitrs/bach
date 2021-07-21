use ansi_term::Colour::Green;
use ansi_term::Colour::Red;
use ansi_term::Colour::Yellow;
use bach_bus::packet::*;
use bach_module::*;
use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, AtomicU8, Ordering},
    Arc, Mutex,
};
extern crate bach_module_tests;
use bach_module_tests::*;

#[derive(BachModuleStdTests)]
pub struct StdLogger {
    ctrl: Arc<AtomicU8>,
    out_alive: Arc<AtomicBool>,
    message_stack: Arc<Mutex<RefCell<Vec<Packet>>>>,
}

impl Module for StdLogger {
    fn name(&self) -> String {
        "StdLogger".to_string()
    }

    fn init(&self) -> ModResult<()> {
        Ok(())
    }

    fn fire(&self) -> ModuleFireMethod {
        Box::new(|_, run_control, _, _| -> ModResult<()> {
            run_control.store(bach_module::RUN_IDLE, Ordering::SeqCst);
            Ok(())
        })
    }

    fn config_path(&self) -> Option<PathBuf> {
        None
    }

    fn run_status(&self) -> &Arc<AtomicU8> {
        &self.ctrl
    }

    fn emit_alive_status(&self) -> &Arc<AtomicBool> {
        &self.out_alive
    }

    fn message_stack(&self) -> &Arc<Mutex<RefCell<Vec<Packet>>>> {
        &self.message_stack
    }

    fn inlet(&self, p: Packet) {
        let nowstr = chrono::Utc::now().to_rfc2822();
        match p {
            Packet::NotifyGood(_) => {
                println!(
                    "[{}] {}",
                    Green.paint(nowstr),
                    Green.paint(Notification::from(p).to_string())
                );
            }
            Packet::NotifyWarn(_) => {
                println!(
                    "[{}] {}",
                    Yellow.paint(nowstr),
                    Yellow.paint(Notification::from(p).to_string())
                );
            }
            Packet::NotifyErr(_) => {
                println!(
                    "[{}] {}",
                    Red.paint(nowstr),
                    Red.paint(Notification::from(p).to_string())
                );
            }
            Packet::LoggerCom(e) => {
                if let LoggerCommand::Write(s) = LoggerCommand::from(e) {
                    println!("[{}] {}", nowstr, s);
                }
            }
            _ => (),
        }
    }

    fn destroy(&self) -> ModResult<()> {
        Ok(())
    }
}

impl StdLogger {
    pub fn new(_config_filename: Option<String>) -> Self {
        StdLogger {
            ctrl: Arc::new(AtomicU8::new(0)),
            out_alive: Arc::new(AtomicBool::new(false)),
            message_stack: Arc::new(Mutex::new(RefCell::new(Vec::new()))),
        }
    }
}

#[cfg(feature = "modular")]
mk_create_module!(StdLogger, StdLogger::new);
