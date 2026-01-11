use zbus::interface;
use serde_json::json;
use crate::messaging::write_message;
use crate::types::MessageType;

pub struct FirefoxWatcher;
impl FirefoxWatcher {
    pub fn new() -> Self { FirefoxWatcher {} }
}

#[interface(name = "com.activity_warden.Watcher")]
impl FirefoxWatcher {
    async fn request_close(&self, metadata: String) -> zbus::fdo::Result<()> {
        // Construct the metadata required to close a tab on timeout.
        let msg = json!({
            "type": MessageType::Close,
            "tab_id": metadata
        });
        write_message(&msg).unwrap();

        Ok(())
    }
}