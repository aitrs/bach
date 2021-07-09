use crate::error::Error;

#[derive(PartialEq, Eq)]
pub enum Level {
    Error,
    Warn,
    Debug,
}

pub fn prefix() -> String {
    String::from(std::format!("[{}] - ", chrono::Local::now().to_rfc3339()))
}

pub trait Logger {
    fn log(&mut self, message: &str) -> Result<(), Error>;
    fn err(&mut self, message: &str) -> Result<(), Error>;
    fn warn(&mut self, message: &str) -> Result<(), Error>;
    fn good(&mut self) -> bool;
}

pub mod fslogger;
