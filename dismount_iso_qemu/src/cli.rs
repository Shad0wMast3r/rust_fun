use std::sync::Arc;
use crate::probe::ProbeManager;
use std::io::{self, Write};

pub fn run(probe_mgr: Arc<ProbeManager>) -> anyhow::Result<()> {
    loop {
        println!("1) Mount ISO");
        println!("2) Scan mounted ISOs");
        println!("3) Exit");
        print!("Select option: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        match input.trim() {
            "1" => {
                println!("Mount ISO not implemented in this snippet.");
            }
            "2" => {
                match crate::virsh::list_vms() {
                    Ok(vms) => {
                        if vms.is_empty() {
                            println!("No VMs found (virsh returned no names).");
                        } else {
                            println!("{:20} {}", "VM", "OS");
                            for vm in vms {
                                match probe_mgr.get_os(&vm) {
                                    Ok(Some(os)) => println!("{:20} {}", vm, os),
                                    Ok(None) => println!("{:20} (unknown)", vm),
                                    Err(e) => println!("{:20} error: {}", vm, e),
                                }
                            }
                        }
                    }
                    Err(e) => {
                        println!("Failed to list VMs via virsh: {}", e);
                    }
                }
            }
            "3" => break,
            _ => println!("Unknown option"),
        }
    }
    Ok(())
}
