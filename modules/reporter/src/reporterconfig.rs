use serde::{Deserialize, Serialize};
use std::process::Command;
use bach_module::{
    ModResult,
    ModError,
};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReporterSource(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename="reporter")]
pub struct ReporterConfig {
    pub name: String,
    pub sources: Vec<ReporterSource>,
    pub template: Option<String>,
    #[serde(rename="mail-command")]
    pub mail_cmd: String,
}

impl ReporterConfig {
    pub fn mailcmd(&self, mailbody: String) -> ModResult<Command> {
        let mut args: Vec<&str> = self.mail_cmd.split(" ").collect();
        if args.len() <= 1 {
            return Err(
                ModError::new("Missing at least MAILBODY placeholder in mail command")
            );
        }
        let mut cmd = Command::new(args.remove(0));
        for a in args {
            if a.eq("MAILBODY") {
                cmd.arg(&format!("\"{}\"", mailbody));
            } else {
                cmd.arg(a);
            }
        }

        Ok(cmd)
    }
}
