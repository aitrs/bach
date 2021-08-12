use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReporterSource(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReporterConfig {
    pub name: String,
    pub sources: Vec<ReporterSource>,
    pub template: String,
    #[serde(rename = "mailer-command")]
    pub mailer_command: String,
}
