use crate::synchrotrait::*;
use crate::task::TaskCustomScript;

pub struct ScriptRunner(pub TaskCustomScript);

impl Synchro for ScriptRunner {
    fn sync(&self) -> SynchroResult<SynchroState> {
        let interp = match &self.0.interpretor {
            Some(i) => i,
            None => "sh",
        };
        let out = std::process::Command::new(interp)
            .arg(self.0.file.to_str().unwrap_or("echo"))
            .output()?;

        match out.status.code() {
            Some(c) => {
                let comp = c as i64;
                if self.0.ok_return_codes.0.contains(&comp) {
                    return Ok(SynchroState::Good(c as u64));
                } else if self.0.warn_return_codes.0.contains(&comp) {
                    return Ok(SynchroState::Warn(c as u64));
                } else if self.0.err_return_codes.0.contains(&comp) {
                    return Err(SynchroError::new(
                        format!(
                            "Synchro échouée : Le script a renvoyé une valeur de {}",
                            comp
                        ),
                        32,
                    ));
                } else {
                    return Ok(SynchroState::Warn(0));
                }
            }
            None => {
                return Ok(SynchroState::Warn(0));
            }
        }
    }
}
