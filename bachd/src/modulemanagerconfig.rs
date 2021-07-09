use bach_module::{ModError, ModResult};
use chrono::{Datelike, Utc};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Whence {
    pub hour: String,
    pub min: String,
}

fn get_month_len_as_secs(month: u64) -> u64 {
    let now = Utc::now();
    match month {
        1 => 2678400,
        2 => {
            if now.year() % 4 == 0 {
                2505600
            } else {
                2419200
            }
        }
        3 => 2678400,
        4 => 2592000,
        5 => 2678400,
        6 => 2592000,
        7 => 2678400,
        8 => 2678400,
        9 => 2592000,
        10 => 2678400,
        11 => 2592000,
        12 => 2678400,
        _ => 2678400,
    }
}

impl Whence {
    pub fn get_whence(&self) -> ModResult<u64> {
        let now = Utc::now();
        let yse = now.year() as u64 - 1970;
        let nyb = yse / 4;
        let nyn = yse - nyb;
        let yearstamp = nyb * 366 * 24 * 60 * 60 + nyn * 365 * 24 * 60 * 60;
        let mut ret = yearstamp;

        for i in 0..now.month() - 1 {
            ret += get_month_len_as_secs(i as u64 + 1);
        }

        for _ in 1..now.day() + 1 {
            ret += 24 * 60 * 60;
        }

        if self.hour.eq("*") {
            ret = 3600;
        } else {
            let h: u64 = self.hour.parse()?;
            for _ in 0..h {
                ret += 3600;
            }
        }

        if self.min.eq("*") {
            ret = 60;
        } else {
            let m: u64 = self.min.parse()?;
            for _ in 0..m {
                ret += 60;
            }
        }

        Ok(ret)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleDefinition {
    pub cyclic: bool,
    pub whence: Option<Whence>,
    #[cfg(feature = "modular")]
    pub file: String,
    #[cfg(feature = "static")]
    pub name: String,
    #[serde(rename = "config-file")]
    pub config: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleManagerConfig {
    pub respawn_duration: u64,
    pub modules: Vec<ModuleDefinition>,
}

impl ModuleManagerConfig {
    pub fn from_xml_file(file: PathBuf) -> ModResult<Self> {
        let f = BufReader::new(File::open(file)?);
        match quick_xml::de::from_reader(f) {
            Ok(o) => Ok(o),
            Err(e) => Err(ModError::new(&e.to_string())),
        }
    }

    pub fn save_to_file(&self, file: PathBuf) -> ModResult<()> {
        let outfile = BufWriter::new(File::create(file)?);
        match quick_xml::se::to_writer(outfile, &self) {
            Ok(_) => Ok(()),
            Err(e) => Err(ModError::new(&e.to_string())),
        }
    }
}
