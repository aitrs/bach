use bach_bus::packet::*;
use bach_module::*;
use std::cell::RefCell;
use std::fs::OpenOptions;
use std::io::prelude::*;
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, AtomicU8, Ordering},
    Arc, Mutex,
};
extern crate bach_module_tests;
use bach_module_tests::*;

#[derive(BachModuleStdTests)]
pub struct Dummy {
    ctrl: Arc<AtomicU8>,
    out_alive: Arc<AtomicBool>,
    message_stack: Arc<Mutex<RefCell<Vec<Packet>>>>,
}

impl Module for Dummy {
    fn name(&self) -> String {
        "Dummy".to_string()
    }

    fn init(&self) -> ModResult<()> {
        println!("Dummy Init");
        Ok(())
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

    fn config_path(&self) -> Option<PathBuf> {
        None
    }

    fn fire(&self) -> ModuleFireMethod {
        Box::new(|_, run_state, _, _| {
            let mut f = OpenOptions::new()
                .write(true)
                .create(true)
                .append(true)
                .open("./dummyout.txt")?;
            f.write_all(b"Dummy wrote\n")?;
            run_state.store(bach_module::RUN_IDLE, Ordering::SeqCst);

            Ok(())
        })
    }

    fn inlet(&self, _: Packet) {}

    fn destroy(&self) -> ModResult<()> {
        Ok(())
    }
}

impl Dummy {
    pub fn new(_config_filename: Option<String>) -> Self {
        Dummy {
            ctrl: Arc::new(AtomicU8::new(0)),
            out_alive: Arc::new(AtomicBool::new(false)),
            message_stack: Arc::new(Mutex::new(RefCell::new(Vec::new()))),
        }
    }
}

#[cfg(feature = "modular")]
mk_create_module!(Dummy, Dummy::new);
