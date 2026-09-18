mod reddit;
mod youtube;

pub use reddit::RedditBackstop;
pub use youtube::YoutubeBackstop;

use std::collections::VecDeque;
use serde_json::Value;

use chrono::{TimeDelta, Utc};
use shared::types::Event;
use shared::types::schema::FocusChange;

use crate::DisplayNameAction;
use crate::context::LastEvent;

/// A `BackStop` is something that acts as an accountability layer to any timers
/// put in place through the GUI. These backstops are less strict than timers.
/// However, the goal is to act as a further deterrent in the case that a
/// necessary timer has been disabled.
pub trait BackStop {
    fn new() -> Self
    where
        Self: Sized;

    fn trigger_backstop(
        &mut self, 
        le: &Option<&LastEvent>,
        e: &Event,
        fc: &Option<FocusChange>,
    ) -> Option<DisplayNameAction>;

    // / Ingest the `FocusChange` for the previous event. Use the last event
    // / to maintain the `Event` metadata for precise URL info.
    // fn process_focus_change(&mut self, le: &LastEvent, fc: &Option<FocusChange>);

//     /// Return a string to be displayed through desktop notifications.
//     /// If None is returned, no warning is presented.
//     fn warn(&mut self, e: &Event) -> Option<String>;

//     /// Should this return a duration? How do I track how much time has been spent?
//     /// Do I have to rewrap some of the context logic?
//     /// How can I both send a warning while also start counting down for blocking?
//     /// If None is returned, do not start a timer. Otherwise, set the timer for the
//     /// passed duration. For an instant block, simply return Some(0)...
//     fn block(&mut self, e: &Event) -> Option<usize>;
}


pub struct FireFoxBackStopContext {
    lookback: VecDeque<FocusChange>,
    lookback_window_size: TimeDelta,
}

impl FireFoxBackStopContext {
    fn new(lookback_seconds: i64) -> Self {
        Self {
            lookback: VecDeque::new(),
            lookback_window_size: TimeDelta::new(lookback_seconds, 0).unwrap(),
        }
    }

    fn prune_lookback(&mut self) {
        let cutoff = Utc::now() - self.lookback_window_size;

        while let Some(fc) = self.lookback.front_mut() {
            let duration = TimeDelta::seconds(fc.duration as i64);
            let end = fc.timestamp + duration;

            if end <= cutoff {
                self.lookback.pop_front();
                continue;
            }

            if fc.timestamp < cutoff {
                fc.timestamp = cutoff;
                fc.duration = (end - cutoff).num_seconds().max(0) as u32;
            }

            break;
        }
    }

    fn process_focus_change(&mut self, fc: FocusChange) {
        self.prune_lookback();

        self.lookback.push_back(fc);
    }

    fn get_total_seconds_in_window(&mut self) -> u32 {
        self.prune_lookback();

        self.lookback
            .iter()
            .map(|fc| fc.duration)
            .sum()
    }
}


fn get_event_url(e: &Event) -> String {
    let metadata: Value = serde_json::from_str(&e.metadata)
        .expect("Invalid metadata structure");

    return metadata.get("url").unwrap().as_str().unwrap().to_string();
}

