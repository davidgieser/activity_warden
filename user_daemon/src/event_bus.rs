use zbus::{fdo::Error as FdoError, interface};
use tokio::sync::broadcast;
use shared::types::Event;

/// Send events to the `User Daemon` to process. The daemon
/// listens for any broadcasts and updates internal state accordingly.
pub struct EventBus {
    sender: broadcast::Sender<Event>,
}

impl EventBus {
    pub fn new(sender: broadcast::Sender<Event>) -> Self {
        EventBus { sender }
    }
}

#[interface(name = "com.activity_warden.EventBus")]
impl EventBus {
    async fn send_event_msg(&self, event: Event) -> zbus::fdo::Result<usize> {
        self.sender.send(event).map_err(|e| {
            FdoError::Failed(format!("Failed to send event: {}", e))
        })
    }
}