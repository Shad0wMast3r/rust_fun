use std::io::{self, Write};
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

use rustyline::completion::{Completer, FilenameCompleter, Pair};
use rustyline::history::FileHistory;
use rustyline::{Context, Editor, Helper};
use rustyline::hint::Hinter;
use rustyline::highlight::Highlighter;
use rustyline::validate::Validator;

use serde_json::Value;
use base64::engine::general_purpose;
use base64::Engine as _;

/* ---------- Utility ---------- */

// Run a Command and return stdout as String; expects &mut Command so callers can build args inline.
fn run(cmd: &mut Command) -> io::Result<String> {
    let output = cmd.output()?;
    if !output.status.success() {
        return Err(io::Error::new(io::ErrorKind::Other, "command failed"));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/* ---------- VM Discovery ---------- */

fn list_all_vms() -> io::Result<Vec<String>> {
    let mut cmd = Command::new("virsh");
    cmd.args(["list", "--all", "--name"]);
    Ok(run(&mut cmd)?
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(String::from)
        .collect())
}

fn list_running_vms() -> io::Result<Vec<String>> {
    let mut cmd = Command::new("virsh");
    cmd.args(["list", "--name"]);
    Ok(run(&mut cmd)?
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(String::from)
        .collect())
}

fn get_cdrom(vm: &str) -> io::Result<Option<String>> {
    let mut cmd = Command::new("virsh");
    cmd.args(["domblklist", vm]);
    for line in run(&mut cmd)?.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.first().map(|d| d.starts_with("hd")).unwrap_or(false) {
            return Ok(Some(parts[0].to_string()));
        }
    }
    Ok(None)
}

fn get_mounted_iso(vm: &str) -> io::Result<Option<String>> {
    let mut cmd = Command::new("virsh");
    cmd.args(["domblklist", vm]);
    for line in run(&mut cmd)?.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() == 2 && parts[1].ends_with(".iso") {
            return Ok(Some(parts[1].to_string()));
        }
    }
    Ok(None)
}

fn insert_iso(vm: &str, device: &str, iso: &str) -> io::Result<()> {
    let mut cmd = Command::new("virsh");
    cmd.args([
        "change-media",
        vm,
        device,
        "--insert",
        iso,
        "--live",
    ]);
    run(&mut cmd)?;
    Ok(())
}

/* ---------- Guest OS probing (multi-strategy) ---------- */

fn probe_guest_os(vm: &str) -> io::Result<Option<String>> {
    // 1) Try guest-get-osinfo (preferred when available)
    if let Ok(Some(s)) = try_guest_get_osinfo(vm) {
        return Ok(Some(s));
    }

    // 2) Try guest-get-os (older structured RPC)
    if let Ok(Some(s)) = try_guest_get_os(vm) {
        return Ok(Some(s));
    }

    // 3) Fallback to guest-exec multi-probe (reads /etc/os-release, lsb_release, etc.)
    try_guest_exec_multi_probe(vm)
}

/// Query guest-get-osinfo and return a friendly OS string if available.
fn try_guest_get_osinfo(vm: &str) -> io::Result<Option<String>> {
    let payload = r#"{"execute":"guest-get-osinfo"}"#;
    let out = std::process::Command::new("virsh")
        .args(["qemu-agent-command", "--timeout", "5", vm, payload])
        .output()?;
    if !out.status.success() {
        return Ok(None);
    }
    let s = String::from_utf8_lossy(&out.stdout);
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&s) {
        if let Some(ret) = json.get("return") {
            if let Some(pretty_name) = ret.get("pretty-name").and_then(|v| v.as_str()) {
                return Ok(Some(pretty_name.to_string()));
            }
            if let Some(pretty) = ret.get("pretty").and_then(|v| v.as_str()) {
                return Ok(Some(pretty.to_string()));
            }
            if let Some(name) = ret.get("name").and_then(|v| v.as_str()) {
                let ver = ret.get("version").and_then(|v| v.as_str()).unwrap_or("");
                return Ok(Some(if ver.is_empty() { name.to_string() } else { format!("{} {}", name, ver) }));
            }
            return Ok(Some(ret.to_string()));
        }
    }
    Ok(None)
}

fn try_guest_get_os(vm: &str) -> io::Result<Option<String>> {
    let payload = r#"{"execute":"guest-get-os"}"#;
    let mut cmd = Command::new("virsh");
    cmd.args(["qemu-agent-command", "--timeout", "5", vm, payload]);
    let out = cmd.output()?;
    if !out.status.success() {
        return Ok(None);
    }
    let s = String::from_utf8_lossy(&out.stdout);
    if let Ok(json) = serde_json::from_str::<Value>(&s) {
        if let Some(ret) = json.get("return") {
            // prefer "pretty-name" (observed in your output), then "pretty", then name+version
            if let Some(pretty_name) = ret.get("pretty-name").and_then(|v| v.as_str()) {
                return Ok(Some(pretty_name.to_string()));
            }
            if let Some(pretty) = ret.get("pretty").and_then(|v| v.as_str()) {
                return Ok(Some(pretty.to_string()));
            }
            if let Some(name) = ret.get("name").and_then(|v| v.as_str()) {
                let ver = ret.get("version").and_then(|v| v.as_str()).unwrap_or("");
                return Ok(Some(if ver.is_empty() { name.to_string() } else { format!("{} {}", name, ver) }));
            }
            // fallback: stringify the whole return object
            return Ok(Some(ret.to_string()));
        }
    }
    Ok(None)
}

/// Try multiple read-only commands/files inside the guest to discover OS info.
fn try_guest_exec_multi_probe(vm: &str) -> io::Result<Option<String>> {
    let probes = [
        ("/bin/cat", vec!["/etc/os-release"]),
        ("/bin/cat", vec!["/usr/lib/os-release"]),
        ("/usr/bin/lsb_release", vec!["-ds"]),
        ("/bin/cat", vec!["/etc/lsb-release"]),
        ("/bin/cat", vec!["/etc/redhat-release"]),
        ("/bin/sh", vec!["-c", "cat /etc/*-release 2>/dev/null || true"]),
    ];

    fn run_guest_exec_and_get_output(vm: &str, path: &str, args: &[&str]) -> io::Result<Option<String>> {
        let args_json = serde_json::to_string(args).unwrap_or("[]".to_string());
        let payload = format!(
            r#"{{"execute":"guest-exec","arguments":{{"path":"{}","arg":{},"capture-output":true}}}}"#,
            path, args_json
        );

        let mut cmd = Command::new("virsh");
        cmd.args(["qemu-agent-command", "--timeout", "10", vm, &payload]);
        let out = cmd.output()?;
        if !out.status.success() {
            return Ok(None);
        }
        let s = String::from_utf8_lossy(&out.stdout);
        let json: Value = match serde_json::from_str(&s) {
            Ok(j) => j,
            Err(_) => return Ok(None),
        };

        let pid_val = json.get("return").and_then(|r| r.get("pid")).cloned();
        let pid = match pid_val {
            Some(v) if v.is_u64() => v.as_u64().map(|n| n.to_string()),
            Some(v) if v.is_string() => v.as_str().map(|s| s.to_string()),
            _ => None,
        };
        let pid = match pid {
            Some(p) => p,
            None => return Ok(None),
        };

        let start = Instant::now();
        let timeout = Duration::from_secs(12);
        while start.elapsed() < timeout {
            let status_payload_num = format!(r#"{{"execute":"guest-exec-status","arguments":{{"pid":{}}}}}"#, pid);
            let mut cmd_num = Command::new("virsh");
            cmd_num.args(["qemu-agent-command", "--timeout", "8", vm, &status_payload_num]);
            let out_num = cmd_num.output()?;
            let stdout_num = String::from_utf8_lossy(&out_num.stdout);

            if out_num.status.success() {
                if let Ok(status_json) = serde_json::from_str::<Value>(&stdout_num) {
                    if let Some(ret) = status_json.get("return") {
                        if ret.get("exited").and_then(|v| v.as_bool()) == Some(true) {
                            if let Some(out_data) = ret.get("out-data").and_then(|v| v.as_str()) {
                                if let Ok(decoded) = general_purpose::STANDARD.decode(out_data) {
                                    if let Ok(text) = String::from_utf8(decoded) {
                                        return Ok(Some(text));
                                    }
                                }
                            }
                            return Ok(None);
                        }
                    }
                }
            } else {
                let status_payload_str = format!(r#"{{"execute":"guest-exec-status","arguments":{{"pid":"{}"}}}}"#, pid);
                let mut cmd_str = Command::new("virsh");
                cmd_str.args(["qemu-agent-command", "--timeout", "8", vm, &status_payload_str]);
                let out_str = cmd_str.output()?;
                let stdout_str = String::from_utf8_lossy(&out_str.stdout);
                if out_str.status.success() {
                    if let Ok(status_json) = serde_json::from_str::<Value>(&stdout_str) {
                        if let Some(ret) = status_json.get("return") {
                            if ret.get("exited").and_then(|v| v.as_bool()) == Some(true) {
                                if let Some(out_data) = ret.get("out-data").and_then(|v| v.as_str()) {
                                    if let Ok(decoded) = general_purpose::STANDARD.decode(out_data) {
                                        if let Ok(text) = String::from_utf8(decoded) {
                                            return Ok(Some(text));
                                        }
                                    }
                                }
                                return Ok(None);
                            }
                        }
                    }
                }
            }

            std::thread::sleep(Duration::from_millis(300));
        }

        Ok(None)
    }

    for (path, args_vec) in probes.iter() {
        let args_slice: Vec<&str> = args_vec.iter().map(|s| *s).collect();
        if let Ok(Some(raw)) = run_guest_exec_and_get_output(vm, path, &args_slice) {
            let mut text = raw.trim().to_string();
            if text.starts_with('"') && text.ends_with('"') && text.len() >= 2 {
                text = text[1..text.len()-1].to_string();
            }
            if text.contains("PRETTY_NAME=") || text.contains("NAME=") {
                return Ok(Some(parse_os_release(&text)));
            }
            if let Some(line) = text.lines().find(|l| !l.trim().is_empty()) {
                return Ok(Some(line.trim().to_string()));
            }
        }
    }

    Ok(None)
}

fn parse_os_release(content: &str) -> String {
    let mut pretty: Option<String> = None;
    let mut name: Option<String> = None;
    let mut version: Option<String> = None;
    for line in content.lines() {
        if let Some(v) = line.strip_prefix("PRETTY_NAME=") {
            pretty = Some(trim_quotes(v));
        } else if let Some(v) = line.strip_prefix("NAME=") {
            name = Some(trim_quotes(v));
        } else if let Some(v) = line.strip_prefix("VERSION=") {
            version = Some(trim_quotes(v));
        }
    }
    if let Some(p) = pretty {
        p
    } else {
        match (name, version) {
            (Some(n), Some(v)) => format!("{} {}", n, v),
            (Some(n), None) => n,
            _ => "(unknown)".to_string(),
        }
    }
}

fn trim_quotes(s: &str) -> String {
    let s = s.trim();
    if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

/* ---------- Human-readable helpers ---------- */

fn human_mem(mem_kib: u64) -> String {
    let mut size = mem_kib as f64 * 1024.0;
    let units = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut idx = 0usize;
    while size >= 1024.0 && idx < units.len() - 1 {
        size /= 1024.0;
        idx += 1;
    }
    if idx == 0 {
        format!("{} {}", size as u64, units[idx])
    } else {
        format!("{:.1} {}", size, units[idx])
    }
}

fn human_cpu(cpu_ns: u64) -> String {
    if cpu_ns < 1_000_000 {
        return format!("{} ns", cpu_ns);
    }
    let ms = cpu_ns as f64 / 1_000_000.0;
    if ms < 1000.0 {
        return format!("{:.0} ms", ms);
    }
    let total_secs = (cpu_ns / 1_000_000_000) as u64;
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;
    if hours > 0 {
        format!("{}h {:02}m {:02}s", hours, minutes, seconds)
    } else if minutes > 0 {
        format!("{}m {:02}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
}

/* ---------- Status Display ---------- */

fn show_vm_status() -> io::Result<()> {
    let vms = list_all_vms()?;

    println!("\nCurrent VM Status:");
    println!(
        "{:<24} {:<10} {:<16} {:<16} {:<32}",
        "VM", "State", "Memory", "CPU Time", "Guest OS"
    );
    println!("{}", "-".repeat(110));

    for vm in vms {
        let mut cmd_info = Command::new("virsh");
        cmd_info.args(["dominfo", &vm]);
        let info = run(&mut cmd_info)?;

        let mut cmd_stats = Command::new("virsh");
        cmd_stats.args(["domstats", &vm, "--cpu-total", "--balloon"]);
        let stats = run(&mut cmd_stats)?;

        let state = info
            .lines()
            .find(|l| l.starts_with("State:"))
            .and_then(|l| l.split(':').nth(1))
            .unwrap_or("unknown")
            .trim();

        let mem_kib: u64 = info
            .lines()
            .find(|l| l.starts_with("Used memory:"))
            .and_then(|l| l.split(':').nth(1))
            .unwrap_or("0")
            .trim()
            .split_whitespace()
            .next()
            .unwrap_or("0")
            .parse::<u64>()
            .unwrap_or(0);

        let cpu_ns: u64 = stats
            .lines()
            .find(|l| l.contains("cpu.time"))
            .and_then(|l| l.split('=').nth(1))
            .unwrap_or("0")
            .trim()
            .parse::<u64>()
            .unwrap_or(0);

        let mem_human = human_mem(mem_kib);
        let cpu_human = human_cpu(cpu_ns);

        // Best-effort guest OS probe (may be slow for many VMs)
        let guest_os = probe_guest_os(&vm).unwrap_or(None);

        println!(
            "{:<24} {:<10} {:<16} {:<16} {:<32}",
            vm,
            state,
            mem_human,
            cpu_human,
            guest_os.as_deref().unwrap_or("(unknown)")
        );
    }

    Ok(())
}

/* ---------- Rustyline Helpers (rustyline 17.x) ---------- */

struct PathHelper {
    completer: FilenameCompleter,
}

impl Completer for PathHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        self.completer.complete(line, pos, ctx)
    }
}

impl Hinter for PathHelper {
    type Hint = String;
    fn hint(&self, _line: &str, _pos: usize, _ctx: &Context<'_>) -> Option<String> {
        None
    }
}
impl Highlighter for PathHelper {}
impl Validator for PathHelper {}
impl Helper for PathHelper {}

fn readline_path(prompt: &str) -> rustyline::Result<String> {
    let mut rl = Editor::<PathHelper, FileHistory>::new()?;
    rl.set_helper(Some(PathHelper {
        completer: FilenameCompleter::new(),
    }));
    rl.load_history(".iso_tool_history").ok();

    let line = rl.readline(prompt)?;
    rl.add_history_entry(line.as_str())?;
    rl.save_history(".iso_tool_history").ok();

    Ok(line)
}

struct VmHelper {
    vms: Vec<String>,
}

impl Completer for VmHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        _pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        let matches: Vec<Pair> = self
            .vms
            .iter()
            .filter(|vm| vm.starts_with(line))
            .map(|vm| Pair {
                display: vm.clone(),
                replacement: vm.clone(),
            })
            .collect();
        Ok((0, matches))
    }
}

impl Hinter for VmHelper {
    type Hint = String;
    fn hint(&self, _line: &str, _pos: usize, _ctx: &Context<'_>) -> Option<String> {
        None
    }
}
impl Highlighter for VmHelper {}
impl Validator for VmHelper {}
impl Helper for VmHelper {}

fn readline_vm(prompt: &str, vms: Vec<String>) -> rustyline::Result<String> {
    let mut rl = Editor::<VmHelper, FileHistory>::new()?;
    rl.set_helper(Some(VmHelper { vms }));
    rl.load_history(".iso_tool_history").ok();

    let line = rl.readline(prompt)?;
    rl.add_history_entry(line.as_str())?;
    rl.save_history(".iso_tool_history").ok();

    Ok(line)
}

/* ---------- Actions ---------- */

fn scan_vms() -> io::Result<()> {
    let vms = list_running_vms()?;
    println!("\nMounted ISOs:");
    for vm in vms {
        match get_mounted_iso(&vm)? {
            Some(iso) => println!("{} → {}", vm, iso),
            None => println!("{} → (empty)", vm),
        }
    }
    Ok(())
}

fn mount_iso() -> io::Result<()> {
    let running = list_running_vms()?;

    let target = readline_vm("Enter VM name or * for all VMs: ", running.clone())
        .unwrap()
        .trim()
        .to_string();

    let iso = readline_path("Enter full path to ISO: ").unwrap();

    if !Path::new(&iso).is_file() {
        println!("Invalid ISO path.");
        return Ok(());
    }

    let targets = if target == "*" {
        running
    } else {
        vec![target]
    };

    for vm in targets {
        if let Some(cdrom) = get_cdrom(&vm)? {
            insert_iso(&vm, &cdrom, &iso)?;
            println!("Mounted {} on {}", iso, vm);
        } else {
            println!("{} has no CD-ROM device", vm);
        }
    }

    Ok(())
}

/* ---------- Main ---------- */

fn main() -> io::Result<()> {
    loop {
        show_vm_status()?;

        println!("\n1) Mount ISO");
        println!("2) Scan mounted ISOs");
        println!("3) Exit");
        print!("Select option: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        match input.trim() {
            "1" => mount_iso()?,
            "2" => scan_vms()?,
            "3" => break,
            _ => println!("Invalid option"),
        }
    }

    Ok(())
}
