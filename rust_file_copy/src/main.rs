use std::fs;
use std::io::{stdin, stdout, Write};
use std::path::Path;
use std::io::Result;
use std::time::Instant;

/// Recursively copies all files and directories from `src` to `dst`,
/// preserving the directory structure. Returns the total number of bytes copied.
fn recursive_copy(src: &Path, dst: &Path) -> Result<u64> {
    let mut total_bytes = 0;
    if src.is_dir() {
        fs::create_dir_all(dst)?;
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());
            if src_path.is_dir() {
                total_bytes += recursive_copy(&src_path, &dst_path)?;
            } else {
                let bytes = fs::copy(&src_path, &dst_path)?;
                total_bytes += bytes;
                println!("Copied file: {:?}", src_path);
            }
        }
    } else {
        let bytes = fs::copy(src, dst)?;
        println!("Copied file: {:?}", src);
        total_bytes = bytes;
    }
    Ok(total_bytes)
}

/// Copies a specific file (`src_file`) into the destination directory (`dest_dir`).
/// Returns the number of bytes copied.
fn copy_file(src_file: &Path, dest_dir: &Path) -> Result<u64> {
    if !dest_dir.exists() {
        fs::create_dir_all(dest_dir)?;
    }
    let file_name = src_file.file_name().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::Other, "File has no valid name")
    })?;
    let dest_file = dest_dir.join(file_name);
    let bytes = fs::copy(src_file, &dest_file)?;
    println!("Copied file: {:?}", src_file);
    Ok(bytes)
}

/// Prompts the user for input, flushes stdout to ensure the prompt is shown,
/// and returns the trimmed input as a String.
fn input_from_user(prompt: &str) -> String {
    let mut input = String::new();
    print!("{}", prompt);
    stdout().flush().expect("Failed to flush stdout");
    stdin().read_line(&mut input).expect("Failed to read line");
    input.trim().to_string()
}

fn main() -> Result<()> {
    loop {
        // Prompt the user for the source directory or "q" to quit.
        let src = input_from_user("Enter the source directory path (or 'q' to quit): ");
        if src.to_lowercase() == "q" {
            println!("Exiting.");
            break;
        }
        let src_path = Path::new(&src);
        if !src_path.exists() || !src_path.is_dir() {
            println!("The source '{}' is not a valid directory.", src);
            continue;
        }

        // List the contents of the source directory.
        println!("\nContents of '{}':", src);
        match fs::read_dir(src_path) {
            Ok(entries) => {
                for entry in entries.filter_map(Result::ok) {
                    let path = entry.path();
                    if path.is_dir() {
                        println!("Directory: {:?}", path);
                    } else if path.is_file() {
                        println!("File: {:?}", path);
                    }
                }
            }
            Err(e) => {
                println!("Failed to list directory contents: {}", e);
                continue;
            }
        }

        // Ask the user what they would like to copy.
        println!("\nWhat would you like to copy?");
        println!("  Enter 'f' to copy a specific file.");
        println!("  Enter 'd' to copy the entire directory recursively.");
        println!("  Enter 'q' to quit.");
        let choice = input_from_user("Your choice (f/d/q): ");
        if choice.to_lowercase() == "q" {
            println!("Exiting.");
            break;
        }

        // Record the start time for the copy operation.
        let start = Instant::now();
        let total_bytes = match choice.to_lowercase().as_str() {
            "f" => {
                // Copy a specific file.
                let filename =
                    input_from_user("Enter the specific file name (with extension) to copy: ");
                let file_path = src_path.join(&filename);
                if !file_path.exists() || !file_path.is_file() {
                    println!(
                        "The file '{:?}' does not exist or is not a valid file.",
                        file_path
                    );
                    continue;
                }
                let dest = input_from_user("Enter the destination directory path (or 'q' to quit): ");
                if dest.to_lowercase() == "q" {
                    println!("Exiting.");
                    break;
                }
                let dest_path = Path::new(&dest);
                match copy_file(&file_path, dest_path) {
                    Ok(bytes) => {
                        println!("File copied successfully.");
                        bytes
                    }
                    Err(e) => {
                        println!("Failed to copy file: {}", e);
                        continue;
                    }
                }
            }
            "d" => {
                // Copy the entire directory recursively.
                let dest = input_from_user("Enter the destination directory path (or 'q' to quit): ");
                if dest.to_lowercase() == "q" {
                    println!("Exiting.");
                    break;
                }
                let dest_path = Path::new(&dest);
                match recursive_copy(src_path, dest_path) {
                    Ok(bytes) => {
                        println!("Directory copied successfully.");
                        bytes
                    }
                    Err(e) => {
                        println!("Failed to copy directory: {}", e);
                        continue;
                    }
                }
            }
            _ => {
                println!("Invalid choice. Operation aborted.");
                continue;
            }
        };

        // Calculate the elapsed time and display transfer statistics.
        let duration = start.elapsed();
        let seconds = duration.as_secs_f64();
        if seconds > 0.0 {
            // Convert total bytes to gigabytes (1 GB = 1024^3 bytes)
            let gigabytes = total_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
            let speed_gbps = gigabytes / seconds;
            println!(
                "\nTransfer complete: {:.2} GB in {:.2} seconds ({:.2} GB/s).",
                gigabytes, seconds, speed_gbps
            );
        } else {
            println!("\nTransfer complete in less than a second ({} bytes).", total_bytes);
        }

        let cont = input_from_user("Press Enter to continue, or type 'q' to quit: ");
        if cont.to_lowercase() == "q" {
            println!("Exiting.");
            break;
        }
        println!(); // Blank line for clarity.
    }
    Ok(())
}

