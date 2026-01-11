use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use std::fmt;
use zvariant::Type;

use crate::dbus::Host;

#[derive(Type, Serialize, Deserialize, Debug, Clone)]
pub struct Timer {
    /// The name of the website or application as read by the user.
    pub display_name: String,
    /// The host, or `Watcher`, that oversees this `display_name`.
    pub host: Host,
    /// The number of minutes allowed until the `display_name` will be blocked.
    pub time_limit: u32,
    /// A boolean array corresponding to the 7 days of the week.
    /// A `true` implies that a the limit will be enforced on the given day.
    /// A `false` implies that no time will be allowed on the given day.
    pub allowed_days: Vec<bool>,
}   

#[derive(Debug, Type, Serialize, Deserialize, Clone)]
pub struct FocusChange {
    /// The host from where the focus change originated.
    pub host: Host,
    /// The display name passed by the `Watcher`.
    pub display_name: String,
    /// The timestamp of the event.
    pub timestamp: DateTime<Utc>,
    /// The length of the session on the given `display_name`.
    pub duration: u32,
}

pub type Password = String;

#[derive(Type, Serialize, Deserialize)]
pub enum AWTables {
    /// Stores the user-inputted timers.
    Timers,
    /// Stores changes to the currently focused window.
    FocusChanges,
}

impl fmt::Display for AWTables {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let enum_str = match self {
            AWTables::FocusChanges => "focus_changes",
            AWTables::Timers => "timers",
        };

        f.write_str(enum_str)
    }
}

#[derive(Type, Serialize, Deserialize)]
pub enum QueryType {
    SELECT,
    CREATE,
    INSERT,
    DELETE,
    UPDATE
}

impl fmt::Display for QueryType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let enum_str = match self {
            QueryType::SELECT => "select",
            QueryType::UPDATE => "update",
            QueryType::DELETE => "delete",
            QueryType::INSERT => "insert",
            QueryType::CREATE => "create",
        };

        f.write_str(enum_str)
    }
}