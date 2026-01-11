# User Daemon:

The user daemon serves as the central processing hub for all actions in the system. All core business logic stays within the user daemon.

To set the user daemon up, create a systemd service (i.e. `activity_warden.service`) located at an accepted systemd location like `~/.config/systemd/user`.

An example configuration file is provided below:

```
[Unit]
Description=Activity Warden Daemon

[Service]
Type=simple
Environment="RUST_LOG=debug"
ExecStart=/home/davidgieser/Coding/activity_warden/user_daemon/target/release/user_daemon
Restart=on-failure

[Install]
WantedBy=default.target
```