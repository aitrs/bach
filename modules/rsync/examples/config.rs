use rsync::rsynconfig::*;
pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    gen_dummy_configs(std::path::PathBuf::from("example.config.xml"))
}
