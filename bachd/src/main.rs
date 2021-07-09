#[cfg(all(feature = "modular", feature = "static"))]
compile_error!("Static and modular features cannot be used at the same time !!!");
use bachd::daemon::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    spawn()?;
    Ok(())
}
