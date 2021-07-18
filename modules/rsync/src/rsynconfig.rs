use crate::host::Host;
use bach_module::ModResult;
use chrono::{prelude::*, Local, Weekday};
use serde::{Deserialize, Serialize};
use std::io::prelude::*;
use std::net::Ipv4Addr;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Label(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename = "mount-point")]
pub struct MountPoint {
    pub device: String,
    pub path: String,
    #[serde(rename = "loop")]
    pub oloop: bool,
    pub offset: Option<u32>,
    pub unmount: bool,
    #[serde(rename = "check-only")]
    pub check_only: Option<bool>,
}

// This was an enum. But serde miserably fails when deserializing nested enums
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerdeTargetType {
    pub directory: Option<String>,
    pub mount: Option<MountPoint>,
}

pub enum TargetType {
    Directory(String),
    Mount(MountPoint),
}

impl SerdeTargetType {
    pub fn to_enum(&self) -> TargetType {
        match &self.directory {
            Some(s) => TargetType::Directory(s.to_string()),
            None => match &self.mount {
                Some(m) => TargetType::Mount(m.clone()),
                None => TargetType::Directory("non".to_string()),
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Exclude(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source(pub String);

#[derive(Debug, Serialize, Deserialize)]
pub struct RsynConfigItem {
    #[serde(rename = "type")]
    pub ttype: SerdeTargetType,
    pub source: Source,
    pub exclude: Option<Exclude>,
    pub host: Option<Host>,
    #[serde(rename = "source-host")]
    pub source_host: Option<Host>,
    #[serde(rename = "use-host-name")]
    pub use_host_name: bool,
    #[serde(rename = "day-by-day")]
    pub day_by_day: bool,
    #[serde(rename = "stamp-name")]
    pub stamp_name: Option<String>,
    pub author: Option<String>,
    pub timeout: Option<u64>,
    pub delete: bool,
}

#[allow(unused_variables)]
impl RsynConfigItem {
    pub fn get_desc(&self) -> String {
        match &self.ttype.to_enum() {
            TargetType::Directory(e) => e.to_string(),
            TargetType::Mount(e) => format!("{} -> {}", e.device, e.path),
        }
    }

    fn genpathstr(&self, ttype: &TargetType, dbd: bool) -> String {
        if dbd {
            format!(
                "{}/{}/",
                match ttype {
                    TargetType::Directory(e) => e,
                    TargetType::Mount(e) => &e.path,
                },
                match Local::now().weekday() {
                    Weekday::Mon => "Monday".to_string(),
                    Weekday::Tue => "Tuesday".to_string(),
                    Weekday::Wed => "Wednesday".to_string(),
                    Weekday::Thu => "Thursday".to_string(),
                    Weekday::Fri => "Friday".to_string(),
                    Weekday::Sat => "Saturday".to_string(),
                    Weekday::Sun => "Sunday".to_string(),
                }
            )
        } else {
            match ttype {
                TargetType::Directory(e) => e.to_string(),
                TargetType::Mount(e) => e.path.to_string(),
            }
        }
    }

    pub fn to_cmd(&self) -> Command {
        let mut ret = if self.source_host.is_some() {
            Command::new("ssh")
        } else {
            Command::new("rsync")
        };

        match &self.source_host {
            Some(h) => {
                ret.arg(&format!(
                    "{}@{}",
                    h.user(),
                    if self.use_host_name {
                        h.name()
                    } else {
                        h.ip().to_string()
                    }
                ))
                .arg("rsync");
            }
            None => (),
        }

        match &self.host {
            Some(h) => {
                ret.args(&["-a", "-z", "-e", "ssh"]);
            }
            None => {
                ret.args(&["-a"]);
            }
        }

        if self.delete {
            ret.arg("--delete");
        }

        match &self.exclude {
            Some(e) => {
                ret.arg(&format!("--exclude-from={}", e.0));
            }
            None => (),
        }
        ret.arg(&self.source.0.to_string());
        match &self.host {
            Some(h) => {
                ret.arg(&format!(
                    "{}@{}:{}",
                    h.user(),
                    if self.use_host_name {
                        h.name()
                    } else {
                        h.ip().to_string()
                    },
                    self.genpathstr(&self.ttype.to_enum(), self.day_by_day)
                ));
            }
            None => {
                ret.arg(self.genpathstr(&self.ttype.to_enum(), self.day_by_day));
            }
        }
        ret
    }

    pub fn check_host_ping(&self) -> bool {
        if let Some(h) = &self.host {
            println!("Host IP {:?}", h.ip());
        }
        match &self.host {
            Some(h) => match h.ping_test(5) {
                Ok(count) => count > 2,
                Err(_) => false,
            },
            None => true,
        }
    }

    pub fn check_mounted(&self) -> ModResult<bool> {
        #[cfg(test)]
        println!("Checking if target is mounted");
        match &self.ttype.to_enum() {
            TargetType::Directory(_) => Ok(true),
            TargetType::Mount(e) => {
                let mut path = e.path.to_string();
                if path.ends_with("/") {
                    path.truncate(path.len() - 1);
                }

                let mut cmd = if self.host.is_some() {
                    Command::new("ssh")
                } else {
                    Command::new("df")
                };

                match &self.host {
                    Some(h) => {
                        cmd.args(&[
                            &format!(
                                "{}@{}",
                                h.user(),
                                if self.use_host_name {
                                    h.name()
                                } else {
                                    h.ip().to_string()
                                }
                            ),
                            "df",
                            "-h",
                        ]);
                    }
                    None => {
                        cmd.arg("-h");
                    }
                }
                #[cfg(test)]
                println!("Spawning {:?}", &cmd);
                let child = cmd.stdout(Stdio::piped()).spawn()?;
                for line in std::io::BufReader::new(child.stdout.unwrap()).lines() {
                    let s = line?.to_string();
                    #[cfg(test)]
                    println!("Scanning line {} for path {}", &s, &path);
                    if s.contains(&path) {
                        #[cfg(test)]
                        println!("Line contains {} => target is already mounted", e.path);
                        return Ok(true);
                    }
                }
                Ok(false)
            }
        }
    }

    pub fn mount_target(&self) -> ModResult<bool> {
        match &self.ttype.to_enum() {
            TargetType::Directory(_) => Ok(true),
            TargetType::Mount(e) => {
                let run = match e.check_only {
                    Some(b) => !b,
                    None => true,
                };

                if !run {
                    return Ok(true);
                }
                if self.check_mounted()? {
                    Ok(true)
                } else {
                    let mut cmd = if self.host.is_some() {
                        Command::new("ssh")
                    } else {
                        Command::new("mount")
                    };

                    match &self.host {
                        Some(h) => {
                            cmd.arg(format!(
                                "{}@{}",
                                h.user(),
                                if self.use_host_name {
                                    h.name()
                                } else {
                                    h.ip().to_string()
                                }
                            ))
                            .arg("mount");
                        }
                        None => (),
                    }
                    if e.oloop {
                        let options = match e.offset {
                            Some(val) => format!("{},offset={}", "loop", val),
                            None => "loop".to_string(),
                        };
                        cmd.arg("-o").arg(&options);
                    }
                    cmd.arg(&e.device).arg(&e.path);
                    #[cfg(test)]
                    println!("Mount command : {:?}", &cmd);
                    let mut child = cmd.spawn()?;
                    let start = Instant::now();
                    loop {
                        if let Some(stat) = child.try_wait()? {
                            std::thread::sleep(Duration::from_secs(10));
                            return Ok(stat.success());
                        } else if start.elapsed().gt(&Duration::from_secs(600)) {
                            return Err(bach_module::ModError::new(&format!("Mount command {:?} didn't returned after 10 minutes, should check target, aborting job", &cmd)));
                        }
                    }
                }
            }
        }
    }

    pub fn umount_target(&self) -> Result<bool, Box<dyn std::error::Error>> {
        match &self.ttype.to_enum() {
            TargetType::Directory(_) => Ok(true),
            TargetType::Mount(e) => {
                if e.unmount {
                    let mut cmd = if self.host.is_some() {
                        Command::new("ssh")
                    } else {
                        Command::new("umount")
                    };

                    match &self.host {
                        Some(h) => {
                            cmd.arg(format!(
                                "{}@{}",
                                h.user(),
                                if self.use_host_name {
                                    h.name()
                                } else {
                                    h.ip().to_string()
                                }
                            ))
                            .arg("umount");
                        }
                        None => (),
                    }
                    cmd.arg(&e.path);
                    let stat = cmd.status()?;
                    Ok(stat.success())
                } else {
                    Ok(true)
                }
            }
        }
    }

    pub fn check_device(&self) -> Result<bool, Box<dyn std::error::Error>> {
        match &self.ttype.to_enum() {
            TargetType::Directory(_) => Ok(true),
            TargetType::Mount(e) => {
                let mut cmd = match &self.host {
                    Some(h) => Command::new("ssh")
                        .arg(&format!(
                            "{}@{}",
                            h.user(),
                            if self.use_host_name {
                                h.name()
                            } else {
                                h.ip().to_string()
                            }
                        ))
                        .arg("ls")
                        .arg(&e.device)
                        .spawn()?,
                    None => Command::new("ls").arg(&e.device).spawn()?,
                };
                Ok(cmd.wait()?.success())
            }
        }
    }

    pub fn check_target(&self) -> Result<bool, Box<dyn std::error::Error>> {
        match &self.ttype.to_enum() {
            TargetType::Directory(e) => match &self.host {
                Some(h) => {
                    let mut cmd = Command::new("ssh")
                        .arg(&format!(
                            "{}@{}",
                            h.user(),
                            if self.use_host_name {
                                h.name()
                            } else {
                                h.ip().to_string()
                            }
                        ))
                        .arg("ls")
                        .arg(&self.genpathstr(&self.ttype.to_enum(), self.day_by_day))
                        .spawn()?;
                    Ok(cmd.wait()?.success())
                }
                None => {
                    let p = PathBuf::from(e);
                    Ok(p.exists())
                }
            },
            TargetType::Mount(e) => Ok(true),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename = "rsync-config")]
pub struct RsynConfig {
    pub label: String,
    #[serde(rename = "synchro")]
    pub synchros: Vec<RsynConfigItem>,
}

pub fn gen_dummy_configs(outfile: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let mut toser = RsynConfig {
        label: "test".to_string(),
        synchros: Vec::new(),
    };

    toser.synchros.push(RsynConfigItem {
        ttype: SerdeTargetType {
            directory: Some("/home/dorian".to_string()),
            mount: None,
        },
        source: Source("/usr/bin".to_string()),
        source_host: Some(Host::new(
            "db",
            Ipv4Addr::new(192, 168, 10, 130),
            "root",
            "passwd",
        )),
        exclude: None,
        host: None,
        use_host_name: true,
        day_by_day: false,
        stamp_name: Some("infos.txt".to_string()),
        author: Some("bachd".to_string()),
        timeout: None,
        delete: false,
    });

    toser.synchros.push(RsynConfigItem {
        ttype: SerdeTargetType {
            directory: None,
            mount: Some(MountPoint {
                device: "/dev/sda1".to_string(),
                path: "/mnt/backup".to_string(),
                oloop: false,
                offset: None,
                unmount: true,
                check_only: None,
            }),
        },
        source: Source("/var/log".to_string()),
        source_host: None,
        exclude: Some(Exclude("/etc/exclude".to_string())),
        host: Some(Host::new(
            "nas",
            Ipv4Addr::new(192, 168, 10, 123),
            "admin",
            "password",
        )),
        use_host_name: false,
        day_by_day: true,
        stamp_name: None,
        author: None,
        timeout: Some(360),
        delete: true,
    });

    let file = std::fs::File::create(outfile)?;

    quick_xml::se::to_writer(file, &toser)?;

    Ok(())
}

#[cfg(test)]
mod test {
    use crate::rsynconfig::*;
    #[test]
    fn rsynconf_to_cmd() {
        let confitem = RsynConfigItem {
            ttype: SerdeTargetType {
                directory: None,
                mount: Some(MountPoint {
                    device: "/dev/sda1".to_string(),
                    path: "/mnt/backup".to_string(),
                    oloop: false,
                    offset: None,
                    unmount: true,
                    check_only: Some(false),
                }),
            },
            source: Source("/var/log".to_string()),
            exclude: Some(Exclude("/etc/exclude".to_string())),
            host: Some(Host::new(
                "nas",
                Ipv4Addr::new(192, 168, 10, 123),
                "admin",
                "password",
            )),
            source_host: None,
            use_host_name: false,
            day_by_day: true,
            stamp_name: None,
            author: None,
            timeout: Some(360),
            delete: true,
        };
        let cmd = confitem.to_cmd();
        println!("{:?}", cmd);
    }
}
