use std::error::Error;
use std::sync::mpsc::SendError;

use std::fmt::Display;

#[derive(Debug, Clone)]
pub struct SynchroError {
    message: String,
    code: u64,
}

impl Display for SynchroError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "Synchro Error {} : {}",
            self.code, self.message
        ))
    }
}
impl Error for SynchroError {}

impl From<std::io::Error> for SynchroError {
    fn from(item: std::io::Error) -> Self {
        SynchroError {
            message: item.to_string(),
            code: item.raw_os_error().unwrap_or(0) as u64,
        }
    }
}

impl From<quick_xml::DeError> for SynchroError {
    fn from(item: quick_xml::DeError) -> Self {
        SynchroError {
            message: item.to_string(),
            code: 2,
        }
    }
}

impl<T> From<SendError<T>> for SynchroError {
    fn from(item: SendError<T>) -> Self {
        SynchroError {
            message: item.to_string(),
            code: 22,
        }
    }
}

impl From<ssh2::Error> for SynchroError {
    fn from(item: ssh2::Error) -> Self {
        let message = match item.code() {
            ssh2::ErrorCode::Session(_) => format!("Session : {}", item.message()),
            ssh2::ErrorCode::SFTP(_) => format!("Sftp : {}", item.message()),
        };
        let code = match item.code() {
            ssh2::ErrorCode::Session(c) => c as i64,
            ssh2::ErrorCode::SFTP(c) => c as i64,
        };

        SynchroError {
            message,
            code: code as u64,
        }
    }
}

impl From<std::time::SystemTimeError> for SynchroError {
    fn from(item: std::time::SystemTimeError) -> Self {
        SynchroError {
            message: item.to_string(),
            code: 32,
        }
    }
}

impl SynchroError {
    pub fn new(message: String, code: u64) -> Self {
        SynchroError { message, code }
    }

    pub fn code(&self) -> u64 {
        self.code
    }
    pub fn message(&self) -> &str {
        &self.message
    }
}

pub enum SynchroState {
    Good(u64),
    Warn(u64),
}

pub type SynchroResult<T> = Result<T, SynchroError>;

pub trait Synchro {
    fn sync(&self) -> SynchroResult<SynchroState>;
}
