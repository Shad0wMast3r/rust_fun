# Andy Kukuc - Rust Developer

Hello! I'm Andy, a software developer with a passion for creating efficient, reliable, and practical tools using Rust. I enjoy building command-line utilities and backend services that solve real-world problems and automate complex tasks.

Below are a few of my projects that showcase my interest in systems programming and automation.

---

## Featured Projects

### 1. LibreNMS Speedtest RRD Updater (`app_creator_librenms`)

A specialized Rust program designed to integrate with the LibreNMS monitoring platform. It parses the output from a Speedtest script and updates the corresponding RRD (Round-Robin Database) files. The tool is built for flexibility, automatically detecting the system hostname and locating the correct LibreNMS application directory to ensure it works across different environments without hardcoded paths.

-   **Key Features:** Dynamic hostname detection, RRD file updates via `rrdtool`, robust parsing.
-   **Technologies:** Rust

### 2. Command Executor with Web UI (`cmd_exec`)

This project combines a Rust backend with a simple web frontend to execute Linux commands remotely. A key feature is its `build_and_package.sh` script, which compiles the Rust code into a fully static, self-contained binary. This allows the entire application (backend and frontend) to be packaged into a single archive and deployed on any Linux server, even those without internet access or the Rust toolchain.

-   **Key Features:** Static binary compilation for portability, web-based command execution, dependency-free deployment.
-   **Technologies:** Rust, Warp, `musl`, Shell Scripting.

### 3. Rust Email Cleanup Utility (`rust_email_cleanup`)

A command-line utility for decluttering a Yahoo Mail account. It connects securely to the IMAP server, lists all mail folders, and allows the user to interactively select a folder for cleanup. It then proceeds to delete emails older than a specified duration (e.g., 30 days). Credentials are managed securely using a `.env` file to keep them out of the source code.

-   **Key Features:** Secure IMAP connection, interactive folder selection, date-based email deletion.
-   **Technologies:** Rust, `imap`, `native-tls`, `dotenv`, `chrono`.

---

My focus is on building robust, maintainable, and performant software. I'm always exploring new ways to leverage Rust's powerful features to create high-quality applications.
