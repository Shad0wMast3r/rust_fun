### Overview
A compact Rust CLI tool that enumerates libvirt/QEMU virtual machines, probes each guest for OS, memory, and CPU telemetry, prints a human‑readable status table at startup, and provides an interactive menu for on‑demand scans and actions. Designed for reliability, safety, and easy extension into production workflows.

---

### Features
- **Startup VM scan** that lists VM name, detected OS, memory used/max, and normalized CPU time.  
- **Multi‑strategy OS detection** using QEMU guest agent RPCs with conservative fallbacks.  
- **Dominfo parsing** to extract memory and CPU metrics from `virsh dominfo`.  
- **Human readable formatting** for memory (KiB → KiB/MiB/GiB) and CPU time (days/hours/minutes/seconds).  
- **ProbeManager** with configurable timeouts and cache TTL to reduce repeated slow probes.  
- **Modular codebase** split into `cli`, `virsh`, `agent`, `probe`, and `utils` for easy testing and extension.

---

### Installation
- **Prerequisites**: Rust toolchain (cargo), libvirt and virsh installed, appropriate permissions to run `virsh` commands.  
- **Build**:
```bash
git clone <repo>
cd dismount_iso_qemu
cargo build --release
```
- **Run**:
```bash
cargo run
# or use the built binary
target/release/dismount_iso_qemu
```

---

### Usage
- **Startup behavior**: the program performs a synchronous scan and prints a table like:
```
VM                   OS                                       Memory (used/max)     CPU time
pinhole_new          Ubuntu 18.04.6 LTS                       8.0 GiB / 8.0 GiB      1d 10h 5m
...
```
- **Interactive menu**: after the initial scan the CLI shows:
```
1) Mount ISO
2) Scan mounted ISOs
3) Exit
Select option:
```
- **Rescan**: choose option **2** to re-enumerate VMs and refresh probes.  
- **Configuration**: set `LIBVIRT_URI` environment variable to change the libvirt connection string, for example:
```bash
export LIBVIRT_URI="qemu+ssh://root@host/system"
```

---

### Configuration
- **Probe timeout**: configured in `main.rs` via `Duration::from_secs(5)`; increase for slow guests.  
- **Cache TTL**: configured in `ProbeManager` via `Duration::from_secs(60)`; increase to reduce probe frequency.  
- **Localization**: `virsh dominfo` output can vary by locale; adjust `parse_dominfo` if your environment uses non‑English labels.  
- **Productionization tips**:
  - Run as a systemd service or container for continuous monitoring.  
  - Expose metrics (Prometheus) and structured logs for observability.  
  - Parallelize probes with a thread pool or `rayon` for large VM fleets.

---

### Roadmap
- **Background scanning** with a channel to update the CLI without interleaving prompts.  
- **Parallel probes** to reduce startup latency for many VMs.  
- **Cache dominfo** results in `ProbeManager` and add TTL per metric.  
- **Prometheus metrics and health checks** for integration with monitoring systems.  
- **Integration tests** that mock `virsh` and guest agent responses to validate parsing and fallbacks.  

---

**Quick start tip**: keep `LIBVIRT_URI` and probe timeout tuned to your environment, and add the binary to systemd for continuous status reporting.