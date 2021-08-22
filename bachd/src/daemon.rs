use crate::modulemanager;
use crate::modulemanagerconfig::ModuleManagerConfig;
use crate::tcpmessages::*;
use bach_bus::bus::Bus;
use bach_bus::packet::{BackupCommand, Packet, PacketCore, CORE_SIZE};
use bach_module::ModError;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::prelude::*;
use std::io::BufReader;
use std::net::TcpListener;
use std::path::Path;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

lazy_static! {
    static ref MANAGER: Mutex<modulemanager::ModuleManager> = Mutex::new(
        modulemanager::ModuleManager::from_config(DaemonConfig::load().unwrap().module_manager)
            .unwrap()
    );
    static ref BUS: Mutex<Bus> = Mutex::new(Bus::new());
}

#[derive(Debug, Clone)]
pub struct DaemonError {
    message: String,
    code: u64,
}

impl DaemonError {
    pub fn new(message: String, code: u64) -> Self {
        DaemonError { message, code }
    }
}

impl std::error::Error for DaemonError {}

impl std::fmt::Display for DaemonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "Code: {} :: Message: {}",
            self.code, self.message
        ))
    }
}

impl From<std::io::Error> for DaemonError {
    fn from(item: std::io::Error) -> Self {
        DaemonError {
            code: 1,
            message: item.to_string(),
        }
    }
}

impl From<quick_xml::DeError> for DaemonError {
    fn from(item: quick_xml::DeError) -> Self {
        DaemonError {
            code: 2,
            message: item.to_string(),
        }
    }
}

impl<T> From<std::sync::PoisonError<T>> for DaemonError {
    fn from(item: std::sync::PoisonError<T>) -> Self {
        DaemonError {
            code: 4,
            message: item.to_string(),
        }
    }
}

impl From<std::time::SystemTimeError> for DaemonError {
    fn from(item: std::time::SystemTimeError) -> Self {
        DaemonError {
            code: 6,
            message: item.to_string(),
        }
    }
}

impl From<ModError> for DaemonError {
    fn from(item: ModError) -> Self {
        DaemonError {
            code: 3,
            message: item.to_string(),
        }
    }
}

pub type DaemonResult<T> = Result<T, DaemonError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonConfigAcceptIp(String);
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonConfigTcpPort(u64);
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonConfigLogLevel(String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonConfig {
    pub port: DaemonConfigTcpPort,
    pub ip: DaemonConfigAcceptIp,
    #[serde(rename = "log-level")]
    pub log_level: DaemonConfigLogLevel,
    #[serde(rename = "module-manager")]
    pub module_manager: ModuleManagerConfig,
}

impl DaemonConfig {
    pub fn load() -> DaemonResult<Self> {
        for v in std::env::vars() {
            if v.0.eq("BACH_DEFAULT_CONFIG") && !v.1.is_empty() {
                println!("Loading config file {}", &v.1);
                let file = fs::File::open(Path::new(&v.1))?;
                let ret: DaemonConfig = quick_xml::de::from_reader(BufReader::new(file))?;

                return Ok(ret);
            }
        }

        let file = fs::File::open(Path::new("/etc/bach/bachd.conf.xml"))?;
        let ret: DaemonConfig = quick_xml::de::from_reader(BufReader::new(file))?;
        Ok(ret)
    }

    pub fn save(&self, fname: &Path) -> DaemonResult<()> {
        let file = fs::File::create(fname)?;
        quick_xml::se::to_writer(file, &self)?;

        Ok(())
    }
}

pub fn mk_tcp_connection(config: &DaemonConfig) -> DaemonResult<TcpListener> {
    let port = config.port.0;
    let cstr = format!("{}:{}", config.ip.0, port);
    let stream = TcpListener::bind(&cstr)?;
    #[cfg(feature = "debug")]
    println!("Daemon made TCP connection with IP {} on port {}", config.ip.0, port);

    Ok(stream)
}

fn process_tcp_command_list(list: TcpCommandList) -> DaemonResult<()> {
    #[cfg(feature = "debug")]
    println!("TCP connection got LIST command");
    match list {
        TcpCommandList::Loaded => {
            let list: Vec<String> = MANAGER.lock()?.get_module_list()?;
            for i in list {
                println!("\t{}", i);
            }
        }
        TcpCommandList::Running => {
            let list: Vec<String> = MANAGER.lock()?.get_spawned_list()?;

            for i in list {
                println!("\t{}", i);
            }
        }
    }
    Ok(())
}

fn join_and_print() -> DaemonResult<()> {
    let vecres = MANAGER.lock()?.join_all();
    for r in vecres {
        match r.1 {
            Ok(()) => (),
            Err(e) => {
                println!("Error: Module {} failed => {}", r.0, e.to_string())
            }
        }
    }

    Ok(())
}

pub fn spawn() -> DaemonResult<()> {
    let config: DaemonConfig = DaemonConfig::load()?;
    let tcp = mk_tcp_connection(&config)?;
    let mut run = true;
    
    tcp.set_nonblocking(true)?;
    modulemanager::connect(&MANAGER, &BUS)?;
    MANAGER.lock()?.spawn_all()?;
    loop {
        MANAGER.lock()?.fire_cyclic()?;
        for tcpstream in tcp.incoming() {
            let mut current_core = [0u8; CORE_SIZE];
            match tcpstream {
                Ok(mut stream) => {
                    stream.read_exact(&mut current_core)?;
                    match TcpCommand::from(current_core) {
                        TcpCommand::List(list) => {
                            process_tcp_command_list(list)?;
                        }
                        TcpCommand::Status(name) => {
                            println!("{}", MANAGER.lock()?.get_status(&name));
                        }
                        TcpCommand::Stop(name) => {
                            BUS.lock()?.send(Packet::new_stop(&name));
                        }
                        TcpCommand::Terminate => {
                            BUS.lock()?.send(Packet::new_term());
                            run = false;
                            break;
                        }
                        TcpCommand::Fire(name) => {
                            BUS.lock()?.send(Packet::BackupCom(PacketCore::from(
                                BackupCommand::Fire(Some(name)),
                            )));
                        }
                        _ => (),
                    }
                }
                Err(_) => break,
            }
        }
        BUS.lock()?.perform();
        if !run {
            break;
        }
        thread::sleep(Duration::from_millis(250));
    }

    join_and_print()?;

    Ok(())
}
