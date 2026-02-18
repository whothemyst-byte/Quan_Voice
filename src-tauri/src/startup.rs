use std::env;
use std::process::Command;

use thiserror::Error;
const RUN_KEY_PATH: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
const VALUE_NAME: &str = "Quan Voice";

#[derive(Debug, Error)]
pub enum StartupError {
    #[error("failed to resolve current executable path")]
    MissingExecutable,
    #[error("startup registry update failed: {0}")]
    RegistryCommand(String),
}

pub fn sync_startup(enabled: bool) -> Result<(), StartupError> {
    if enabled {
        let exe = env::current_exe().map_err(|_| StartupError::MissingExecutable)?;
        let value = format!("\"{}\"", exe.display());
        let output = Command::new("reg")
            .args([
                "add",
                RUN_KEY_PATH,
                "/v",
                VALUE_NAME,
                "/t",
                "REG_SZ",
                "/d",
                &value,
                "/f",
            ])
            .output()
            .map_err(|err| StartupError::RegistryCommand(err.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(StartupError::RegistryCommand(stderr));
        }
        return Ok(());
    }

    let output = Command::new("reg")
        .args(["delete", RUN_KEY_PATH, "/v", VALUE_NAME, "/f"])
        .output()
        .map_err(|err| StartupError::RegistryCommand(err.to_string()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let missing = stderr.contains("unable to find")
            || stderr.contains("cannot find")
            || stderr.contains("The system was unable");
        if !missing {
            return Err(StartupError::RegistryCommand(stderr));
        }
    }
    Ok(())
}
