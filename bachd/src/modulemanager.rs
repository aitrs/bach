use crate::modulemanagerconfig::{ModuleManagerConfig, Whence};
#[cfg(feature = "static")]
use crate::staticmodmatcher;
use bach_bus::bus::{Bus, BusConnection};
use bach_bus::packet::{BackupCommand, Packet, PacketCore};
use bach_module::*;
use lazy_static::lazy_static;
#[cfg(feature = "modular")]
use libloading::{Library, Symbol};
use std::cell::RefCell;
#[cfg(feature = "modular")]
use std::ffi::OsStr;
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};
use chrono::prelude::*;

pub struct ModuleManagerContainer {
    pub module: Box<dyn Module>,
    #[cfg(feature = "modular")]
    pub lib: Library,
    pub whence: Option<Whence>,
}

lazy_static! {
    pub static ref CONNECTIONS: Mutex<RefCell<Vec<BusConnection>>> =
        Mutex::new(RefCell::new(Vec::new()));
}

#[cfg(feature = "modular")]
macro_rules! unwind_moderror {
    ($fn: expr) => {{
        match $fn {
            Ok(a) => a,
            Err(e) => return Err(ModError::new(&e.to_string())),
        }
    }};
}

pub struct LastTimeSeenAlive(RefCell<Instant>);

impl LastTimeSeenAlive {
    pub fn update(&self) {
        self.0.replace(Instant::now());
    }
}

struct ModSpwned {
    pub handle: thread::JoinHandle<ModResult<()>>,
    pub modules_index: usize,
    pub name: String,
    pub last_time_seen_alive: LastTimeSeenAlive,
    pub last_cycle: RefCell<Instant>,
    pub whence: Option<Whence>,
}

pub enum ModSpawnState {
    Spawned,
    NotFound,
    Error(String),
}

pub struct ModuleManager {
    spwned: RefCell<Vec<ModSpwned>>,
    respawn_duration: RefCell<Duration>,
    output: RefCell<Option<Packet>>,
    modules: Vec<ModuleManagerContainer>,
}

impl ModuleManager {
    pub fn new(respawn_duration: Duration) -> Self {
        ModuleManager {
            spwned: RefCell::new(Vec::new()),
            respawn_duration: RefCell::new(respawn_duration),
            output: RefCell::new(None),
            modules: Vec::new(),
        }
    }

    pub fn from_config(conf: ModuleManagerConfig) -> ModResult<Self> {
        let mut ret = ModuleManager::new(Duration::from_secs(conf.respawn_duration));
        for m in conf.modules {
            #[cfg(feature = "modular")]
            ret.load(m.file, m.whence, &m.config)?;

            #[cfg(feature = "static")]
            ret.load(m.name, m.whence, &m.config)?;
        }
        Ok(ret)
    }

    #[cfg(feature = "modular")]
    pub fn load<P: AsRef<OsStr> + std::fmt::Debug + Clone>(
        &mut self,
        filename: P,
        cyclewhence: Option<Whence>,
        config_filename: &Option<String>,
    ) -> ModResult<usize> {
        let size: usize;
        unsafe {
            type ModGen = unsafe fn(&Option<String>) -> Box<dyn Module>;
            let lib = unwind_moderror!(Library::new(filename.as_ref()));
            let cons: Symbol<ModGen> = unwind_moderror!(lib.get(b"bach_create_module"));
            let module: Box<dyn Module> = cons(config_filename);
            self.modules.push(ModuleManagerContainer {
                module,
                lib,
                whence: cyclewhence,
            });
            size = self.modules.len();
        }
        Ok(size)
    }

    #[cfg(feature = "static")]
    pub fn load(
        &mut self,
        name: String,
        cyclewhence: Option<Whence>,
        config_filename: &Option<String>,
    ) -> ModResult<usize> {
        let size: usize = 0;
        self.modules.push(ModuleManagerContainer {
            module: staticmodmatcher::fetch(&name, config_filename)?,
            whence: cyclewhence,
        });
        Ok(size)
    }

    pub fn spawn_all(&self) -> ModResult<()> {
        for (dex, m) in self.modules.iter().enumerate() {
            m.module.init()?;
            let spwn = ModSpwned {
                handle: m.module.spawn(),
                modules_index: dex,
                name: m.module.name().to_string(),
                last_time_seen_alive: LastTimeSeenAlive(RefCell::new(Instant::now())),
                last_cycle: RefCell::new(Instant::now()),
                whence: m.whence.clone(),
            };
            self.spwned.borrow_mut().push(spwn);
        }
        Ok(())
    }

    pub fn get_module_list(&self) -> ModResult<Vec<String>> {
        let mut ret: Vec<String> = Vec::new();
        for m in &self.modules {
            ret.push(m.module.name().to_string());
        }
        Ok(ret)
    }

    pub fn get_spawned_list(&self) -> ModResult<Vec<String>> {
        let mut ret: Vec<String> = Vec::new();
        for m in self.spwned.borrow().iter() {
            ret.push(m.name.to_string());
        }
        Ok(ret)
    }

    pub fn get_status(&self, mod_name: &str) -> String {
        for m in self.spwned.borrow().iter() {
            if m.name.eq(mod_name) {
                return "Running".to_string();
            }
        }

        for m in &self.modules {
            if m.module.name().eq(mod_name) {
                return "Loaded".to_string();
            }
        }

        "Not found".to_string()
    }

    pub fn join_all(&self) -> Vec<(String, ModResult<()>)> {
        let mut res: Vec<(String, ModResult<()>)> = Vec::new();
        let mut spwned = self.spwned.replace(Vec::new());

        while !spwned.is_empty() {
            let item = spwned.pop();
            match item {
                Some(i) => match i.handle.join() {
                    Ok(r) => {
                        let n = i.name.to_string();
                        res.push((i.name, r));
                        for mo in &self.modules {
                            if mo.module.name().eq(&n.to_string()) {
                                match mo.module.destroy() {
                                    Ok(()) => (),
                                    Err(e) => {
                                        res.push((n.to_string(), Err(e)));
                                    }
                                }
                            }
                        }
                    }
                    Err(_) => {
                        res.push((i.name, Err(ModError::new("Panicked !!!"))));
                    }
                },
                None => {
                    break;
                }
            }
        }

        res
    }

    pub fn spawn(&self, mod_name: &str) -> ModSpawnState {
        for (dex, m) in self.modules.iter().enumerate() {
            if m.module.name().eq(mod_name) {
                match m.module.init() {
                    Ok(()) => (),
                    Err(e) => {
                        return ModSpawnState::Error(e.to_string());
                    }
                }
                let spwn = ModSpwned {
                    handle: m.module.spawn(),
                    modules_index: dex,
                    name: m.module.name(),
                    last_time_seen_alive: LastTimeSeenAlive(RefCell::new(Instant::now())),
                    last_cycle: RefCell::new(Instant::now()),
                    whence: m.whence.clone(),
                };
                self.spwned.borrow_mut().push(spwn);
                return ModSpawnState::Spawned;
            }
        }
        ModSpawnState::NotFound
    }

    pub fn respawn(&self, mod_name: &str) -> Option<ModSpawnState> {
        let mut dex = 0;
        if let Ok(mut spawned) = self.spwned.try_borrow_mut() {
            for m in spawned.iter() {
                if m.name.eq(&mod_name) {
                    break;
                }
                dex += 1;
            }
            let torespawn = spawned.remove(dex);
            match torespawn.handle.join() {
                Ok(res) => match res {
                    Ok(()) => {
                        self.output.replace(Some(Packet::new_nw(
                            &format!("Module {} stopped", mod_name),
                            "Module Manager",
                            "Respawn",
                        )));
                    }
                    Err(e) => {
                        self.output.replace(Some(Packet::new_ne(
                            &format!("Module {} exited with error {}", mod_name, e.to_string()),
                            "Module Manager",
                            "Respawn",
                        )));
                    }
                },
                Err(_) => {
                    self.output.replace(Some(Packet::new_ne(
                        &format!("Module {} panicked", mod_name),
                        "Module Manager",
                        "Respawn",
                    )));
                }
            }
            Some(self.spawn(mod_name))
        } else {
            None
        }
    }

    pub fn fire_cyclic(&self) -> ModResult<()> {
        let now: chrono::DateTime<chrono::Local> = chrono::Local::now();
        let stamp = now.timestamp() as i64;
        let offset = now.offset().fix().local_minus_utc() as i64;
        let timestamp = (stamp + offset) as u64;
        for m in self.spwned.borrow().iter() {
            match &m.whence {
                Some(w) => {
                    if timestamp == w.get_whence()? {
                        let namec = m.name.to_string();
                        self.output.replace(Some(Packet::BackupCom(PacketCore::from(
                            BackupCommand::Fire(Some(namec)),
                        ))));
                        m.last_cycle.replace(Instant::now());
                    }
                }
                None => (),
            }
        }
        Ok(())
    }
}

pub fn connect(
    shared_self: &'static Mutex<ModuleManager>,
    bus: &'static Mutex<Bus>,
) -> ModResult<()> {
    bus.lock()?.connect(BusConnection::new(
        move |packet| match shared_self.try_lock() {
            Ok(sup) => {
                if let Packet::Alive(n) = packet {
                    let name = bach_bus::packet::core_2_string(&n[5..bach_bus::packet::CORE_SIZE]);
                    for m in sup.spwned.borrow().iter() {
                        if m.name.eq(&name) {
                            m.last_time_seen_alive.update();
                        } else if m
                            .last_time_seen_alive
                            .0
                            .borrow()
                            .elapsed()
                            .gt(&sup.respawn_duration.borrow())
                        {
                            sup.respawn(&name);
                        }
                    }
                }
            }
            Err(e) => {
                let bus = bus.lock().unwrap();
                bus.send(Packet::new_ne(
                    &format!("Unable to lock module manager : {}", e.to_string()),
                    "Module Manager",
                    "Connect",
                ));
            }
        },
        move || -> Option<Packet> {
            match shared_self.try_lock() {
                Ok(sup) => sup.output.replace(None),
                Err(e) => {
                    let bus = bus.lock().unwrap();
                    bus.send(Packet::new_ne(
                        &format!("Unable to lock module manager : {}", e.to_string()),
                        "Module Manager",
                        "Connect",
                    ));
                    None
                }
            }
        },
    ));

    let lock = shared_self.lock()?;
    let len = lock.modules.len();
    drop(lock);
    for i in 0..len {
        bus.lock()?.connect(BusConnection::new(
            move |packet| {
                shared_self.lock().unwrap().modules[i].module.input(packet);
            },
            move || -> Option<Packet> { shared_self.lock().unwrap().modules[i].module.output() },
        ));
    }

    Ok(())
}
