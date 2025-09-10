// SPDX-License-Identifier: MIT
// Author: Andy Kukuc (Creator)
// Contributor: Microsoft Copilot

use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::Command;
use std::env;

/// Struct to hold parsed Speedtest metrics
#[derive(Debug)]
struct SpeedtestMetrics {
    down_local: f64,
    up_local: f64,
}

/// Parse raw output from Speedtest shell script
fn parse_speedtest_output(output_path: &str) -> Option<SpeedtestMetrics> {
    let file = File::open(output_path).ok()?;
    let reader = BufReader::new(file);

    let mut down = None;
    let mut up = None;

    for line in reader.lines().flatten() {
        if line.contains("Local Download") {
            down = line.split_whitespace()
                       .filter_map(|s| s.parse::<f64>().ok())
                       .next();
        } else if line.contains("Local Upload") {
            up = line.split_whitespace()
                     .filter_map(|s| s.parse::<f64>().ok())
                     .next();
        }
    }

    Some(SpeedtestMetrics {
        down_local: down?,
        up_local: up?,
    })
}

/// Update LibreNMS RRD files with parsed metrics
fn update_rrd(rrd_dir: &str, metrics: &SpeedtestMetrics) {
    let down_rrd = format!("{}/down_local.rrd", rrd_dir);
    let up_rrd = format!("{}/up_local.rrd", rrd_dir);

    for (path, value) in &[(down_rrd, metrics.down_local), (up_rrd, metrics.up_local)] {
        if Path::new(path).exists() {
            let _ = Command::new("rrdtool")
                .args(["update", path, &format!("N:{}", value)])
                .status();
        }
    }
}

/// Get the system hostname
fn get_hostname() -> Option<String> {
    // On Unix-like systems, use the 'hostname' command
    let output = Command::new("hostname").output().ok()?;
    let hostname = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if hostname.is_empty() {
        None
    } else {
        Some(hostname)
    }
}

/// Find the app-speedtest-* directory for the current hostname
fn find_app_speedtest_dir(rrd_base: &str) -> Option<String> {
    let entries = fs::read_dir(rrd_base).ok()?;
    for entry in entries.flatten() {
        let file_name = entry.file_name();
        let file_name_str = file_name.to_string_lossy();
        if file_name_str.starts_with("app-speedtest-") {
            return Some(file_name_str.into_owned());
        }
    }
    None
}

fn main() {
    let output_path = "/opt/librenms/scripts/speedtest_output.txt";

    let hostname = match get_hostname() {
        Some(h) => h,
        None => {
            eprintln!("Could not determine hostname.");
            return;
        }
    };

    let rrd_base = format!("/data/rrd/{}", hostname);

    let app_name = match find_app_speedtest_dir(&rrd_base) {
        Some(name) => name,
        None => {
            eprintln!("Could not find app-speedtest-* directory.");
            return;
        }
    };

    let rrd_path = format!("{}/{}", rrd_base, app_name);

    if let Some(metrics) = parse_speedtest_output(output_path) {
        println!("Parsed metrics: {:?}", metrics);
        update_rrd(&rrd_path, &metrics);
    } else {
        eprintln!("Failed to parse Speedtest output.");
    }
}