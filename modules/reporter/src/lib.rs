use bach_bus::packet::*;
use bach_module::*;
use std::cell::RefCell;
use std::fs::{self, File};
use std::io::{prelude::*, BufReader, LineWriter};
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, AtomicU8},
    Arc, Mutex,
};
extern crate bach_module_tests;
use bach_module_tests::*;

pub mod reporterconfig;
use reporterconfig::ReporterConfig;
pub mod mailtemplate;
use mailtemplate::gen_mail;

fn tmp_format(s: &str) -> String {
    format!("/tmp/{}.bach.report", s)
}

fn init_tmp_file(conf: &ReporterConfig) -> ModResult<()> {
    let mut file = File::create(&tmp_format(&conf.name))?;
    file.write_all("Received Notifications : \n".as_bytes())?;
    Ok(())
}

fn translate_severity(sev: String) -> String {
    if sev.eq("debug") {
        "Good".to_string()
    } else if sev.eq("warn") || sev.eq("warning") {
        "Warnings".to_string()
    } else if sev.eq("error") {
        "Errors".to_string()
    } else {
        "Unknown Status".to_string()
    }
}

#[derive(BachModuleStdTests)]
pub struct Reporter {
    ctrl: Arc<AtomicU8>,
    out_alive: Arc<AtomicBool>,
    config_file: Option<PathBuf>,
    out_stack: Arc<Mutex<RefCell<Vec<Packet>>>>,
}

impl Reporter {
    pub fn new(config_filename: &Option<String>) -> Self {
        #[cfg(feature = "debug")]
        println!("Reporter instanciated with {:?}", &config_filename.clone());
        Reporter {
            ctrl: Arc::new(AtomicU8::new(0)),
            out_alive: Arc::new(AtomicBool::new(false)),
            config_file: config_filename.clone().map(PathBuf::from),
            out_stack: Arc::new(Mutex::new(RefCell::new(Vec::new()))),
        }
    }
}

impl Module for Reporter {
    fn name(&self) -> String {
        if let Some(config_file) = &self.config_file {
            let conf: ReporterConfig =
                match quick_xml::de::from_reader(BufReader::new(match File::open(&config_file) {
                    Ok(f) => f,
                    Err(e) => {
                        return format!(
                            "{} : {}",
                            config_file.to_str().unwrap_or("NOT FOUND"),
                            e.to_string()
                        );
                    }
                })) {
                    Ok(conf) => conf,
                    Err(e) => return e.to_string(),
                };

            conf.name
        } else {
            "undefined".to_string()
        }
    }

    fn fire(&self) -> ModuleFireMethod {
        Box::new(
            |message_stack, run_control, config_path, name| -> ModResult<()> {
                let check_level = |conf: &ReporterConfig, severity: &str| -> bool {
                    if conf.level.eq("debug") {
                        true
                    } else if conf.level.eq("warning") {
                        !severity.eq("debug")
                    } else if conf.level.eq("error") {
                        severity.eq("error")
                    } else {
                        false
                    }
                };

                bach_module::wait_for_running_status(run_control);
                if let Some(path) = config_path.lock()?.borrow().as_ref() {
                    let conf: ReporterConfig =
                        quick_xml::de::from_reader(BufReader::new(File::open(path)?))?;
                    let fname = tmp_format(&conf.name);
                    let tmpfile = File::open(&fname)?;
                    let rawlines = BufReader::new(tmpfile).lines();
                    let mut lines: Vec<String> = Vec::new();
                    for l in rawlines.flatten() {
                        lines.push(l);
                    }
                    let mail_and_severity =
                        gen_mail(lines, &conf.clone().template.map(PathBuf::from))?;

                    if check_level(&conf, &mail_and_severity.1) {
                        let stat = conf
                            .mailcmd(mail_and_severity.0, translate_severity(mail_and_severity.1))?
                            .status()?;

                        if !stat.success() {
                            if let Ok(stack) = message_stack.lock() {
                                stack.borrow_mut().push(Packet::new_ne(
                                    "Reporter could not send mail",
                                    &name.lock()?.borrow().to_string(),
                                    "fire",
                                ));
                            }
                        }
                    }
                    fs::remove_file(&tmp_format(&conf.name))?;
                }
                Ok(())
            },
        )
    }

    fn init(&self) -> ModResult<()> {
        #[cfg(feature = "debug")]
        println!("Initializing reporter with {:?}", &self.config_file);
        if let Some(config_file) = &self.config_file {
            match quick_xml::de::from_reader(BufReader::new(File::open(config_file)?)) {
                Ok(conf) => {
                    self.outlet(if self.name().contains("error") {
                        Packet::new_ne("ERROR", &self.name(), "Init")
                    } else {
                        init_tmp_file(&conf)?;
                        Packet::new_ng(
                            &format!("{} reporter module initialized", &self.name()),
                            &self.name(),
                            "Init",
                        )
                    });
                    Ok(())
                },
                Err(e) => {
                    Err(ModError::new(
                        &format!("Reporter : Config File: {}:{}", config_file.to_str().unwrap_or("NOT FOUND"), e.to_string())
                    ))
                }
            }
        } else {
            Err(ModError::new(
                "Reporter : No config file passed"
            ))
        }
    }

    fn destroy(&self) -> ModResult<()> {
        if let Some(config_file) = &self.config_file {
            let conf: ReporterConfig =
                quick_xml::de::from_reader(BufReader::new(File::open(config_file)?))?;
            fs::remove_file(&tmp_format(&conf.name))?;
        }

        Ok(())
    }

    fn run_status(&self) -> &Arc<AtomicU8> {
        &self.ctrl
    }

    fn config_path(&self) -> Option<PathBuf> {
        self.config_file.clone()
    }

    fn emit_alive_status(&self) -> &Arc<AtomicBool> {
        &self.out_alive
    }

    fn message_stack(&self) -> &Arc<Mutex<RefCell<Vec<Packet>>>> {
        &self.out_stack
    }

    fn inlet(&self, p: Packet) {
        let is_provider = move |conf: &ReporterConfig, n: &Notification| {
            for s in &conf.source {
                if s.0.eq(&n.provider) {
                    return true;
                }
            }

            false
        };
        let filter = move |p: Packet, prefix: &str| -> ModResult<()> {
            if let Some(config_file) = &self.config_file {
                let conf: ReporterConfig =
                    quick_xml::de::from_reader(BufReader::new(File::open(config_file)?))?;
                let notif = Notification::from(p);
                if is_provider(&conf, &notif) {
                    let file = fs::OpenOptions::new()
                        .append(true)
                        .open(&tmp_format(&conf.name))?;
                    let mut file = LineWriter::new(file);
                    let nowstr = chrono::Local::now().to_rfc2822();
                    let form = format!("[{}] {}:{}\n", nowstr, prefix, notif.message);
                    file.write_all(form.as_bytes())?;
                }
            }

            Ok(())
        };

        let init_file_wrap = move || -> ModResult<()> {
            if let Some(config_file) = &self.config_file {
                let conf: ReporterConfig =
                    quick_xml::de::from_reader(BufReader::new(File::open(config_file)?))?;
                if !std::path::Path::new(&tmp_format(&conf.name)).exists() {
                    init_tmp_file(&conf)?;
                }
            }
            Ok(())
        };

        let route = move |p: Packet| -> ModResult<()> {
            match p {
                Packet::NotifyCom(_) => filter(p, "INFO")?,
                Packet::NotifyGood(_) => filter(p, "GOOD")?,
                Packet::NotifyWarn(_) => filter(p, "WARN")?,
                Packet::NotifyErr(_) => filter(p, "ERROR")?,
                _ => (),
            }
            Ok(())
        };

        if init_file_wrap().is_err() {
            println!(
                "Reporter {} cannot create file {}",
                &self.name(),
                tmp_format(&self.name()),
            );
        }

        if route(p).is_err() {
            println!(
                "Reporter {} cannot write to file {}",
                &self.name(),
                tmp_format(&self.name())
            );
        }
    }
}

#[cfg(feature = "modular")]
mk_create_module!(Reporter, Reporter::new);
