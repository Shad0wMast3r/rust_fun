mod cli;
mod virsh;
mod agent;
mod probe;
mod utils;

use std::sync::Arc;
use std::time::Duration;
use probe::ProbeManager;

/// Entry point: perform an initial synchronous VM scan (OS, memory, CPU time),
/// print a human-friendly table, then enter the interactive CLI.
fn main() -> anyhow::Result<()> {
    let libvirt_uri = std::env::var("LIBVIRT_URI").unwrap_or_else(|_| "qemu:///system".into());
    let timeout = Duration::from_secs(5);
    let cache_ttl = Duration::from_secs(60);

    let probe_mgr = Arc::new(ProbeManager::new(libvirt_uri, timeout, cache_ttl)?);

    // --- STARTUP SCAN: enumerate VMs and print OS / memory / CPU table ---
    match crate::virsh::list_vms() {
        Ok(vms) => {
            if vms.is_empty() {
                println!("No VMs found (virsh returned no names).\n");
            } else {
                println!("{:20} {:40} {:24} {}", "VM", "OS", "Memory (used/max)", "CPU time");
                for vm in vms {
                    // OS probe (cached by ProbeManager)
                    let os = match probe_mgr.get_os(&vm) {
                        Ok(Some(s)) => s,
                        Ok(None) => "(unknown)".to_string(),
                        Err(e) => format!("error: {}", e),
                    };

                    // dominfo probe (raw virsh output -> parsed DomInfo)
                    let dominfo = match crate::virsh::dominfo_raw(&vm) {
                        Ok(raw) => crate::utils::parse_dominfo(&raw),
                        Err(_) => crate::utils::DomInfo { max_memory_mb: None, used_memory_mb: None, cpu_time: None },
                    };

                    // Memory formatting: parse_dominfo returns numeric tokens (treat as KiB)
                    let mem_used = crate::utils::format_memory_kib(dominfo.used_memory_mb);
                    let mem_max = crate::utils::format_memory_kib(dominfo.max_memory_mb);
                    let mem = if mem_used != "(unknown)" && mem_max != "(unknown)" {
                        format!("{} / {}", mem_used, mem_max)
                    } else if mem_used != "(unknown)" {
                        mem_used
                    } else if mem_max != "(unknown)" {
                        mem_max
                    } else {
                        "(unknown)".to_string()
                    };

                    // CPU time: try to parse into seconds and pretty-print; fallback to raw string
                    let cpu = dominfo.cpu_time
                        .as_deref()
                        .and_then(|s| crate::utils::parse_cpu_time_to_seconds(s))
                        .map(|secs| crate::utils::format_seconds_dhms(secs))
                        .unwrap_or_else(|| dominfo.cpu_time.clone().unwrap_or_else(|| "(unknown)".to_string()));

                    println!("{:20} {:40} {:24} {}", vm, os, mem, cpu);
                }
                println!(); // blank line before menu
            }
        }
        Err(e) => {
            eprintln!("Warning: failed to list VMs on startup: {}", e);
        }
    }
    // --- END STARTUP SCAN ---

    // Enter interactive CLI (blocking)
    cli::run(probe_mgr)?;
    Ok(())
}
