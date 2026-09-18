use crate::{
    backstop::{
        BackStop, 
        FireFoxBackStopContext,
        get_event_url
    }, 
    context::LastEvent,
};
use crate::DisplayNameAction;
use shared::types::{Event, schema::FocusChange};

pub struct RedditBackstop {
    context: FireFoxBackStopContext,
}

const REDDIT_CUTOFF_SECONDS: u32 = 10;
const REDDIT_BASE_URL: &str = "https://www.reddit.com/";


fn matches_blacklist_urls(url: &String) -> bool {
    let popular_url: String = format!("{}r/popular", REDDIT_BASE_URL);

    *url == REDDIT_BASE_URL.to_string() || url.starts_with(&popular_url)
}


impl BackStop for RedditBackstop {
    fn new() -> Self {
        const REDDIT_WINDOW_RESET_SECONDS: i64 = 60 * 60;
        Self {
            context: FireFoxBackStopContext::new(REDDIT_WINDOW_RESET_SECONDS),
        }
    }

    fn trigger_backstop(
        &mut self, 
        le: &Option<&LastEvent>,
        e: &Event,
        fc: &Option<FocusChange>,
    ) -> Option<DisplayNameAction> {
        if let Some(last_event) = le {
            let prev_url = get_event_url(&last_event.event);
            
            if matches_blacklist_urls(&prev_url) {
                self.context.process_focus_change(fc.clone().unwrap());
            }
        }

        let url = get_event_url(e);
        let num_seconds = self.context.get_total_seconds_in_window();
        let remaining = Ord::max(REDDIT_CUTOFF_SECONDS - num_seconds, 0);
        if matches_blacklist_urls(&url) && remaining > 0 {
            let msg = "You seem to be doom-scrolling on Reddit r/popular.".to_string();

            return Some(DisplayNameAction::WarnTime(msg, remaining));
        } else if matches_blacklist_urls(&url) && remaining == 0 {
            return Some(DisplayNameAction::Block);
        }

        None
    }
}