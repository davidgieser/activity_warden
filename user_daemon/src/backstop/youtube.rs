use crate::{
    backstop::{
        BackStop, 
        FireFoxBackStopContext,
        get_event_url
    }, 
    context::LastEvent,
};
use crate::DisplayNameAction;
use chrono::{DateTime, Duration, Local, TimeZone};
use log::info;
use shared::types::{Event, schema::FocusChange};

pub struct YoutubeBackstop {
    base_context: FireFoxBackStopContext,
    shorts_context: FireFoxBackStopContext,
}

const YOUTUBE_CUTOFF_SECONDS: u32 = 10 * 60;
const YOUTUBE_SHORTS_CUTOFF_SECONDS: u32 = 30;
const YOUTUBE_BASE_URL: &str = "https://www.youtube.com/";


fn is_shorts_url(url: &String) -> bool {
    let shorts_url: String = format!("{}shorts", YOUTUBE_BASE_URL);

    url.starts_with(&shorts_url)
}


fn matches_blacklist_urls(url: &String) -> bool {
    url.starts_with(YOUTUBE_BASE_URL)
}


fn construct_literal(hour: u32, min: u32, sec: u32, next_day: bool) -> DateTime<Local> {
    let mut dt = Local::now();
    if next_day {
        dt += Duration::days(1)
    }

    Local.with_ymd_and_hms(
            dt.format("%Y").to_string().parse().unwrap(),
            dt.format("%m").to_string().parse().unwrap(),
            dt.format("%d").to_string().parse().unwrap(),
            hour,
            min,
            sec,
        )
        .single()
        .unwrap()
}


impl BackStop for YoutubeBackstop {
    fn new() -> Self {
        const YOUTUBE_WINDOW_RESET_SECONDS: i64 = 60 * 60;
        Self {
            base_context: FireFoxBackStopContext::new(YOUTUBE_WINDOW_RESET_SECONDS),
            shorts_context: FireFoxBackStopContext::new(YOUTUBE_WINDOW_RESET_SECONDS),
        }
    }

    fn trigger_backstop(
        &mut self, 
        le: &Option<&LastEvent>,
        e: &Event,
        fc: &Option<FocusChange>,
    ) -> Option<DisplayNameAction> {
        let now = Local::now();
        let ten_pm = construct_literal(20, 0, 0, false);
        let four_am = construct_literal(4, 0, 0, true);

        if let Some(last_event) = le {
            let prev_url = get_event_url(&last_event.event);
            let should_count_youtube = matches_blacklist_urls(&prev_url) && now > ten_pm && now < four_am;

            // Only track URLs that are for YouTube and the time is between the blacklisted times.
            info!("{}, {}, {}", now, ten_pm, four_am);
            if should_count_youtube {
                info!("True!");
                self.base_context.process_focus_change(fc.clone().unwrap());
            }

            // For shorts specifically, we double count and allow a shorter duration.
            if is_shorts_url(&prev_url) {
                self.shorts_context.process_focus_change(fc.clone().unwrap());
            }
        }

        let url = get_event_url(e);
        if !matches_blacklist_urls(&url) {
            return None;
        }

        // Only block YouTube if it is between the outlawed times. Shorts are always blocked.
        let should_block_youtube = matches_blacklist_urls(&url) && now > ten_pm && now < four_am;

        let num_seconds = self.base_context.get_total_seconds_in_window();
        let remaining = Ord::max(YOUTUBE_CUTOFF_SECONDS - num_seconds, 0);
        if is_shorts_url(&url) {
            let msg = "You seem to be doom-scrolling on YouTube shorts.".to_string();
            let num_shorts_seconds = self.shorts_context.get_total_seconds_in_window();
            let remaining_shorts = Ord::max(YOUTUBE_SHORTS_CUTOFF_SECONDS - num_shorts_seconds, 0);

            return Some(DisplayNameAction::WarnTime(msg.into(), remaining_shorts));
        } else if remaining == 0 && should_block_youtube {
            return Some(DisplayNameAction::Block);
        } else if should_block_youtube {
            let msg = "You seem to be doom-scrolling on YouTube.".to_string();
            return Some(DisplayNameAction::WarnTime(msg, remaining));
        } else {
            return None;
        }
    }
}