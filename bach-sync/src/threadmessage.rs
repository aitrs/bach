use std::io::{Error, ErrorKind};

pub enum ThreadMessage {
    Inf((String, u64)),
    Er((Error, u64)),
    Warn((String, u64)),
}

impl ThreadMessage {
    pub fn info(message: &str, code: u64) -> Self {
        ThreadMessage::Inf((message.to_string(), code))
    }

    pub fn error(kind: ErrorKind, message: &str, code: u64) -> Self {
        ThreadMessage::Er((Error::new(kind, message), code))
    }

    pub fn from_error(e: Error) -> Self {
        ThreadMessage::Er((e, 42))
    }

    pub fn warn(message: &str, code: u64) -> Self {
        ThreadMessage::Warn((message.to_string(), code))
    }

    pub fn get_message(&self) -> String {
        match &self {
            ThreadMessage::Inf(t) => t.0.to_string(),
            ThreadMessage::Er(t) => t.0.to_string(),
            ThreadMessage::Warn(t) => t.0.to_string(),
        }
    }

    pub fn get_code(&self) -> u64 {
        match &self {
            ThreadMessage::Inf(t) => t.1,
            ThreadMessage::Er(t) => t.1,
            ThreadMessage::Warn(t) => t.1,
        }
    }

    pub fn to_string(&self) -> String {
        match &self {
            ThreadMessage::Inf(t) => format!("INFO : Code {} :: Message {}", t.0, t.1),
            ThreadMessage::Er(t) => format!("ERROR : Code {} :: Message {}", t.0.to_string(), t.1),
            ThreadMessage::Warn(t) => format!("WARNING : Code {} :: Message {}", t.0, t.1),
        }
    }
}
