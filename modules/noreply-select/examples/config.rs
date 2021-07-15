use noreply_select::NoreplySelect;
use std::cell::RefCell;
use std::sync::{
    atomic::{AtomicBool, AtomicU8},
    Arc,
    Mutex
};
pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    let norep = NoreplySelect {
        mail_template_file: "/etc/bach/mail.html".to_string(),
        hostname: "stetienne".to_string(),
        log_level: "all".to_string(),
        ctrl: Arc::new(AtomicU8::new(0)),
        out_alive: Arc::new(AtomicBool::new(false)),
        r_log_level: RefCell::new(String::new()),
        message_stack: Arc::new(Mutex::new(RefCell::new(Vec::new()))),
    };
    quick_xml::se::to_writer(std::fs::File::create("noreply.example.xml")?, &norep)?;

    Ok(())
}
