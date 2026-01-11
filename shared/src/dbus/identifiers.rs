use serde::{Serialize, Deserialize};
use zvariant::Type;
use std::{fmt, str};

#[derive(Serialize, Deserialize, Type, Clone, Debug, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub enum Host {
    /// The main decision-maker using the data from the watchers.
    UserDaemon,
    /// The watcher to monitor browser-related activity.
    FirefoxWatcher,
    /// The desktop application to view the data.
    GnomeApplication,
    /// The watcher to monitor application usage across the computer.
    GnomeExtension
}

impl fmt::Display for Host {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        let to_string = match self {
            Host::UserDaemon => "user_daemon",
            Host::FirefoxWatcher => "firefox_watcher",
            Host::GnomeApplication => "gnome_application",
            Host::GnomeExtension => "gnome_extension",
        };

        write!(f,"{}", to_string)
    }
}

impl str::FromStr for Host {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "user_daemon" => Ok(Host::UserDaemon),
            "firefox_watcher" => Ok(Host::FirefoxWatcher),
            "gnome_application" => Ok(Host::GnomeApplication),
            "gnome_extension" => Ok(Host::GnomeExtension),
            _ => Err(format!("'{}' is not a valid color", s)),
        }
    }
}

#[derive(Serialize, Deserialize, Type, Clone, Debug)]
pub enum Interface {
    /// Implemented by the User Daemon to handle all events that 
    /// result from changes in the system state.
    EventBus,
    /// Implemented by any application watchers. Each interface 
    /// should expose methods as defined in the SOMEWHERE_TRAIT.
    Watcher,
    /// Implemented by the User Daemon to handle all requests
    /// to access or modify local state.
    DaemonContext,
}

impl fmt::Display for Interface {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        let to_string = match self {
            Interface::EventBus => "EventBus",
            Interface::Watcher => "Watcher",
            Interface::DaemonContext => "DaemonContext",
        };

        write!(f, "{}", to_string)
    }
}