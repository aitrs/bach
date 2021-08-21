use bach_module::{ModError, ModResult};
use serde::{Deserialize, Serialize};
use std::process::Command;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReporterSource(pub String);
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReporterCommandArgument(pub String);
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReporterCommand {
    pub arg: Vec<ReporterCommandArgument>,
}

impl ReporterCommand {
    pub fn to_cmd(&self, mailbody: String, overall: String) -> ModResult<Command> {
        if self.arg.len() <= 1 {
            return Err(ModError::new(
                "Missing at least MAILBODY placeholder in mail command",
            ));
        }

        let mut vecopy = self.arg.clone();
        let mut cmd = Command::new(vecopy.remove(0).0);
        for arg in vecopy {
            if arg.0.eq("MAILBODY") {
                cmd.arg(mailbody.clone());
            } else if arg.0.contains("STATUS") {
                cmd.arg(arg.0.replace("STATUS", &overall));
            } else {
                cmd.arg(arg.0);
            }
        }
        Ok(cmd)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename = "reporter")]
pub struct ReporterConfig {
    pub name: String,
    pub source: Vec<ReporterSource>,
    pub template: Option<String>,
    #[serde(rename = "mail-command")]
    pub mail_cmd: ReporterCommand,
    pub level: String,
}

impl ReporterConfig {
    pub fn mailcmd(&self, mailbody: String, overall: String) -> ModResult<Command> {
        self.mail_cmd.to_cmd(mailbody, overall)
    }
}
