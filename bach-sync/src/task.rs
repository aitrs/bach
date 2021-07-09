use crate::net::host::Host;
use chrono::{Datelike, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TaskCron {
    pub year: u64,
    pub month: u64,
    pub day: u64,
    pub hour: u64,
    pub min: u64,
}

fn get_month_len_as_secs() -> u64 {
    let now = Utc::now();
    match now.month() {
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

fn get_year_length_as_secs() -> u64 {
    let now = Utc::now();
    if now.year() % 4 == 0 {
        return 31622400;
    }
    31536000
}

impl TaskCron {
    pub fn get_whence(&self) -> u64 {
        self.min * 60
            + self.hour * 3600
            + self.day * 86400
            + self.month * get_month_len_as_secs()
            + self.year * get_year_length_as_secs()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskCustomOkReturnCodes(pub Vec<i64>);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskCustomWarnReturnCodes(pub Vec<i64>);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskCustomErrorReturnCodes(pub Vec<i64>);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename = "custom-script")]
pub struct TaskCustomScript {
    pub file: PathBuf,
    pub interpretor: Option<String>,
    #[serde(rename = "valid-return-codes")]
    pub ok_return_codes: TaskCustomOkReturnCodes,
    #[serde(rename = "warning-return-codes")]
    pub warn_return_codes: TaskCustomWarnReturnCodes,
    #[serde(rename = "error-return-codes")]
    pub err_return_codes: TaskCustomErrorReturnCodes,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskFsDeltaSync {
    pub src: PathBuf,
    pub dest: PathBuf,
    #[serde(rename = "max-threads")]
    pub max_threads: u64,
    #[serde(rename = "exclude-file")]
    pub exclude_file: PathBuf,
    #[serde(rename = "prelude-scripts")]
    pub pre_scripts: Vec<TaskCustomScript>,
    #[serde(rename = "post-scripts")]
    pub post_scripts: Vec<TaskCustomScript>,
    pub whence: TaskCron,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSshDeltaSync {
    pub src: PathBuf,
    pub dest: PathBuf,
    #[serde(rename = "max-threads")]
    pub max_threads: u64,
    #[serde(rename = "max-retries")]
    pub max_retries: u32,
    #[serde(rename = "exclude-file")]
    pub exclude_file: PathBuf,
    pub host: Host,
    #[serde(rename = "prelude-scripts")]
    pub pre_scripts: Vec<TaskCustomScript>,
    #[serde(rename = "post-scripts")]
    pub post_scripts: Vec<TaskCustomScript>,
    pub whence: TaskCron,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskVirtDisk {
    pub file: PathBuf,
    pub device: String,
}

#[cfg(feature = "virt-support")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskFsVirtSync {
    pub connection: String,
    pub domain: String,
    pub disks: Vec<TaskVirtDisk>,
    #[serde(rename = "snap-name")]
    pub snap_name: String,
    #[serde(rename = "snap-count")]
    pub snap_count: usize,
    pub src: PathBuf,
    pub dest: PathBuf,
    #[serde(rename = "prelude-scripts")]
    pub pre_scripts: Vec<TaskCustomScript>,
    #[serde(rename = "post-scripts")]
    pub post_scripts: Vec<TaskCustomScript>,
    pub whence: TaskCron,
}

#[cfg(feature = "virt-support")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSshVirtSync {
    pub connection: String,
    pub domain: String,
    pub disks: Vec<TaskVirtDisk>,
    #[serde(rename = "snap-name")]
    pub snap_name: String,
    #[serde(rename = "snap-count")]
    pub snap_count: usize,
    pub src: PathBuf,
    pub dest: PathBuf,
    pub host: Host,
    #[serde(rename = "prelude-scripts")]
    pub pre_scripts: Vec<TaskCustomScript>,
    #[serde(rename = "post-scripts")]
    pub post_scripts: Vec<TaskCustomScript>,
    pub whence: TaskCron,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskCustomSync {
    #[serde(rename = "prelude-scripts")]
    pub pre_scripts: Vec<TaskCustomScript>,
    #[serde(rename = "post-scripts")]
    pub post_scripts: Vec<TaskCustomScript>,
    #[serde(rename = "main-script")]
    pub main_script: TaskCustomScript,
    pub whence: TaskCron,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Task {
    #[serde(rename = "filesystem-delta-synchro")]
    FileSystemDeltaSyncro(TaskFsDeltaSync),
    #[serde(rename = "ssh-delta-synchro")]
    SshDeltaSynchro(TaskSshDeltaSync),
    #[cfg(feature = "virt-support")]
    #[serde(rename = "filesystem-virt-synchro")]
    FileSystemVirtSynchro(TaskFsVirtSync),
    #[cfg(feature = "virt-support")]
    #[serde(rename = "ssh-virt-synchro")]
    SshVirtSynchro(TaskSshVirtSync),
    #[serde(rename = "custom-synchro")]
    CustomSynchro(TaskCustomSync),
}

impl Task {
    pub fn whence(&self) -> TaskCron {
        match self {
            Task::FileSystemDeltaSyncro(e) => e.whence,
            Task::SshDeltaSynchro(e) => e.whence,
            #[cfg(feature = "virt-support")]
            Task::FileSystemVirtSynchro(e) => e.whence,
            #[cfg(feature = "virt-support")]
            Task::SshVirtSynchro(e) => e.whence,
            Task::CustomSynchro(e) => e.whence,
        }
    }

    pub fn get_desc(&self) -> String {
        match self {
            Task::FileSystemDeltaSyncro(_) => "Filesystem Delta Synchro".to_string(),
            Task::SshDeltaSynchro(_) => "Ssh Delta Synchro".to_string(),
            #[cfg(feature = "virt-support")]
            Task::FileSystemVirtSynchro(_) => "Filesystem Libvirt Synchro".to_string(),
            #[cfg(feature = "virt-support")]
            Task::SshVirtSynchro(_) => "Ssh Libvirt Synchro".to_string(),
            Task::CustomSynchro(_) => "Custom Synchro".to_string(),
        }
    }
}
