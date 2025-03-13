# Client
* Windows/Intel Mac (specifically 2017 iMac) only
* Allow selection of alternate server via menu bar
* Different modes: local, remote
  * Local: unix socket, starts server process automatically
  * Remote: TCP, connects to server process with onboarding:
    * Detect server and its info on local network via DNS-SD
    * Save server info

# Server
* Intel Mac only (specifically 2017 iMac)
* TCP used
* Broadcasts service over DNS-SD
* stdin, stdout, and stderr are attached to the server process
* Login item created
* Configuration saved:
  * Java path
  * Server path
  * Server arguments
  * User to run server as

# Common
* Both share the same GUIs
* Specify:
    * Java path (auto-detected if possible)
    * Path to server jar
    * Arguments passed to server (`Vec<String>`), parse modes:
        * Automatic, parse like POSIX shell
        * Manual, specify each string manually
    * User to run server as
* Console display: stdout and stderr, with stdin input
* Ability to start/stop server
* Send notification when:
    * Server starts
    * Server stops
    * Server crashes

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