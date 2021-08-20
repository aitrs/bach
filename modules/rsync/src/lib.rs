use ansi_term::Colour::Blue;
#[cfg(test)]
use ansi_term::Colour::Purple;
use bach_bus::packet::*;
use bach_module::*;
use std::cell::RefCell;
use std::fs::File;
use std::io::{prelude::*, BufReader};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::{
    atomic::{AtomicBool, AtomicU8, Ordering},
    Arc, Mutex,
};
use std::time::{Duration, Instant};
extern crate bach_module_tests;
use bach_module_tests::*;

pub mod host;
pub mod rsynconfig;

use rsynconfig::*;

#[derive(BachModuleStdTests)]
pub struct Rsync {
    ctrl: Arc<AtomicU8>,
    out_alive: Arc<AtomicBool>,
    config_file: Option<PathBuf>,
    out_stack: Arc<Mutex<RefCell<Vec<Packet>>>>,
}

impl Rsync {
    pub fn new(config_filename: Option<String>) -> Self {
        Rsync {
            ctrl: Arc::new(AtomicU8::new(0)),
            out_alive: Arc::new(AtomicBool::new(false)),
            config_file: config_filename.map(PathBuf::from),
            out_stack: Arc::new(Mutex::new(RefCell::new(Vec::new()))),
        }
    }
}

fn clog(format: String, onlytest: bool) {
    let nowstr = chrono::Local::now().to_rfc2822();
    if onlytest {
        #[cfg(test)]
        println!("[{}] {}", Purple.paint(nowstr), Purple.paint(format));
    } else {
        println!("[{}] {}", Blue.paint(nowstr), Blue.paint(format));
    }
}

fn push_write_command(
    format: String,
    message_stack: &Arc<Mutex<RefCell<Vec<Packet>>>>,
) -> ModResult<()> {
    message_stack
        .lock()?
        .borrow_mut()
        .push(Packet::LoggerCom(PacketCore::from(LoggerCommand::Write(
            format,
        ))));
    Ok(())
}

fn perform_checks(
    item: &RsynConfigItem,
    stack: &Arc<Mutex<RefCell<Vec<Packet>>>>,
    label: &str,
) -> bool {
    let check_target = item.check_target();
    let check_device = item.check_device();
    let check_host = item.check_host_ping();
    let lock_generr = move |format: String| -> bool {
        clog(format.clone(), false);
        if let Ok(cell) = stack.lock() {
            cell.borrow_mut()
                .push(Packet::new_ng(&format, label, "Prelude checks"));
        }
        false
    };

    if check_target.is_err() {
        lock_generr(format!("Target {} not testable", item.get_desc()))
    } else if check_device.is_err() {
        lock_generr(format!("Target device {} not testable", item.get_desc()))
    } else if !check_host {
        lock_generr(format!("Target {} host not reachable", item.get_desc()))
    } else if !check_target.unwrap_or(false) {
        lock_generr(format!("Target {} not reachable", item.get_desc()))
    } else if !check_device.unwrap_or(false) {
        lock_generr(format!("Target device {} not reachable", item.get_desc()))
    } else {
        true
    }
}

fn process_rsync_exit_code(
    item: &RsynConfigItem,
    code: Option<i32>,
    stderr: &str,
    stack: &Arc<Mutex<RefCell<Vec<Packet>>>>,
    label: &str,
) {
    let lock_genwarn = move |format: &str| {
        if let Ok(cell) = stack.lock() {
            cell.borrow_mut().push(Packet::new_nw(
                &format!(
                    "Target {} : {} => {}",
                    item.get_desc(),
                    format,
                    stderr.to_string()
                ),
                label,
                "Exit",
            ));
        }
    };

    let lock_generr = move |format: &str| {
        if let Ok(cell) = stack.lock() {
            cell.borrow_mut().push(Packet::new_ne(
                &format!(
                    "Target {} : {} => {}",
                    item.get_desc(),
                    format,
                    stderr.to_string()
                ),
                label,
                "Exit",
            ));
        }
    };

    let lock_gengood = move |format: &str| {
        if let Ok(cell) = stack.lock() {
            cell.borrow_mut().push(Packet::new_ng(
                &format!("Target {} : {}", item.get_desc(), format),
                label,
                "Exit",
            ));
        }
    };

    if code.is_none() {
        return lock_generr("Rsync is not supposed to return nothing !");
    }

    match code.unwrap() {
        -1 => lock_generr("Rsync killed before end of execution"),
        0 => lock_gengood("Ok"),
        1 => lock_generr("Syntax or usage error"),
        2 => lock_generr("Incompatible protocol"),
        3 => lock_generr("I/O Files selection error"),
        4 => lock_generr("Unsupported action"),
        5 => lock_generr("Client/Server startup error"),
        6 => lock_generr("Could not open log file"),
        10 => lock_generr("I/O socket error"),
        11 => lock_generr("I/O file error"),
        12 => lock_generr("Data flow error"),
        13 => lock_generr("Diagnostic error"),
        14 => lock_generr("IPC error"),
        20 => lock_generr("Killed by user"),
        21 => lock_generr("Waitpid() failed"),
        22 => lock_generr("Buffer allocation error"),
        23 => lock_genwarn("Partial transfer due to modified files during backup"),
        24 => lock_genwarn("Partial transfer due to modified files during backup"),
        25 => lock_genwarn("Max delete limit reached, suppressions stopped"),
        30 => lock_generr("Timeout rx/tx error"),
        31 => lock_generr("Connection timeout error"),
        127 => lock_generr("Rsync executable is corrupted"),
        255 => lock_generr("Ssh disconnected"),
        _ => lock_generr("Unexpected return code"),
    }
}

fn do_mount(
    item: &RsynConfigItem,
    stackc: &Arc<Mutex<RefCell<Vec<Packet>>>>,
    namecc: &str,
) -> ModResult<bool> {
    println!("Doing Mount");
    let mount = item.mount_target();
    if !mount.unwrap_or(false) {
        stackc.lock()?.borrow_mut().push(Packet::new_ne(
            &format!("Unable to mount target {}", item.get_desc()),
            namecc,
            "Mount",
        ));
        println!("Mount failed");
        Ok(false)
    } else {
        println!("Mount success");
        Ok(true)
    }
}

fn do_umount(
    item: &RsynConfigItem,
    stackc: &Arc<Mutex<RefCell<Vec<Packet>>>>,
    namecc: String,
) -> ModResult<()> {
    match item.umount_target() {
        Ok(b) => {
            if !b {
                stackc.lock()?.borrow_mut().push(Packet::new_nw(
                    &format!("Target {} was not unmounted", item.get_desc(),),
                    &namecc,
                    "Unmount",
                ));
                Err(ModError::new(&format!(
                    "Target {} was not numounted",
                    item.get_desc()
                )))
            } else {
                Ok(())
            }
        }
        Err(e) => {
            stackc.lock()?.borrow_mut().push(Packet::new_ne(
                &format!(
                    "Target {} unmount crashed: {}",
                    item.get_desc(),
                    e.to_string()
                ),
                &namecc,
                "Unmount",
            ));
            Err(ModError::new(&format!(
                "Target {} umount crashed",
                item.get_desc()
            )))
        }
    }
}

fn wait_or_kill(
    run_control: &Arc<AtomicU8>,
    child: &mut std::process::Child,
    timeout: Option<u64>,
) -> ModResult<Option<(std::process::ExitStatus, String)>> {
    let start = Instant::now();
    let runc = Arc::downgrade(run_control);
    let stat = loop {
        if let Some(st) = child.try_wait()? {
            let buf = match &mut child.stdout {
                Some(ref mut out) => {
                    let mut res = String::new();
                    out.read_to_string(&mut res)?;
                    res
                }
                None => "".to_string(),
            };
            break Some((st, buf));
        }

        if let Some(timeout) = timeout {
            if start.elapsed().gt(&Duration::from_secs(timeout * 60)) {
                child.kill()?;
                break None;
            }
        }

        if let Some(ctrlc) = runc.upgrade() {
            let c = ctrlc.load(Ordering::SeqCst);
            if c == bach_module::RUN_TERM || c == bach_module::RUN_EARLY_TERM {
                child.kill()?;
                break None;
            }
        } else {
            child.kill()?;
            break None;
        }

        std::thread::sleep(Duration::from_millis(100));
    };
    Ok(stat)
}

impl Module for Rsync {
    fn name(&self) -> String {
        if let Some(config_file) = &self.config_file {
            let conf: RsynConfig =
                match quick_xml::de::from_reader(BufReader::new(match File::open(&config_file) {
                    Ok(f) => f,
                    Err(e) => {
                        return format!(
                            "{} : {}",
                            config_file.to_str().unwrap_or("NOT FOUND"),
                            e.to_string()
                        )
                    }
                })) {
                    Ok(conf) => conf,
                    Err(e) => return e.to_string(),
                };

            conf.label
        } else {
            "undefined".to_string()
        }
    }

    fn fire(&self) -> ModuleFireMethod {
        Box::new(
            |message_stack, run_control, config_path, name| -> ModResult<()> {
                bach_module::wait_for_running_status(run_control);
                if let Some(path) = config_path.lock()?.borrow().as_ref() {
                    clog("Fire Rsync Start".to_string(), true);
                    let config: RsynConfig =
                        quick_xml::de::from_reader(BufReader::new(File::open(path.as_path())?))?;

                    for item in config.synchros {
                        let namecc = name.lock()?.borrow().to_string();
                        clog(format!("Config name: {}", namecc), true);

                        if perform_checks(&item, message_stack, &namecc)
                            && do_mount(&item, message_stack, &namecc)?
                        {
                            clog("Passed checks".to_string(), true);
                            let mut cmd = item.to_cmd();
                            let mut child =
                                cmd.stdout(Stdio::null()).stderr(Stdio::piped()).spawn()?;
                            run_control.store(bach_module::RUN_RUNNING, Ordering::SeqCst);
                            clog(format!("Spawning {:?}", cmd), true);

                            push_write_command(
                                format!(
                                    "Command {:?} successfully launched on target {}",
                                    &cmd,
                                    item.get_desc()
                                ),
                                message_stack,
                            )?;
                            let w = wait_or_kill(run_control, &mut child, item.timeout)?;
                            let stderr = match &w {
                                Some(p) => p.1.to_string(),
                                None => "".to_string(),
                            };

                            process_rsync_exit_code(
                                &item,
                                match &w {
                                    Some(proc1) => proc1.0.code(),
                                    None => Some(-1),
                                },
                                &stderr,
                                message_stack,
                                &namecc,
                            );
                            std::thread::sleep(std::time::Duration::from_secs(1));
                            do_umount(&item, message_stack, namecc)?;
                        }
                        std::thread::sleep(std::time::Duration::from_secs(10));
                    }
                    run_control.store(bach_module::RUN_IDLE, Ordering::SeqCst);
                } else {
                    clog(
                        "Rsync module requires a configuration file".to_string(),
                        true,
                    );
                    run_control.store(bach_module::RUN_IDLE, Ordering::SeqCst);
                }
                Ok(())
            },
        )
    }

    fn run_status(&self) -> &Arc<AtomicU8> {
        &self.ctrl
    }

    fn config_path(&self) -> Option<PathBuf> {
        self.config_file.as_ref().cloned()
    }

    fn emit_alive_status(&self) -> &Arc<AtomicBool> {
        &self.out_alive
    }

    fn message_stack(&self) -> &Arc<Mutex<RefCell<Vec<Packet>>>> {
        &self.out_stack
    }

    fn init(&self) -> ModResult<()> {
        self.outlet(if self.name().contains("error") {
            Packet::new_ne("ERROR", &self.name(), "Init")
        } else {
            Packet::new_ng(
                &format!("{} rsync module initialized", self.name()),
                &self.name(),
                "Init",
            )
        });
        Ok(())
    }

    fn inlet(&self, _: Packet) {}

    fn destroy(&self) -> ModResult<()> {
        Ok(())
    }
}

#[cfg(feature = "modular")]
mk_create_module!(Rsync, Rsync::new);
