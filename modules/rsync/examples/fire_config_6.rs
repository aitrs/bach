use bach_module::*;
use bach_bus::packet::*;
use rsync::*;
use std::sync::{
    Arc, Mutex,
    atomic::AtomicU8
};
use std::cell::RefCell;
use std::path::PathBuf;

pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rsync = Rsync::new(Some("./example.config.6.xml".to_string()));
    let method = rsync.fire();
    let message_stack: Arc<Mutex<RefCell<Vec<Packet>>>> 
        = Arc::new(Mutex::new(RefCell::new(Vec::new())));
    let run_control = Arc::new(AtomicU8::new(bach_module::RUN_RUNNING));
    let path = Arc::new(Mutex::new(RefCell::new(Some(PathBuf::from("./example.config.6.xml")))));
    let name = Arc::new(Mutex::new(RefCell::new("test".to_string())));
    let res = method(&message_stack, &run_control, &path, &name);
    Ok(res?)
}
