use bach_module::ModResult;
use reporter::mailtemplate::gen_mail;
use reporter::reporterconfig::ReporterConfig;
fn main() -> ModResult<()> {
    let config: ReporterConfig = quick_xml::de::from_reader(std::io::BufReader::new(
        std::fs::File::open("reporter.example.xml")?,
    ))?;
    let lines = vec!["One".to_string(), "Two".to_string()];
    let mail = gen_mail(
        lines,
        &config.clone().template.map(std::path::PathBuf::from),
    )?;
    let cmd = config.mailcmd(mail.0.replace('\n', " "))?;
    println!("{:?}", cmd);
    Ok(())
}
