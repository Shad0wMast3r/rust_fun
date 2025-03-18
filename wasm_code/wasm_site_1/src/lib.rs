use wasm_bindgen::prelude::*;
use chrono::Local; // For working with dates and times (add `chrono` to dependencies)

// Expose the function to JavaScript
#[wasm_bindgen]
pub fn process_input_with_time(user_input: &str) -> String {
    // Get the current date and time
    let current_time = Local::now();
    let formatted_time = current_time.format("%Y-%m-%d %H:%M:%S").to_string();

    // Return the user's input along with the date and time
    format!("You entered: {}\nCurrent date and time: {}", user_input, formatted_time)
}
