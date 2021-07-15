use bach_bus::packet::{core_2_string, BackupCommand, Packet};
use std::any::Any;
use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, AtomicU8, Ordering},
    Arc, Mutex,
};
use std::thread::{self, JoinHandle};
use std::time::Duration;

pub static RUN_IDLE: u8 = 0;
pub static RUN_FIRE: u8 = 1;
pub static RUN_TERM: u8 = 2;
pub static RUN_RUNNING: u8 = 3;
pub static RUN_EARLY_TERM: u8 = 4;
pub static RUN_MODULE_SPEC1: u8 = 5;
pub static RUN_MODULE_SPEC2: u8 = 6;
pub static ALIVE_PACKET_EMISSION_TIMEOUT: u64 = 2;

pub type ModuleFireMethod = Box<
    dyn Fn(
            &Arc<Mutex<RefCell<Vec<Packet>>>>,
            &Arc<AtomicU8>,
            &Arc<Mutex<RefCell<Option<PathBuf>>>>,
            &Arc<Mutex<RefCell<String>>>,
        ) -> ModResult<()>
        + Sync
        + Send,
>;

#[derive(Debug, Clone)]
pub struct ModError {
    message: String,
}

impl ModError {
    pub fn new(message: &str) -> Self {
        ModError {
            message: message.to_string(),
        }
    }
}

impl std::error::Error for ModError {}

impl std::fmt::Display for ModError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.message))
    }
}

impl From<std::io::Error> for ModError {
    fn from(item: std::io::Error) -> Self {
        ModError {
            message: item.to_string(),
        }
    }
}

impl From<quick_xml::DeError> for ModError {
    fn from(item: quick_xml::DeError) -> Self {
        ModError {
            message: item.to_string(),
        }
    }
}

impl<T> From<std::sync::PoisonError<T>> for ModError {
    fn from(item: std::sync::PoisonError<T>) -> Self {
        ModError {
            message: item.to_string(),
        }
    }
}

impl From<std::num::ParseIntError> for ModError {
    fn from(item: std::num::ParseIntError) -> Self {
        ModError {
            message: item.to_string(),
        }
    }
}

impl From<std::time::SystemTimeError> for ModError {
    fn from(item: std::time::SystemTimeError) -> Self {
        ModError {
            message: item.to_string(),
        }
    }
}

impl From<Box<dyn std::any::Any + Send>> for ModError {
    fn from(_: Box<dyn std::any::Any + Send>) -> Self {
        ModError {
            message: "Join Error".to_string(),
        }
    }
}

/// Convenience type for results in the ['Module'] trait.
pub type ModResult<T> = Result<T, ModError>;

pub trait Module: Any + Send {
    fn name(&self) -> String;
    fn init(&self) -> ModResult<()>;
    fn fire(&self) -> ModuleFireMethod;
    fn destroy(&self) -> ModResult<()>;
    fn inlet(&self, p: Packet);

    fn outlet(&self, p: Packet) {
        if let Ok(message_stack) = self.message_stack().lock() {
            message_stack.borrow_mut().push(p);
        }
    }
    fn run_status(&self) -> &Arc<AtomicU8>;
    fn emit_alive_status(&self) -> &Arc<AtomicBool>;
    fn message_stack(&self) -> &Arc<Mutex<RefCell<Vec<Packet>>>>;
    fn config_path(&self) -> Option<PathBuf>;

    fn input(&self, p: Packet) {
        match p {
            Packet::BackupCom(core) => {
                if let BackupCommand::Fire(Some(s)) = BackupCommand::from(core) {
                    if s.eq(&self.name()) {
                        self.run_status().store(RUN_FIRE, Ordering::SeqCst);
                    }
                }
            }
            Packet::Stop(core) => {
                let core_name = core_2_string(&core);
                if core_name.eq(&self.name()) {
                    self.run_status().store(RUN_TERM, Ordering::SeqCst);
                }
            }
            Packet::Terminate => {
                self.run_status().store(RUN_TERM, Ordering::SeqCst);
            }
            _ => {
                self.inlet(p);
            }
        }
    }

    fn output(&self) -> Option<Packet> {
        if let Ok(message_stack) = self.message_stack().lock() {
            if self.emit_alive_status().load(Ordering::SeqCst) {
                self.emit_alive_status().store(false, Ordering::SeqCst);
                message_stack
                    .borrow_mut()
                    .push(Packet::new_alive(&self.name()));
            }

            message_stack.borrow_mut().pop()
        } else {
            println!(
                "Big problem : Unable to lock message stack for module {}",
                &self.name()
            );
            None
        }
    }

    fn spawn_alive_emitter(&self) -> JoinHandle<()> {
        let emitalive = self.emit_alive_status().clone();
        let ctrlstat = self.run_status().clone();
        thread::spawn(move || {
            let mut run = true;
            while run {
                let c = ctrlstat.load(Ordering::SeqCst);
                if c == RUN_TERM || c == RUN_EARLY_TERM {
                    run = false;
                }
                thread::sleep(Duration::from_secs(ALIVE_PACKET_EMISSION_TIMEOUT));
                emitalive.store(true, Ordering::SeqCst);
            }
        })
    }

    fn spawn(&self) -> JoinHandle<ModResult<()>> {
        let ctrlstat = self.run_status().clone();
        let ctrlstat2 = ctrlstat.clone();
        let ctrlstat3 = ctrlstat.clone();
        let message_stack = self.message_stack().clone();
        let main_method = self.fire();
        let name_arc = Arc::new(Mutex::new(RefCell::new(self.name())));
        let config_arc = Arc::new(Mutex::new(RefCell::new(self.config_path())));
        self.spawn_alive_emitter();

        thread::spawn(move || -> ModResult<()> {
            let mut run = true;
            let mut main = move || -> ModResult<()> {
                while run {
                    let c = ctrlstat.load(Ordering::SeqCst);
                    if c == RUN_TERM || c == RUN_EARLY_TERM {
                        run = false;
                    } else if c == RUN_FIRE {
                        ctrlstat.store(RUN_RUNNING, Ordering::SeqCst);
                        match main_method(
                            &message_stack,
                            &ctrlstat2,
                            &config_arc,
                            &name_arc.clone(),
                        ) {
                            Ok(()) => {
                                message_stack.lock()?.borrow_mut().push(Packet::new_ng(
                                    "Successful End",
                                    &name_arc.lock()?.borrow(),
                                    "END",
                                ));
                                ctrlstat.store(RUN_IDLE, Ordering::SeqCst);
                            }
                            Err(e) => {
                                message_stack.lock()?.borrow_mut().push(Packet::new_ne(
                                    &e.message,
                                    &name_arc.lock()?.borrow(),
                                    "RUN",
                                ));
                                ctrlstat.store(RUN_EARLY_TERM, Ordering::SeqCst);
                            }
                        }
                    }
                }

                Ok(())
            };

            if let Err(e) = main() {
                ctrlstat3.store(RUN_EARLY_TERM, Ordering::SeqCst);
                return Err(e);
            }

            Ok(())
        })
    }
}

#[cfg(feature = "modular")]
#[macro_export]
macro_rules! mk_create_module {
    ($plugin_type: ty, $constructor:path) => {
        #[no_mangle]
        pub extern "C" fn bach_create_module(
            config_filename: Option<String>,
        ) -> Box<dyn $crate::Module> {
            let constructor: fn(Option<String>) -> $plugin_type = $constructor;
            let object = constructor(config_filename);
            let boxed: Box<dyn $crate::Module> = Box::new(object);
            boxed
        }
    };
}
