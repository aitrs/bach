use bach_bus::packet::*;
use bach_module::*;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::fs::File;
use std::io::{prelude::*, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{
    atomic::{AtomicBool, AtomicU8, Ordering},
    Arc, Mutex,
};
use std::thread;
extern crate bach_module_tests;
use bach_module_tests::*;

#[derive(Clone, Debug, Serialize, Deserialize, BachModuleStdTests)]
#[serde(rename_all = "kebab-case")]
pub struct NoreplySelect {
    pub mail_template_file: String,
    pub hostname: String,
    pub log_level: String,
    #[serde(skip)]
    pub ctrl: Arc<AtomicU8>,
    #[serde(skip)]
    pub out_alive: Arc<AtomicBool>,
    #[serde(skip)]
    pub r_log_level: RefCell<String>,
    #[serde(skip)]
    pub message_stack: Arc<Mutex<RefCell<Vec<Packet>>>>,
}

impl NoreplySelect {
    pub fn new(config_filename: Option<String>) -> Self {
        let gendef = || -> NoreplySelect {
            NoreplySelect {
                mail_template_file: "/etc/bach/mail.html".to_string(),
                hostname: "localhost".to_string(),
                log_level: "error".to_string(),
                ctrl: Arc::new(AtomicU8::new(0)),
                out_alive: Arc::new(AtomicBool::new(false)),
                r_log_level: RefCell::new(String::new()),
                message_stack: Arc::new(Mutex::new(RefCell::new(Vec::new()))),
            }
        };
        match config_filename {
            Some(s) => {
                let f = match File::open(s.to_string()) {
                    Ok(f) => f,
                    Err(_) => {
                        println!(
                            "NoreplySelect : Unable to open file {}, fallback to default",
                            s
                        );
                        return gendef();
                    }
                };

                match quick_xml::de::from_reader(BufReader::new(f)) {
                    Ok(c) => c,
                    Err(e) => {
                        println!("NoreplySelect: {:?}, fallback to default", e);
                        gendef()
                    }
                }
            }
            None => gendef(),
        }
    }
}

pub fn mailbose(
    message: String,
    label: String,
    stage: String,
    prefix: String,
    hostname: String,
) -> thread::JoinHandle<ModResult<()>> {
    thread::spawn(move || -> ModResult<()> {
        let tt = chrono::Local::now();
        let dt = tt.format("%Y-%m-%d %H:%M:%S").to_string();
        let mut mailfile = File::open("/etc/bach/mail.html")?;
        let mut mail_contents = String::new();
        mailfile.read_to_string(&mut mail_contents)?;
        mail_contents = mail_contents
            .replace("MESSAGE", &message)
            .replace("DATE", &dt)
            .replace("HOST", &hostname);
        let mut norep = Command::new("noreply_select")
            .arg("-t")
            .arg("-s")
            .arg(&format!(
                "Bach {} {} : {}/{}",
                hostname, prefix, label, stage
            ))
            .arg("-m")
            .arg(&mail_contents)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let res = norep.wait()?;
        let code = res.code().unwrap_or(-1);
        if res.success() {
            Ok(())
        } else {
            Err(ModError::new(&format!(
                "noreply_select exited with error status {}",
                code
            )))
        }
    })
}

impl Module for NoreplySelect {
    fn name(&self) -> String {
        "noreply_select".to_string()
    }

    fn fire(&self) -> ModuleFireMethod {
        Box::new(|_, run_control, _, _| -> ModResult<()> {
            run_control.store(bach_module::RUN_IDLE, Ordering::SeqCst);
            Ok(())
        })
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

    fn init(&self) -> ModResult<()> {
        self.r_log_level.borrow_mut().clear();
        self.r_log_level.borrow_mut().push_str(&self.log_level);
        Ok(())
    }

    fn inlet(&self, p: Packet) {
        let printonerr = |p: Packet, prefix: &str| {
            let not = Notification::from(p);
            if not.good {
                mailbose(
                    not.message,
                    not.provider,
                    not.stage,
                    prefix.to_string(),
                    self.hostname.to_string(),
                );
            }
        };
        let cond_log_level = move |os: Option<String>, level: String| match os {
            Some(s) => {
                if s.eq(&self.name()) {
                    self.r_log_level.borrow_mut().clear();
                    self.r_log_level.borrow_mut().push_str(&level);
                }
            }
            None => {
                self.r_log_level.borrow_mut().clear();
                self.r_log_level.borrow_mut().push_str(&level);
            }
        };
        match p {
            Packet::NotifyGood(_) => {
                if self.log_level.eq("all") {
                    printonerr(p, "Ok");
                }
            }
            Packet::NotifyWarn(_) => {
                if self.log_level.eq("all") || self.log_level.eq("warn") {
                    printonerr(p, "WARN");
                }
            }
            Packet::NotifyErr(_) => {
                if self.log_level.eq("all")
                    || self.log_level.eq("warn")
                    || self.log_level.eq("error")
                {
                    printonerr(p, "ERROR");
                }
            }
            Packet::NotifyCom(core) => {
                let command = NotifyCommand::from(core);
                match command {
                    NotifyCommand::Debug(os) => cond_log_level(os, "all".to_string()),
                    NotifyCommand::Warning(os) => cond_log_level(os, "warn".to_string()),
                    NotifyCommand::Error(os) => cond_log_level(os, "error".to_string()),
                    NotifyCommand::ShutUp(os) => cond_log_level(os, "silent".to_string()),
                    NotifyCommand::Undef => (),
                }
            }
            _ => (),
        }
    }

    fn destroy(&self) -> ModResult<()> {
        Ok(())
    }
}

#[cfg(feature = "modular")]
mk_create_module!(NoreplySelect, NoreplySelect::new);
