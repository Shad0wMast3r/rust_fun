// src/virsh.rs
use std::process::Command;
use std::io;
use serde_json::Value;

/// Simple wrapper to call `virsh qemu-agent-command` and return parsed JSON.
pub fn virsh_qemu_agent(vm: &str, payload: &str, timeout_secs: u64) -> io::Result<Value> {
    let out = Command::new("virsh")
        .args(["qemu-agent-command", "--timeout", &timeout_secs.to_string(), vm, payload])
        .output()?;
    if !out.status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("virsh qemu-agent-command failed: {}", String::from_utf8_lossy(&out.stderr)),
        ));
    }
    let s = String::from_utf8_lossy(&out.stdout);
    let json: Value = serde_json::from_str(&s)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("json parse: {}", e)))?;
    Ok(json)
}

/// Return VM names from `virsh list --all --name`.
/// Trims empty lines and returns Vec<String>.
pub fn list_vms() -> io::Result<Vec<String>> {
    let out = Command::new("virsh")
        .args(["list", "--all", "--name"])
        .output()?;
    if !out.status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("virsh list failed: {}", String::from_utf8_lossy(&out.stderr)),
        ));
    }
    let s = String::from_utf8_lossy(&out.stdout);
    let vms: Vec<String> = s
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .map(|l| l.to_string())
        .collect();
    Ok(vms)
}

/// Return the raw `virsh dominfo <vm>` output as a String.
pub fn dominfo_raw(vm: &str) -> io::Result<String> {
    let out = Command::new("virsh")
        .args(["dominfo", vm])
        .output()?;
    if !out.status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("virsh dominfo failed: {}", String::from_utf8_lossy(&out.stderr)),
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}
