# Client GUI Design
* Two modes: local, remote
    * Local: via unix socket
    * Remote: via TCP/IP
        * Requires setup process first
* Setup process:
    * User needs to select the appropriate server detected from DNS-SD
    * Description:

the general layout of our user interface is inspired by apple.

layout is grouped horizontally. step-by-step process. setup process is reminiscent of Apple's setup process.

header text, bold, centered, large font size: "Raphy"
subheader text, centered, medium font size: "Setup Process"
subheader text, centered, medium font size: "To begin, please select the appropriate server from the list below."

a list of connected servers are shown. to select one, the user needs to click on it. only one server can be selected. each server has:
* computer icon,
* hostname,
* ip address.

there is a space saved for "Connecting to server..." text. this text is shown when the user clicks on a server then clicks on "Connect". the server is highlighted, and the text is shown. the list of servers is then greyed out.

in the lower portion, there are two arrows `<` and `>`. there is a "Connect" button in the center. state:
* if a server is not selected yet: both arrows and "Connect" button are disabled.
* if a server is selected: both arrows are still disabled, but "Connect" button is enabled.

when the user clicks on "Connect", the server is connected to. the "Connecting to server..." text is shown. the list of servers is greyed out. the "Connect" button is disabled.

when the connection is successful, the "Connecting to server..." text is replaced with "Connected to server." with a subtle checkmark icon beside it. The text should briefly animate with a subtle fade transition. The list of servers remains greyed out. The "Connect" button is disabled. Right arrow becomes enabled with a soft pulsing animation to draw attention. The server is saved in the configuration file.

when the connection is unsuccessful, the "Connecting to server..." text is replaced with "Failed to connect to server." in a muted red color. An exclamation icon appears next to it which when clicked reveals a dropdown with the detailed log info. The list of servers becomes active again. The "Connect" button becomes enabled. Consider adding a "Retry" option that appears next to the error message.

If no servers are found initially, show a helpful message: "No servers detected" with a "Refresh" button beneath it. The refresh button should have a subtle spinning animation when refreshing the server list.

when the user clicks on the right arrow, a subtle transition animation plays and the setup process is completed. The user is taken to the main screen.

main screen:

Layout uses Apple's standard spacing and grouping with clear visual hierarchy. Consider a light/dark mode toggle in settings to follow Apple's design language.

Header text uses San Francisco font (Apple's system font), bold, centered, with proper capitalization: "Raphy Client Management Interface"

Server status is displayed as a pill-shaped indicator with appropriate color (green if started, red if stopped, yellow if restarting) and includes a small icon alongside text (e.g., "Running", "Stopped", "Restarting").

Configuration section is contained in a card-like container with subtle shadow and rounded corners:

Java path: Clean text field with monospaced font for paths. If blank, auto-detect Java and show the path in light gray as placeholder text. If not detected, show "Java path not detected." with a small warning icon. Add a "Browse..." button that opens a native file picker.

Server path: Similar text field with "Browse..." button. Shows validation state (green outline when valid, red when invalid).

Server arguments: Text field with segmented control above to switch between `Parsed` and `Manual` modes. For Manual mode, provide a "+" button to add new argument fields in a vertical list with proper alignment.

User dropdown: Uses native macOS/iOS-style dropdown with smooth animation. "Other..." option opens a sheet-style dialog (not a popup) for username entry.

All changes are tracked, and the "Save" button follows Apple's style - disabled (greyed out) when no changes made, becomes highlighted blue when changes detected. When clicked, it briefly shows a spinner and then a checkmark animation.

Add a "Reset" button next to "Save" that only appears when unsaved changes exist.

Console button is a small terminal icon in the corner that expands the view with a smooth animation to reveal the console log. The console has a monospaced font with syntax highlighting for different message types (error, warning, info).

Bottom control section has three clearly labeled buttons with icons:
- Start (play icon)
- Stop (square icon)
- Restart (circular arrow icon)

These buttons have appropriate hover states and subtle click animations. They're disabled with explanatory tooltips when operations aren't available (e.g., can't stop a server that isn't running).

Include a discrete "Settings" gear icon in the top-right corner for application preferences.

For handling connection loss, add a reconnection mechanism that shows a non-intrusive notification and attempts to reconnect automatically.
