use bach_bus::packet::*;
use bach_module::*;
use std::cell::RefCell;
use std::fs::File;
use std::io::{prelude::*, BufReader, LineWriter};
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, AtomicU8, Ordering},
    Arc, Mutex,
};
extern crate bach_module_tests;
use bach_module_tests::*;

pub mod reporterconfig;
use reporterconfig::ReporterConfig;

fn tmp_format(s: &str) -> String {
    format!("/tmp/{}.bach.report", s)
}

#[derive(BachModuleStdTests)]
pub struct Reporter {
    ctrl: Arc<AtomicU8>,
    out_alive: Arc<AtomicBool>,
    config_file: Option<PathBuf>,
    out_stack: Arc<Mutex<RefCell<Vec<Packet>>>>,
}

impl Reporter {
    pub fn new(config_filename: Option<String>) -> Self {
        Reporter {
            ctrl: Arc::new(AtomicU8::new(0)),
            out_alive: Arc::new(AtomicBool::new(false)),
            config_file: match config_filename {
                Some(s) => Some(PathBuf::from(s)),
                None => None,
            },
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
                bach_module::wait_for_running_status(run_control);
                if let Some(path) = config_path.lock()?.borrow().as_ref() {
                    // TODO: Read lines and compose mail with them in /tmp/{name}.bach.report
                }
                Ok(())
            },
        )
    }

    fn init(&self) -> ModResult<()> {
        if let Some(config_file) = &self.config_file {
            let conf: ReporterConfig =
                quick_xml::de::from_reader(BufReader::new(File::open(config_file)?))?;
            File::create(&tmp_format(&conf.name))?;
            self.outlet(if self.name().contains("error") {
                Packet::new_ne("ERROR", &self.name(), "Init")
            } else {
                Packet::new_ng(
                    &format!("{} reporter module initialized", &self.name()),
                    &self.name(),
                    "Init",
                )
            });
        }
        Ok(())
    }

    fn destroy(&self) -> ModResult<()> {
        if let Some(config_file) = &self.config_file {
            let conf: ReporterConfig =
                quick_xml::de::from_reader(BufReader::new(File::open(config_file)?))?;
            std::fs::remove_file(&tmp_format(&conf.name))?;
        }

        Ok(())
    }

    fn run_status(&self) -> &Arc<AtomicU8> {
        &self.ctrl
    }

    fn config_path(&self) -> Option<PathBuf> {
        if let Some(path) = &self.config_file {
            Some(path.clone())
        } else {
            None
        }
    }

    fn emit_alive_status(&self) -> &Arc<AtomicBool> {
        &self.out_alive
    }

    fn message_stack(&self) -> &Arc<Mutex<RefCell<Vec<Packet>>>> {
        &self.out_stack
    }

    fn inlet(&self, p: Packet) {
        let is_provider = move |conf: &ReporterConfig, n: &Notification| {
            for s in &conf.sources {
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
                    let file = File::open(&tmp_format(&conf.name))?;
                    let mut file = LineWriter::new(file);
                    let nowstr = chrono::Utc::now().to_rfc2822();
                    let form = format!("[{}] {}:{}\n", nowstr, prefix, notif.message);
                    file.write_all(&form.as_bytes());
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

        if route(p).is_err() {
            println!(
                "Reporter {} cannot write to file {}",
                &self.name(),
                tmp_format(&self.name())
            );
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}