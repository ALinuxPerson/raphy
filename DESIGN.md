# Raphy Client GUI Design Document

This document outlines the design specifications for the Raphy client graphical user interface (GUI). The design prioritizes a clean, intuitive user experience, drawing inspiration from established UI/UX principles, particularly those evident in Apple's software ecosystem.

## I. Operational Modes

The Raphy client will feature two primary operational modes:

*   **Local Mode:** Communication with the Raphy server occurs via a Unix domain socket. This mode is intended for scenarios where the client and server are running on the same machine.
*   **Remote Mode:** Communication with the Raphy server occurs over TCP/IP. This mode facilitates client-server interaction across a network and requires an initial setup process.
    *   **Prerequisite:** Successful completion of the server setup process.

## II. Server Setup Process (Remote Mode)

The setup process guides the user in establishing a connection with a remote Raphy server.

### A. Initial Server Discovery

1.  **Header:**
    *   Primary Title: "Raphy" (Bold, centered, large font size).
    *   Subtitle: "Setup Process" (Centered, medium font size).
    *   Instructional Text: "To begin, please select the appropriate server from the list below." (Centered, medium font size).

2.  **Server List:**
    *   Servers are detected using DNS-SD (Bonjour/Zero-conf).
    *   A vertically scrollable list displays available servers.
    *   Each list item will feature:
        *   A computer icon.
        *   Hostname.
        *   IP address.
    *   **Interaction:**
        *   Users click on a server entry to select it.
        *   Only one server can be selected at a time.
        *   The selected server will be visually highlighted.

3.  **Connection Status Indicator:**
    *   A designated area will display connection status messages (e.g., "Connecting to server...").

4.  **Navigation and Action Controls:**
    *   Located in the lower portion of the screen.
    *   **Controls:**
        *   Previous (`<`) Button: Disabled during the initial setup phase.
        *   Connect Button: Centered.
        *   Next (`>`) Button: Disabled during the initial setup phase.
    *   **State Logic:**
        *   **Initial State (No server selected):** `Connect` button is disabled.
        *   **Server Selected:** `Connect` button is enabled. `Previous` and `Next` buttons remain disabled.

### B. Connection Attempt

1.  **User Action:** User clicks the `Connect` button.
2.  **UI Feedback:**
    *   The `Connect` button becomes disabled.
    *   The server list becomes greyed out/inactive.
    *   The connection status indicator displays: "Connecting to server...".

### C. Connection Outcome

1.  **Successful Connection:**
    *   **Status Indicator:** Displays "Connected to server." accompanied by a checkmark icon. A subtle fade-in animation will be applied to the text and icon.
    *   **Server List:** Remains greyed out/inactive.
    *   **Controls:**
        *   `Connect` button remains disabled.
        *   `Next` (>) button becomes enabled, potentially with a soft pulsing animation to guide the user.
    *   **Configuration:** The selected server details are saved to the application's configuration file.

2.  **Unsuccessful Connection:**
    *   **Status Indicator:** Displays "Failed to connect to server." in a muted red color, accompanied by an exclamation icon.
        *   Clicking the exclamation icon reveals a dropdown or expandable section with detailed error log information.
    *   **Server List:** Becomes active, allowing the user to select a different server or re-select the same one.
    *   **Controls:** `Connect` button becomes enabled.
    *   **Optional:** A "Retry" button may appear next to the error message.

### D. No Servers Detected

1.  **Initial State:** If no servers are discovered via DNS-SD upon launching the setup screen.
2.  **UI Feedback:**
    *   A message "No servers detected." is displayed prominently.
    *   A `Refresh` button is displayed beneath the message.
    *   The `Refresh` button will exhibit a subtle spinning animation while actively scanning for servers.

### E. Completing Setup

1.  **User Action:** User clicks the enabled `Next` (>) button after a successful connection.
2.  **UI Feedback:** A subtle screen transition animation plays.
3.  **Outcome:** The setup process is considered complete, and the user is navigated to the main client management interface.

## III. Main Client Management Interface

The main interface provides tools for managing and monitoring the connected Raphy server. The design will adhere to Apple's Human Interface Guidelines for spacing, visual hierarchy, and component styling. A light/dark mode toggle, accessible via application settings, will be considered.

### A. Header

*   **Title:** "Raphy Client Management Interface" (San Francisco font or system default, bold, centered, standard capitalization).

### B. Server Status Indicator

*   **Display:** A pill-shaped indicator clearly displays the server's current operational status.
*   **Visual Cues:**
    *   **Color:** Green for "Running", Red for "Stopped", Yellow for "Restarting" (or similar states).
    *   **Icon:** A small, relevant icon alongside the status text (e.g., play icon for "Running").
*   **Text:** Descriptive status (e.g., "Running", "Stopped", "Restarting").

### C. Configuration Section

This section will be presented within a card-like container featuring a subtle shadow and rounded corners.

1.  **Java Path:**
    *   **Input Field:** Clean text field, utilizing a monospaced font for path display.
    *   **Placeholder:** If the field is blank, an attempt will be made to auto-detect the Java path. If successful, the detected path is shown as light gray placeholder text.
    *   **Not Detected:** If auto-detection fails, "Java path not detected." is displayed, possibly with a small warning icon.
    *   **Browse Button:** A "Browse..." button will open a native file system picker to select the Java executable.

2.  **Server Path (Raphy Server JAR/Executable):**
    *   **Input Field:** Similar text field styling to Java Path.
    *   **Browse Button:** A "Browse..." button for selecting the server application file.
    *   **Validation:** The field will visually indicate validation status (e.g., green outline for a valid path/file, red for an invalid one).

3.  **Server Arguments:**
    *   **Mode Selection:** A segmented control allows switching between:
        *   `Parsed` Mode: (Details TBD - potentially a more structured way to input common arguments).
        *   `Manual` Mode: A text area or list of text fields for entering raw command-line arguments.
    *   **Manual Mode Input:** If using a list of text fields, a "+" button allows adding new argument fields dynamically, arranged vertically with proper alignment.

4.  **User Dropdown (for Server Process):**
    *   **Control:** A native macOS/iOS-style dropdown menu.
    *   **Options:** Lists available system users.
    *   **"Other..." Option:** Selecting "Other..." opens a sheet-style dialog (modal, non-blocking if possible, or a standard modal dialog) for manual username entry.

### D. Configuration Management

1.  **Save Button:**
    *   **Style:** Adheres to Apple's button styling.
    *   **State:**
        *   **Disabled (Greyed Out):** When no unsaved changes are present in the configuration section.
        *   **Enabled (Highlighted Blue):** When unsaved changes are detected.
    *   **Interaction:** Upon clicking:
        *   Briefly displays a spinner animation.
        *   Upon successful save, shows a checkmark animation.

2.  **Reset Button:**
    *   **Visibility:** Appears next to the `Save` button only when unsaved changes exist.
    *   **Action:** Reverts configuration fields to their last saved state.

### E. Console Access

*   **Console Button:** A small terminal icon, typically located in a corner of the interface.
*   **Interaction:** Clicking the button expands a section of the view (with a smooth animation) to reveal the server console log.
*   **Console View:**
    *   **Font:** Monospaced font for readability.
    *   **Syntax Highlighting:** Differentiates message types (e.g., ERROR, WARN, INFO) with distinct colors or styles.

### F. Server Control Buttons

Located in a dedicated bottom control section, each button will be clearly labeled and accompanied by an icon.

*   **Start Button:** (Play icon) Initiates the server process.
*   **Stop Button:** (Square icon) Terminates the server process.
*   **Restart Button:** (Circular arrow icon) Stops and then starts the server process.

*   **Button States & Feedback:**
    *   Appropriate hover states.
    *   Subtle click animations.
    *   Buttons are disabled when the corresponding action is not applicable (e.g., `Stop` button is disabled if the server is not running).
    *   Explanatory tooltips appear on hover, especially for disabled buttons, clarifying why the action is unavailable.

### G. Application Settings

*   **Access:** A discrete "Settings" gear icon, typically located in the top-right corner of the window or a standard application menu.
*   **Content:** Application-specific preferences (e.g., theme selection (light/dark), update settings).

### H. Connection Management

*   **Connection Loss:** Implement a reconnection mechanism.
*   **Notification:** If the connection to the server is lost, display a non-intrusive notification.
*   **Automatic Reconnection:** The client should attempt to reconnect automatically in the background for a defined period or number of attempts.
