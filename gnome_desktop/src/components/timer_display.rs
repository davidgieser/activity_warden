use relm4::adw;
use relm4::adw::prelude::*;
use relm4::gtk;
use relm4::prelude::*;
use relm4::factory::DynamicIndex;
use chrono::Datelike;

use shared::types::schema::Timer;
use crate::DurationId;

#[derive(Debug, Clone)]
pub enum TimerDisplayInput {
    UpdateDuration(DurationId),
}

#[derive(Debug)]
pub enum TimerDisplayOutput {
    Delete(DynamicIndex),
    Edit(DynamicIndex, Timer),
}

pub struct TimerDisplayModel {
    timer: Timer,
    duration: usize,
    index: DynamicIndex,
}

pub struct TimerInit {
    pub timer: Timer,
    pub duration: usize
}

#[relm4::factory(pub)]
impl FactoryComponent for TimerDisplayModel {
    type Init = TimerInit;
    type Input = TimerDisplayInput;
    type Output = TimerDisplayOutput;
    type CommandOutput = ();
    type ParentWidget = gtk::ListBox;
    type Index = DynamicIndex;

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 6,
            set_margin_top: 6,
            set_margin_bottom: 6,
            set_margin_start: 8,
            set_margin_end: 8,

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 0,
                add_css_class: "card",

                adw::ActionRow {
                    set_title: &self.timer.display_name,
                    // Time spent / Max allowed time:
                    #[watch]
                    set_subtitle: &format!(
                        "{} / {}",
                        fmt_mm_ss(self.duration as u32),
                        fmt_timer_duration(&self.timer),
                    ),

                    // Leading avatar:
                    add_prefix = &gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_margin_start: 8,
                        set_margin_end: 8,

                        gtk::Label {
                            add_css_class: "pill",
                            set_margin_top: 6,
                            set_margin_bottom: 6,
                            set_margin_start: 6,
                            set_margin_end: 6,
                            set_xalign: 0.5,
                            set_yalign: 0.5,
                            #[watch]
                            set_label: &self.timer
                                .display_name
                                .chars()
                                .next()
                                .map(|c| c.to_uppercase().collect::<String>())
                                .unwrap_or_else(|| "?".into()),
                        }
                    },

                    // Suffix: 
                    add_suffix = &gtk::Box {
                        set_spacing: 6,
                        set_margin_end: 6,
                        set_halign: gtk::Align::End,
                        set_valign: gtk::Align::Center,
                        set_hexpand: false,
                        set_vexpand: false,

                        // Edit button:
                        gtk::Button {
                            set_tooltip_text: Some("Edit timer"),
                            add_css_class: "flat",
                            set_has_frame: false,

                            #[name = "edit_btn_content"]
                            adw::ButtonContent {
                                set_icon_name: "document-edit-symbolic",
                            },
                            set_child: Some(&edit_btn_content),

                            connect_clicked[sender, index = self.index.clone(), timer = self.timer.clone()] => move |_| {
                                let _ = sender.output(TimerDisplayOutput::Edit(index.clone(), timer.clone()));
                            },
                        },

                        // Delete button:
                        gtk::Button {
                            set_tooltip_text: Some("Delete timer"),
                            add_css_class: "destructive-action",
                            add_css_class: "flat",
                            set_has_frame: false,
                            set_halign: gtk::Align::End,
                            set_valign: gtk::Align::Center,
                            set_hexpand: false,
                            set_vexpand: false,

                            #[name = "del_btn_content"]
                            adw::ButtonContent {
                                set_icon_name: "user-trash-symbolic",
                            },
                            set_child: Some(&del_btn_content),

                            connect_clicked[sender, index = self.index.clone()] => move |_| {
                                let idx = index.clone();
                                let sender = sender.clone();

                                gtk::glib::idle_add_once(move || {
                                    let _ = sender.output(TimerDisplayOutput::Delete(idx));
                                });
                            },
                        },
                    }
                }
            }
        }
    }

    fn init_model(init: Self::Init, index: &Self::Index, _sender: FactorySender<Self>) -> Self {
        TimerDisplayModel {
            timer: init.timer,
            duration: init.duration,
            index: index.clone(),
        }
    }

    fn update(&mut self, msg: Self::Input, _sender: FactorySender<Self>) {
        match msg {
            TimerDisplayInput::UpdateDuration(dur_id) => {
                if dur_id.host == self.timer.host && dur_id.display_name == self.timer.display_name {
                    self.duration = dur_id.duration;
                }
            }
        }
    }
}

/// If the timer is active, display its timer. 
/// Otherwise, display a time of 0.
fn fmt_timer_duration(timer: &Timer) -> String {
    let today = chrono::Local::now();
    let weekday = (today.weekday() as usize) + 1;
    let duration = if timer.allowed_days[weekday] {
        timer.time_limit
    } else {
        0
    };

    fmt_mm_ss(duration)
}

fn fmt_mm_ss(mut secs: u32) -> String {
    let minutes = secs / 60;
    secs %= 60;
    format!("{:02}:{:02}", minutes, secs)
}
