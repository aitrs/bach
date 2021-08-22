use bach_module::*;
use reporter::Reporter;
use rsync::Rsync;
use stdlogger::StdLogger;

pub fn fetch(name: &str, config: &Option<String>) -> ModResult<Box<dyn Module>> {
    match name {
        "stdlogger" => Ok(Box::new(StdLogger::new(config))),
        "rsync" => Ok(Box::new(Rsync::new(config))),
        "reporter" => Ok(Box::new(Reporter::new(config))),
        _ => Err(ModError::new(&format!(
            "The module {} was not embedded at compile time",
            name
        ))),
    }
}
