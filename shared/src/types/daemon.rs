use serde::{Deserialize, Serialize};
use zvariant::Type;

use crate::types::schema::Timer;
use crate::dbus::Host;
use std::collections::HashMap;

type DisplayName = String;
pub type DurationMap = HashMap<Host, HashMap<DisplayName, u32>>;

#[derive(Serialize, Deserialize, Type)]
pub struct DaemonSnapshot {
    pub timers: Vec<Timer>,
    pub durations: DurationMap,
}
