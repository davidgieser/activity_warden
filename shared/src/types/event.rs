use serde::{Deserialize, Serialize};
use zvariant::Type;

use crate::dbus::Host;

#[derive(Serialize, Deserialize, Type, Clone, Debug)]
pub enum EventType {
    FocusChange,
    FocusLost,
    AFK,
}

#[derive(Serialize, Deserialize, Type, Clone, Debug)]
pub struct Event {
    pub event_type: EventType,
    pub source: Host,
    /// The name for which the event will be stored and displayed.
    pub display_name: String,
    /// This field is optionally used to store additional information
    /// for a given `EventType`. Algebraic types will not work due to
    /// the `Type` macro constraint.
    pub metadata: String,
}
