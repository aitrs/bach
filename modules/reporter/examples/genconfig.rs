use reporter::reporterconfig::ReporterCommand;
use reporter::reporterconfig::ReporterCommandArgument;
use reporter::reporterconfig::ReporterConfig;
use reporter::reporterconfig::ReporterSource;

pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ReporterConfig {
        name: "test".to_string(),
        source: vec![
            ReporterSource("test1".to_string()),
            ReporterSource("test2".to_string()),
        ],
        template: Some("machin.xml".to_string()),
        mail_cmd: ReporterCommand {
            arg: vec![
                ReporterCommandArgument("/usr/bin/noreply_select".to_string()),
                ReporterCommandArgument("-t".to_string()),
                ReporterCommandArgument("-s".to_string()),
                ReporterCommandArgument("Bach Report".to_string()),
                ReporterCommandArgument("-m".to_string()),
                ReporterCommandArgument("MAILBODY".to_string()),
            ],
        },
        level: "debug".to_string(),
    };
    quick_xml::se::to_writer(std::fs::File::create("reporter.example.xml")?, &config)?;
    Ok(())
}
