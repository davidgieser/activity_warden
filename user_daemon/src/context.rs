use std::collections::HashMap;
use std::sync::Arc;
use zbus::{Result, object_server::SignalEmitter, interface};
use arc_swap::ArcSwap;
use chrono::{DateTime, NaiveDate, Local, Utc};
use sha2::{Sha256, Digest};

use shared::types::schema::{FocusChange, QueryType, Timer};
use shared::types::daemon::{DurationMap, DaemonSnapshot};
use shared::types::Event;
use shared::dbus::Host;
use crate::persistence::PersistenceLayer;
use crate::EventType;
use log::{info, debug};

#[derive(Clone)]
pub struct LastEvent {
    pub time: DateTime<Utc>,
    pub event: Event
}

/// The `DaemonContext` maintains the state of the `User Daemon`.
/// Furthermore, it exposes a DBus interface for external
/// components to modify internal state. All such changes maintain
/// consistency with the SQLite database.
#[derive(Clone)]
pub struct DaemonContext {
    /// The wrapper around the SQLite DB operations.
    pl: PersistenceLayer,
    /// The vector of timers established by the user.
    /// Writes are infrequent, so an `ArcSwap` is used for concurrency.
    pub timers: Arc<ArcSwap<Vec<Timer>>>,
    /// A mapping of the current durations accumulated over the day.
    pub timer_durations: DurationMap,
    pub last_event: HashMap<Host, LastEvent>,
    /// The current date for which the active state corresponds.
    /// This value is necessary to determine when state should be wiped
    /// on a new day.
    cur_date: NaiveDate,
}

impl DaemonContext {
    pub fn new() -> Self {
        let pl = PersistenceLayer::new();
        let timers = pl.select_timers();
        let durations = pl.select_current_durations();

        let today = Utc::now().date_naive();
        Self {
            pl: pl.clone(),
            timers: Arc::new(ArcSwap::from_pointee(timers)),
            timer_durations: durations,
            last_event: HashMap::new(),
            cur_date: today
        }
    }

    /// At the dawn of a new day, reset the internally stored durations.
    /// To avoid the case where an event might span a day boundary, 
    /// utilize the alarm system to wake up the timer at the midnight boundary.
    pub fn reset_daily_state(&mut self) {
        let check_date = Local::now().date_naive();
        if self.cur_date != check_date {
            info!("[RESET] Resetting internal state of the daemon context.");
            self.timer_durations.clear();
            self.last_event.clear();
            self.cur_date = check_date;
        }
    }


    /// Clear all stored events in particular scenarios.
    /// For example, if the computer shuts down, stop tracking any state.
    pub fn clear_last_event(&mut self) {
        for (host, last_event) in self.last_event.clone().into_iter() {
            let event = Event {
                event_type: EventType::FocusLost,
                source: host,
                display_name: last_event.event.display_name,
                metadata: last_event.event.metadata,
            };
            self.update_event_durations(&event, false);
        }
    }


    /// Update the corresponding durations for a given event.
    pub fn update_event_durations(&mut self, event: &Event, set_last_event: bool) -> Option<FocusChange> {
        let now = Utc::now();
        let mut focus_change = None;
        if let Some(last_event) = self.last_event.get(&event.source) {
            let focus_change_evt = FocusChange {
                host: last_event.event.source.clone(),
                display_name: last_event.event.display_name.clone(),
                timestamp: now,
                duration: (now - last_event.time).num_seconds() as u32,
            };

            self.pl.insert_focus_change(&focus_change_evt);

            let host_map = self.timer_durations.entry(last_event.event.source.clone()).or_default();
            let cur_duration = host_map.entry(last_event.event.display_name.clone()).or_default();
            *cur_duration += focus_change_evt.duration;
            
            debug!("[EVENT]: {} seconds of activity on '{}' to total {} seconds.", focus_change_evt.duration, last_event.event.display_name, cur_duration);

            focus_change = Some(focus_change_evt);
        }

        if set_last_event {
            let last_event = LastEvent {
                time: now,
                event: event.clone(),
            };

            debug!("[EVENT]: Last event from {} at {}.", event.display_name, now.format("%Y-%m-%d %H:%M:%S"));
            self.last_event.insert(event.source.clone(), last_event);
        } else {
            let _ = self.last_event.remove(&event.source);
        }

        focus_change
    }
}

#[interface(name = "com.activity_warden.DaemonContext")]
impl DaemonContext {
    #[zbus(signal)]
    async fn duration_changed(signal_emitter: &SignalEmitter<'_>, change: FocusChange) -> Result<()>;

    pub fn get_daemon_snapshot(&self) -> DaemonSnapshot {
        DaemonSnapshot { 
            timers: (*self.timers.load_full()).clone(),
            durations: self.pl.select_current_durations(),
        }
    }

    pub fn is_locked(&self) -> bool {
        self.pl.get_cur_password().is_some()
    }

    /// This function assumes that any password submission is
    /// intended to either lock or unlock the application.
    /// First, check if the password matches the existing password.
    /// If there is no existing password, set the new password.
    /// 
    /// False indicates that the application should be locked.
    /// True indicates that the application should be unlocked.
    pub fn process_password_submission(&self, password: String) -> bool {
        if let Some(cur_hash) = &self.pl.get_cur_password() {
            let mut hasher = Sha256::new();
            hasher.update(password);
        
            let digest = hasher.finalize();
            let password_hash = hex::encode(digest);
            let is_correct = *cur_hash == password_hash;
            if is_correct {
                self.pl.remove_password();
            }

            return is_correct;
        } else {
            self.pl.set_new_password(password);
            return false;
        }
    }

    pub fn insert_timer(&self, timer: Timer) {
        self.timers.rcu(|old| {
            let mut old_timers = (**old).clone();
            old_timers.push(timer.clone());

            old_timers
        });
        self.pl.modify_timer(QueryType::INSERT, timer);
    }

    pub fn delete_timer(&self, timer: Timer) {
        self.timers.rcu(|old| {
            let old_timers = (**old).clone();
            old_timers.into_iter()
                .filter(|t| {
                    t.display_name != timer.display_name || t.host != timer.host
                })
                .collect::<Vec<Timer>>()
        });
        self.pl.modify_timer(QueryType::DELETE, timer);
    }

    pub fn update_timer(&self, timer: Timer) {
        self.timers.rcu(|old| {
            let old_timers = (**old).clone();
            old_timers.into_iter()
                .map(|t| {
                    if t.display_name == timer.display_name && t.host == timer.host {
                        return timer.clone();
                    } else {
                        return t;
                    }
                })
                .collect::<Vec<Timer>>()
        });

        self.pl.modify_timer(QueryType::UPDATE, timer);
    }
}