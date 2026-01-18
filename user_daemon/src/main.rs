mod event_bus;
mod proxy;
mod persistence;
mod context;

use futures_lite::stream::StreamExt;
use std::sync::{Arc, atomic::{Ordering, AtomicBool}};
use serde_json::Value;
use zbus::Result;
use zbus::connection::Builder;
use tokio::sync::broadcast;
use tokio::signal::unix::{signal, SignalKind};
use tokio::time::{sleep, sleep_until};
use tokio::task::JoinHandle;
use chrono::{DateTime, Datelike, Duration as CDuration, NaiveTime, Local, Utc};
use shared::{dbus::{DBus, Host, Interface}, types::Event};
use shared::types::EventType;
use log::info;
use tokio::time::{self, Duration, Instant};
use zbus::Connection;

use crate::event_bus::EventBus;
use crate::proxy::{FirefoxWatcherProxy, SuspendListenerProxy, ScreenSaverProxy};
use crate::context::DaemonContext;

/// The maximum size of the event bus before old messages are dropped.
const CAPACITY: usize = 100;

pub enum DisplayNameAction {
    /// Close the given display name.
    Block,
    /// Set a timer for `u32` seconds.
    Time(u32),
    /// Track the time, but perform no actions.
    Ignore,
}


/// Determine how much time must elapse until midnight in the local timezone.
/// At this point, the daemon will wake up and close any stale messages.
/// 
/// This is only necessary to prevent durations from the previous day from
/// impacting the following day. For example, a user spending 30 minutes on a
/// display name around midnight would want that time properly split across the
/// day boundary.
fn instant_until_next_local_midnight() -> Instant {
    // Fetch midnight tomorrow based on the local timezone.
    let tomorrow_local = Local::now() + CDuration::days(1);
    let next_midnight_local = DateTime::with_time(
        &tomorrow_local, 
        NaiveTime::from_hms_opt(0, 0, 0).unwrap())
        .single()
        .expect("Failed to find a Local midnight.");

    // Convert to UTC to account for date based edge cases.
    let next_midnight_utc = next_midnight_local.with_timezone(&Utc);
    Instant::now()
        + std::time::Duration::from_secs(
            (next_midnight_utc - Utc::now()).num_seconds() as u64
        )
}


/// Determine if a particular display name is blocked or if a new timer should be set.
fn is_display_name_blocked(context: &DaemonContext, host: &Host, display_name: &String) -> DisplayNameAction {
    let today = chrono::Local::now();
    let timers = context.timers.load_full();
    for timer in (*timers).clone() {
        if timer.display_name == *display_name {
            let weekday = ((today.weekday() as usize) + 1) % 7;
            info!("timer.allowed_days[{}] = {}", weekday, timer.allowed_days[weekday]);
            if timer.allowed_days[weekday] {
                if let Some(host_durations) = context.timer_durations.get(host) {
                    // If the day is specified, and the timer is set to 0, no activity will be tracked.
                    // As such, we implicitly know that the page is blocked.
                    if timer.time_limit == 0 {
                        info!("[BLOCKING] {}: timer is allotted 0 seconds.", display_name);
                        return DisplayNameAction::Block;
                    }

                    let cur_duration = host_durations.get(display_name).unwrap_or(&0);
                    if cur_duration >= &timer.time_limit {
                        info!("[BLOCKING] {}: current duration ({}) is greater than the limit ({}).", display_name, cur_duration, timer.time_limit);
                        return DisplayNameAction::Block;
                    } else {
                        info!("[NON-BLOCKING] {}: current duration ({}) is less than the limit ({}).", display_name, cur_duration, timer.time_limit);
                        return DisplayNameAction::Time(timer.time_limit - cur_duration);
                    }
                }
            }
            info!("[BLOCKING] {}: the timer is disabled on {}.", display_name, today.format("%a"));
            return DisplayNameAction::Block;
        }
    }   

    // Otherwise, conclude that the name is not blocked.
    DisplayNameAction::Ignore
}

/// Close the provided display name.
pub async fn block_display_name(
    session_conn: Connection,
    event: Event,
    timeout: u32,
) -> Result<()> {
    // Wait for the timer to expire before sending the shutdown.
    if timeout > 0 {
        let dur = Duration::from_secs(timeout as u64);
        time::sleep(dur).await;
    }

    // Actually connect to the respective `Watcher` to close the display name.
    match event.source {
        Host::FirefoxWatcher => {
            let proxy = FirefoxWatcherProxy::builder(&session_conn)
                .destination(DBus::host_name(&event.source))?
                .path(DBus::object_path(&Host::FirefoxWatcher, &Interface::Watcher))?
                .build()
                .await?;

            let metadata: Value = serde_json::from_str(&event.metadata)
                .expect("Invalid metadata structure");
            let _ = proxy.request_close(&metadata.to_string()).await;
        },
        _ => { panic!("Received unexpected host for the FocusChange event.") }
    };

    Ok(())
}

/// Inform any listeners (i.e. the GUI) that a new duration has been
/// processed. This enables listeners to maintain state consistent
/// with the daemon.
pub async fn emit_focus_change(
    context: &mut DaemonContext,
    session_conn: &Connection,
    event: Event,
    set_last_event: bool,
) {
    let fc = context.update_event_durations(&event, set_last_event);
    if fc.is_some() {
        info!("[EMIT] FocusChange signal...");
        session_conn.emit_signal(
            None::<&str>,
            DBus::object_path(&Host::UserDaemon, &Interface::DaemonContext), 
            DBus::interface_name(&Interface::DaemonContext),
            "DurationChanged",
            &fc.unwrap(),
        ).await.unwrap();
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Build the rust logger.
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // Expose the daemon on the session DBus.
    let mut context = DaemonContext::new();
    let sender = broadcast::Sender::new(CAPACITY);
    let mut receiver = sender.subscribe();
    let event_channel = EventBus::new(sender);
    let session_conn = Builder::session()?
        .name(DBus::host_name(&Host::UserDaemon))?
        .serve_at(DBus::object_path(&Host::UserDaemon, &Interface::EventBus), event_channel)?
        .serve_at(DBus::object_path(&Host::UserDaemon, &Interface::DaemonContext), context.clone())?
        .build()
        .await?;

    let shutdown_flag = Arc::new(AtomicBool::new(false));
    let flag_clone = shutdown_flag.clone();

    // Listen for Ctrl + C to shut down the daemon.
    tokio::spawn(async move {
        let mut sigterm = signal(SignalKind::terminate())
            .expect("failed to register SIGTERM handler");

        sigterm.recv().await; // Wait for SIGTERM

        info!("SIGTERM received, setting shutdown flag...");
        flag_clone.store(true, Ordering::SeqCst);
    });

    // Listen for events that imply the computer is turning off.
    let system_conn = zbus::Connection::system().await?;
    let suspend_proxy = SuspendListenerProxy::new(&system_conn).await.unwrap();
    let mut sleep_stream = suspend_proxy.receive_prepare_for_sleep().await.unwrap();

    let screen_saver_proxy = ScreenSaverProxy::new(&session_conn).await.unwrap();
    let mut screen_stream = screen_saver_proxy.receive_ActiveChanged().await.unwrap();

    // Set some additional intervals to keep the event loop from becoming stale.
    let mut midnight_dur = instant_until_next_local_midnight();
    let timeout_dur = tokio::time::Duration::from_millis(500);
    let mut timer_task: Option<JoinHandle<()>> = None;
    loop {
        tokio::select! {
            // Listen to the event bus to receive events from watchers.
            event_result = receiver.recv() => {
                let event = if event_result.is_ok() { 
                    event_result.unwrap()
                } else {
                    continue;
                };
                
                // Cancel the timer future on receipt of a new event.
                if let Some(tt) = &timer_task {
                    tt.abort();
                }

                // Process the incoming event.
                context.reset_daily_state();
                match &event.event_type {
                    EventType::FocusChange => {
                        let action = is_display_name_blocked(&context, &event.source, &event.display_name);
                        match action {
                            DisplayNameAction::Time(remaining_duration) => {
                                emit_focus_change(&mut context, &session_conn, event.clone(), true).await;
                                
                                // Spawn the task to close the display name upon timer expiration.
                                timer_task = Some(tokio::spawn({
                                    let session_conn = session_conn.clone();
                                    let event = event.clone();
                                    async move {
                                        let _ = block_display_name(session_conn, event, remaining_duration).await;
                                    }
                                }));
                            }
                            DisplayNameAction::Block => {
                                let _ = block_display_name(session_conn.clone(), event, 0).await;
                            },
                            DisplayNameAction::Ignore => {
                                emit_focus_change(&mut context, &session_conn, event, true).await;
                            },
                        }
                    },
                    EventType::FocusLost => {
                        emit_focus_change(&mut context, &session_conn, event, false).await;
                    },
                    EventType::AFK => { /* Implement in the future. */ },
                }
            }

            suspend = sleep_stream.next() => {
                let resp = suspend.unwrap();
                let suspend_args = resp.args()?;
                info!("[SUSPEND]: {:?}", suspend_args.start);

                context.clear_last_event();
            }

            screen_active = screen_stream.next() => {
                let resp = screen_active.unwrap();
                let screen_args = resp.args()?;
                info!("[SCREEN OFF]: {:?}", screen_args.active);

                context.clear_last_event();
            }
            
            // Wake up at midnight to ensure that state is properly stored across the day boundary.
            _ = sleep_until(midnight_dur) => {
                let last_event_map = context.last_event.clone();
                for (_, last_event) in last_event_map.iter() {
                    context.update_event_durations(
                        &last_event.event,
                        true
                    );
                }

                // Set the new timeout to midnight on the following day.
                midnight_dur = instant_until_next_local_midnight();
            }
        
            // Wake up every `timeout_dur` milliseconds to check the shutdown flag.
            _ = sleep(timeout_dur) => {
                if shutdown_flag.load(Ordering::SeqCst) {
                    break;
                }
            }
        }
    }

    info!("Terminating the User Daemon!");
    Ok(())
}