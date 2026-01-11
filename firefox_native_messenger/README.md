# Firefox Native Messenger

The sole purpose of this application is to shuttle messages from the Firefox browser extension to the User Daemon. All messages are translated from the Firefox native messenging protocol into a DBus format that is accepted by the User Daemon.

The Firefox extension specifies that the binary that is used is the release version of this project. To update this code, simply run `cargo build -r`.