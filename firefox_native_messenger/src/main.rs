mod messaging;
mod types;
mod watcher;

use zbus::connection::Builder;
use zbus::{Result, proxy};
use shared::types::{Event, EventType};
use shared::dbus::{Host, Interface, DBus};
use serde_json::json;

use crate::messaging::{write_message, read_message};
use crate::types::MessageType;
use crate::watcher::FirefoxWatcher;

#[proxy(interface = "com.activity_warden.EventBus")]
trait EventBus {
    async fn send_event_msg(&self, event: Event) -> Result<usize>;
}

#[tokio::main]
async fn main() -> Result<()> {
    // Serve the FirefoxWatcher on the DBus daemon.
    let watcher = FirefoxWatcher::new();
    let conn = Builder::session()?
        .name(DBus::host_name(&Host::FirefoxWatcher))?
        .serve_at(DBus::object_path(&Host::FirefoxWatcher, &Interface::Watcher), watcher)?
        .build()
        .await?;

    // Open up a connection to the EventBus to transmit messages to the User Daemon.
    let proxy = EventBusProxy::builder(&conn)
        .destination(DBus::host_name(&Host::UserDaemon))?
        .path(DBus::object_path(&Host::UserDaemon, &Interface::EventBus))?
        .build()
        .await?;

    // Listen to messages from the extension.
    while let Some(input) = read_message() {
        let reply = json!({
            "type": MessageType::ACK,
        });

        if let Err(_e) = write_message(&reply) {
            break;
        }

        let event_type = input.get("event_type").unwrap().as_str().unwrap();
        let event = match event_type {
            "focus_change" => {
                let tab_id = input.get("tab_id").unwrap();
                let display_name = input.get("display_name").unwrap();

                Event {
                    event_type: EventType::FocusChange,
                    source: Host::FirefoxWatcher,
                    display_name: display_name.as_str().unwrap().to_string(),
                    metadata: tab_id.to_string(),
                }
            },
            "focus_lost" => {
                Event {
                    event_type: EventType::FocusLost,
                    source: Host::FirefoxWatcher,
                    display_name: "".to_string(),
                    metadata: "".to_string(),
                }
            },
            _ => {
                panic!("Unexpected event type: {}", event_type);
            }
        };

        let _ = proxy.send_event_msg(event).await;
    }

    Ok(())
}
