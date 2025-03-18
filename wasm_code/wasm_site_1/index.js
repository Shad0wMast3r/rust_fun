import init, { process_input_with_time } from './pkg/wasm_site_1.js';

async function main() {
    await init(); // Initialize the Wasm module

    // Real-time clock
    function updateTime() {
        const now = new Date();
        const hours = String(now.getHours()).padStart(2, '0');
        const minutes = String(now.getMinutes()).padStart(2, '0');
        const seconds = String(now.getSeconds()).padStart(2, '0');
        const currentTime = `${hours}:${minutes}:${seconds}`;
        document.getElementById("time-display").textContent = `Current Time: ${currentTime}`;
    }

    // Update the clock every second
    setInterval(updateTime, 1000);

    // User input handling
    document.getElementById("submit-btn").addEventListener("click", () => {
        const userInput = document.getElementById("user-input").value; // Get user input
        // Directly display the user input
        document.getElementById("output").textContent = `You entered: ${userInput}`;
    });

    // Initialize time immediately on page load
    updateTime();
}

main();
