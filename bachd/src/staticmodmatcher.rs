use bach_module::*;
use dummy::Dummy;
use noreply_select::NoreplySelect;
use reporter::Reporter;
use rsync::Rsync;
use stdlogger::StdLogger;

pub fn fetch(name: &str, config: Option<String>) -> ModResult<Box<dyn Module>> {
    match name {
        "dummy" => Ok(Box::new(Dummy::new(config))),
        "stdlogger" => Ok(Box::new(StdLogger::new(config))),
        "rsync" => Ok(Box::new(Rsync::new(config))),
        "noreply_select" => Ok(Box::new(NoreplySelect::new(config))),
        "reporter" => Ok(Box::new(Reporter::new(config))),
        _ => Err(ModError::new(&format!(
            "The module {} was not embedded at compile time",
            name
        ))),
    }
}
