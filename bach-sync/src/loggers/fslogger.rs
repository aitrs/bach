use crate::error::Error;
use crate::loggers::{prefix, Level, Logger};
use std::fs::File;
use std::fs::OpenOptions;
use std::io::prelude::*;

pub struct FsLogger {
    pub dest_file: Option<File>,
    pub err_file: Option<File>,
    pub level: Option<Level>,
}

fn gen_non_usable_err() -> Error {
    Error::new(
        crate::error::Kind::Generic,
        None,
        Some("Logger non utilisable"),
    )
}

impl FsLogger {
    pub fn new(dest: &str, err_dest: Option<&str>, olevel: Option<Level>) -> Self {
        let gen_bad = |lv| FsLogger {
            dest_file: None,
            err_file: None,
            level: lv,
        };
        match err_dest {
            Some(s) => {
                let efi = OpenOptions::new().create(true).append(true).open(s);
                match efi {
                    Ok(efile) => {
                        let rfi = OpenOptions::new().create(true).append(true).open(dest);
                        match rfi {
                            Ok(rfile) => FsLogger {
                                dest_file: Some(rfile),
                                err_file: Some(efile),
                                level: olevel,
                            },
                            Err(_e) => gen_bad(olevel),
                        }
                    }
                    Err(_e) => gen_bad(olevel),
                }
            }
            None => {
                let rfi = OpenOptions::new().create(true).append(true).open(dest);
                match rfi {
                    Ok(rfile) => FsLogger {
                        dest_file: Some(rfile),
                        err_file: None,
                        level: olevel,
                    },
                    Err(_e) => gen_bad(olevel),
                }
            }
        }
    }
}

impl Logger for FsLogger {
    fn log(&mut self, message: &str) -> Result<(), Error> {
        let mut log = false;
        match &self.level {
            Some(l) => {
                if l.eq(&Level::Debug) {
                    log = true;
                }
            }
            None => (),
        }
        if !self.good() {
            return Err(gen_non_usable_err());
        } else if log {
            match &mut self.dest_file {
                Some(f) => {
                    match f.write_all(
                        String::from(std::format!("{}{}\n", prefix(), message)).as_bytes(),
                    ) {
                        Ok(_) => return Ok(()),
                        Err(_) => {
                            return Err(Error::new(
                                crate::error::Kind::Generic,
                                None,
                                Some("Impossible d'écrire dans le logger"),
                            ))
                        }
                    }
                }
                None => return Err(Error::new(crate::error::Kind::BadPath, None, None)),
            }
        }

        Ok(())
    }

    fn err(&mut self, message: &str) -> Result<(), Error> {
        if !self.good() {
            return Err(gen_non_usable_err());
        } else {
            match &self.level {
                None => (),
                Some(_l) => match &mut self.err_file {
                    Some(ef) => {
                        match ef.write_all(
                            String::from(std::format!("{}{}\n", prefix(), message)).as_bytes(),
                        ) {
                            Ok(_) => return Ok(()),
                            Err(_) => {
                                return Err(Error::new(
                                    crate::error::Kind::Generic,
                                    None,
                                    Some("Impossible d'écrire dans le logger"),
                                ))
                            }
                        }
                    }
                    None => match &mut self.dest_file {
                        None => return Err(Error::new(crate::error::Kind::BadPath, None, None)),
                        Some(rf) => {
                            match rf.write_all(
                                String::from(std::format!("{}{}\n", prefix(), message)).as_bytes(),
                            ) {
                                Ok(_) => return Ok(()),
                                Err(_) => {
                                    return Err(Error::new(
                                        crate::error::Kind::Generic,
                                        None,
                                        Some("Impossible d'écrire dans le logger"),
                                    ))
                                }
                            }
                        }
                    },
                },
            }
        }

        Ok(())
    }

    fn warn(&mut self, message: &str) -> Result<(), Error> {
        let mut log = false;
        if !self.good() {
            return Err(gen_non_usable_err());
        } else {
            match &self.level {
                Some(l) => match l {
                    Level::Debug => {
                        log = true;
                    }
                    Level::Warn => {
                        log = true;
                    }
                    _ => (),
                },
                None => (),
            }
        }

        if log {
            match &mut self.dest_file {
                Some(f) => {
                    match f.write_all(
                        String::from(std::format!("{}{}\n", prefix(), message)).as_bytes(),
                    ) {
                        Ok(_) => return Ok(()),
                        Err(_) => {
                            return Err(Error::new(
                                crate::error::Kind::Generic,
                                None,
                                Some("Impossible d'écrire dans le logger"),
                            ))
                        }
                    }
                }
                None => return Err(Error::new(crate::error::Kind::BadPath, None, None)),
            }
        }

        Ok(())
    }

    fn good(&mut self) -> bool {
        match &self.dest_file {
            Some(_f) => true,
            None => false,
        }
    }
}
