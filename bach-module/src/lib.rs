use bach_bus::packet::Packet;
use std::any::Any;
use std::thread::JoinHandle;

#[derive(Debug, Clone)]
pub struct ModError {
    message: String,
}

impl ModError {
    pub fn new(message: &str) -> Self {
        ModError {
            message: message.to_string(),
        }
    }
}

impl std::error::Error for ModError {}

impl std::fmt::Display for ModError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.message))
    }
}

impl From<std::io::Error> for ModError {
    fn from(item: std::io::Error) -> Self {
        ModError {
            message: item.to_string(),
        }
    }
}

impl From<quick_xml::DeError> for ModError {
    fn from(item: quick_xml::DeError) -> Self {
        ModError {
            message: item.to_string(),
        }
    }
}

impl<T> From<std::sync::PoisonError<T>> for ModError {
    fn from(item: std::sync::PoisonError<T>) -> Self {
        ModError {
            message: item.to_string(),
        }
    }
}

impl From<std::num::ParseIntError> for ModError {
    fn from(item: std::num::ParseIntError) -> Self {
        ModError {
            message: item.to_string(),
        }
    }
}

impl From<std::time::SystemTimeError> for ModError {
    fn from(item: std::time::SystemTimeError) -> Self {
        ModError {
            message: item.to_string(),
        }
    }
}

impl From<Box<dyn std::any::Any + Send>> for ModError {
    fn from(_: Box<dyn std::any::Any + Send>) -> Self {
        ModError {
            message: "Join Error".to_string(),
        }
    }
}

/// Convenience type for results in the ['Module'] trait.
pub type ModResult<T> = Result<T, ModError>;

pub trait Module: Any + Send {
    fn name(&self) -> String;
    fn init(&self) -> ModResult<()>;
    fn spawn(&self) -> JoinHandle<ModResult<()>>;
    fn destroy(&self) -> ModResult<()>;
    fn inlet(&self, p: Packet);
    fn outlet(&self) -> Option<Packet>;
    fn accept(&self, p: Packet) -> bool;
}

#[cfg(feature = "modular")]
#[macro_export]
macro_rules! mk_create_module {
    ($plugin_type: ty, $constructor:path) => {
        #[no_mangle]
        pub extern "C" fn bach_create_module(
            config_filename: Option<String>,
        ) -> Box<dyn $crate::Module> {
            let constructor: fn(Option<String>) -> $plugin_type = $constructor;
            let object = constructor(config_filename);
            let boxed: Box<dyn $crate::Module> = Box::new(object);
            boxed
        }
    };
}
