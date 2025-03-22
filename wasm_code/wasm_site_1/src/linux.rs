use std::process::Command;

// Function to get the hostname of a Linux server
pub fn get_linux_hostname() -> String {
    match Command::new("hostname").output() {
        Ok(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        }
        Ok(output) => format!(
            "Command failed with error: {}",
            String::from_utf8_lossy(&output.stderr)
        ),
        Err(err) => format!("Failed to execute command: {}", err),
    }
}

// Function to execute a custom Linux command
pub fn execute_linux_command(command: &str) -> String {
    match Command::new("sh").arg("-c").arg(command).output() {
        Ok(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        }
        Ok(output) => format!(
            "Command failed with error: {}",
            String::from_utf8_lossy(&output.stderr)
        ),
        Err(err) => format!("Failed to execute command: {}", err),
    }
}