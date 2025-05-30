# Raphy: Remote Server Management Utility

Raphy is a client-server application designed for managing and interacting with a server process, typically a Java application, across different modes of operation.

## Architecture Overview

The system consists of a client application, a daemon process, and the target server process. Communication can occur locally via Unix sockets or remotely via TCP/IP.

```
┌───────────────┐     Local        ┌──────────────┐      ┌────────────────┐
│ Application   │◄──(Unix Socket)─►│    Daemon    │◄────►│ Server Process │
│ (Local)       │                  │              │      │                │
└───────────────┘                  └──────────────┘      └────────────────┘
                                     ▲
                                     │
                                     │ TCP
                                     ▼
┌───────────────┐     Network     ┌──────────────┐
│ Application   │◄───(TCP/IP)────►│  Daemon      │
│ (Remote)      │                 │ (Other host) │
└───────────────┘                 └──────────────┘
```

## Client

*   **Platform Support:** Windows and Intel-based Macs (tested on 2017 iMac).
*   **Server Selection:** Allows choosing an alternate server through the menu bar.
*   **Operating Modes:**
    *   **Local Mode:**
        *   Communicates with the server daemon via a Unix socket.
        *   Automatically starts the server daemon process.
    *   **Remote Mode:**
        *   Communicates with the server daemon via TCP.
        *   Facilitates onboarding for connecting to a server process:
            *   Detects server and its information on the local network using DNS-SD.
            *   Saves server connection information.

## Server Daemon

*   **Platform Support:** Intel-based Macs (tested on 2017 iMac).
*   **Communication Protocol:** Uses TCP for network communication.
*   **Service Discovery:** Broadcasts its service on the local network via DNS-SD.
*   **Process I/O:** Standard input (stdin), standard output (stdout), and standard error (stderr) of the target server process are attached to the daemon.
*   **Startup:** Configured as a login item for automatic startup.
*   **Persistent Configuration:** Saves the following settings:
    *   Java executable path.
    *   Path to the server application (e.g., JAR file).
    *   Arguments for the server application.
    *   User account under which the server process will run.

## Common Components & Features

*   **Shared GUI:** Both client and server configuration interfaces share common GUI elements.
*   **Configuration Options:**
    *   **Java Path:** Path to the Java executable (auto-detection attempted).
    *   **Server Path:** Path to the server application (e.g., JAR file).
    *   **Server Arguments:** Arguments passed to the server process (`Vec<String>`).
        *   **Automatic Parsing:** Parses arguments similar to a POSIX shell.
        *   **Manual Specification:** Allows each argument string to be specified individually.
    *   **Run-As User:** Specifies the user account for running the server process.
*   **Console Interface:**
    *   Displays stdout and stderr from the server process.
    *   Provides an input field for stdin to the server process.
*   **Server Control:** Ability to start and stop the server process.
*   **Notifications:** Sends system notifications for the following events:
    *   Server started.
    *   Server stopped.
    *   Server crashed.
## Building the Project

This section outlines the steps to build the Raphy server and client applications.

### Prerequisites

*   **Rust:** Ensure you have a recent version of Rust installed. You can get it from [rust-lang.org](https://www.rust-lang.org/tools/install).
*   **Node.js and pnpm:** For building the Tauri client application.
    *   Install Node.js (which includes npm) from [nodejs.org](https://nodejs.org/).
    *   Install pnpm globally: `npm install -g pnpm`
*   **Tauri CLI:** Install the Tauri CLI tools: `cargo install tauri-cli --version "^1.0"` (or the version specified in `client-app/src-tauri/Cargo.toml`).
*   **System Dependencies for Tauri:** Follow the Tauri guide for your specific OS to install necessary dependencies (e.g., webview development libraries): [Tauri Prerequisites](https://tauri.app/v1/guides/getting-started/prerequisites).

### Building the Server Daemon

1.  Navigate to the server directory:
    ```bash
    cd server
    ```
2.  Build the server:
    ```bash
    cargo build --release
    ```
    The executable will be located in `server/target/release/`.

### Building the Client Application (Tauri)

1.  Navigate to the client application directory:
    ```bash
    cd client-app
    ```
2.  Install Node.js dependencies:
    ```bash
    pnpm install
    ```
3.  Build the Tauri application:
    ```bash
    pnpm tauri build
    ```
    The application bundle will be located in `client-app/src-tauri/target/release/bundle/`. The exact path and format (e.g., `.app`, `.dmg`, `.msi`, `.AppImage`) will depend on your operating system.

### Building Common Components

The `common` and `protocol` crates are dependencies of the server and client. They will be built automatically when you build the server or client. If you need to build them independently for testing or development:

1.  Navigate to the respective directory (e.g., `cd common`).
2.  Build the crate:
    ```bash
    cargo build
    ```
