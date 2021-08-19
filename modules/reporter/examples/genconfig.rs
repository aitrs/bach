use reporter::reporterconfig::ReporterConfig;
use reporter::reporterconfig::ReporterSource;

pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ReporterConfig {
        name: "test".to_string(),
        sources: vec![ReporterSource("test1".to_string()), ReporterSource("test2".to_string())],
        template: Some("machin.xml".to_string()),
        mail_cmd: "noreply_select -p MAILBODY".to_string(),
    };
    quick_xml::se::to_writer(std::fs::File::create("reporter.example.xml")?, &config)?;
    Ok(())
}

