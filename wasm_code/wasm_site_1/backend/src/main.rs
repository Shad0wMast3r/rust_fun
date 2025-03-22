use warp::Filter;
use serde::{Deserialize, Serialize};
use std::process::Command;

#[tokio::main]
async fn main() {
    // Define the route to execute a Linux command
    let execute_command_route = warp::path!("execute" / String)
        .map(|command: String| {
            let output = Command::new("sh")
                .arg("-c")
                .arg(command)
                .output();

            let response = match output {
                Ok(output) if output.status.success() => Response {
                    success: true,
                    message: String::from_utf8_lossy(&output.stdout).to_string(),
                },
                Ok(output) => Response {
                    success: false,
                    message: String::from_utf8_lossy(&output.stderr).to_string(),
                },
                Err(err) => Response {
                    success: false,
                    message: format!("Failed to execute command: {}", err),
                },
            };

            warp::reply::json(&response)
        });

    // Start the server
    println!("Server running on http://localhost:3030");
    warp::serve(execute_command_route).run(([127, 0, 0, 1], 3030)).await;
}

// Response structure for JSON
#[derive(Serialize, Deserialize)]
struct Response {
    success: bool,
    message: String,
}