use relm4::adw;
use relm4::adw::prelude::*;
use relm4::gtk;
use relm4::prelude::*;

use shared::dbus::Host;
use shared::types::schema::Timer;

// // What we consider a successful submission
// #[derive(Debug, Clone)]
// pub struct TimerFormData {
//     pub url: String,
//     pub limit: u32,
//     pub days: Vec<bool>,
// }

#[derive(Clone)]
pub struct TimerPopupModel {
    hidden: bool,
    exit: bool,
}

#[derive(Debug)]
pub enum TimerPopupOutput {
    Submit(Timer),
}

#[derive(Debug)]
pub enum TimerPopupInput {
    Show,
    Cancel,
    // Submit now carries the captured form data
    Submit(Timer),
}

const DEFAULT_URL: &str = "";
const DEFAULT_LIMIT: u32 = 20;
const DEFAULT_DAYS: [bool; 7] = [true, true, true, true, true, true, true];

impl TimerPopupModel {
    fn new() -> Self {
        Self { hidden: true, exit: false }
    }
}

#[relm4::component(pub)]
impl Component for TimerPopupModel {
    type Init = ();
    type Input = TimerPopupInput;
    type Output = TimerPopupOutput;
    type CommandOutput = ();

    view! {
        #[name = "title"]
        adw::WindowTitle {
            set_title: "Create Timer",
            set_subtitle: "",
        },

        #[name = "limit_adj"]
        gtk::Adjustment {
            set_lower: 0.0,
            set_upper: 1440.0,
            set_step_increment: 1.0,
            set_page_increment: 10.0,
            set_value: DEFAULT_LIMIT as f64,
        },

        #[root]
        relm4::adw::Window {
            set_default_height: 150,
            set_default_width: 300,
            set_modal: true,
            set_hide_on_close: true,
            set_default_size: (400, 200),
            #[watch]
            set_visible: !model.hidden,

            relm4::gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                
                // Buttons on the header of the popup:
                adw::ToolbarView {
                    add_top_bar = &relm4::adw::HeaderBar {
                        set_show_start_title_buttons: true,
                        set_show_end_title_buttons: true,
                        set_title_widget: Some(&title),
                    },
                },

                // Popup container:
                relm4::gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 12,          
                    set_margin_top: 18,       
                    set_margin_bottom: 18,
                    set_margin_start: 18,
                    set_margin_end: 18,

                    // Spacer row:
                    relm4::adw::EntryRow {
                        set_title: "",
                        set_visible: false,
                    },

                    // URL input:
                    #[name = "url"]
                    relm4::adw::EntryRow {
                        set_title: "URL",
                        set_text: DEFAULT_URL,
                        set_position: -1,
                        add_prefix = &gtk::Image::from_icon_name("system-search-symbolic"),
                    },

                    // Time limit input:
                    #[name = "limit"]
                    relm4::adw::SpinRow {
                        set_title: "Time Limit (minutes)",
                        set_adjustment: Some(&limit_adj),
                        set_value: DEFAULT_LIMIT as f64,
                    },

                    // Active timer days input:
                    gtk::Box {
                        set_spacing: 6,   // space between day toggle buttons
                        add_css_class: "linked",
                        set_valign: gtk::Align::Center,
                        set_halign: gtk::Align::Center,

                        #[name = "sun"] gtk::ToggleButton { set_label: "Sun", set_active: DEFAULT_DAYS[0], },
                        #[name = "mon"] gtk::ToggleButton { set_label: "Mon", set_active: DEFAULT_DAYS[1], },
                        #[name = "tue"] gtk::ToggleButton { set_label: "Tue", set_active: DEFAULT_DAYS[2], },
                        #[name = "wed"] gtk::ToggleButton { set_label: "Wed", set_active: DEFAULT_DAYS[3], },
                        #[name = "thu"] gtk::ToggleButton { set_label: "Thu", set_active: DEFAULT_DAYS[4], },
                        #[name = "fri"] gtk::ToggleButton { set_label: "Fri", set_active: DEFAULT_DAYS[5], },
                        #[name = "sat"] gtk::ToggleButton { set_label: "Sat", set_active: DEFAULT_DAYS[6], },
                    },

                    relm4::gtk::Box {
                        set_valign: gtk::Align::Center,
                        set_halign: gtk::Align::Center,
                        set_spacing: 12,
                        set_orientation: gtk::Orientation::Horizontal,
                        set_margin_top: 12, 

                        // Submit Button: 
                        gtk::Button::with_label("Submit") {
                            connect_clicked[
                                sender, url, limit, sun, mon, tue, wed, thu, fri, sat
                            ] => move |_| {
                                // Multiply by 60 to convert to seconds.
                                let form = Timer {
                                    display_name: url.text().to_string(),
                                    host: Host::FirefoxWatcher,
                                    time_limit: limit.value() as u32 * 60,
                                    allowed_days: vec![
                                        sun.is_active(),
                                        mon.is_active(),
                                        tue.is_active(),
                                        wed.is_active(),
                                        thu.is_active(),
                                        fri.is_active(),
                                        sat.is_active(),
                                    ],
                                };
                                sender.input(TimerPopupInput::Submit(form));
                            }
                        },

                        // Cancel Button:
                        gtk::Button::with_label("Cancel") {
                            connect_clicked[sender] => move |_| {
                                sender.input(TimerPopupInput::Cancel);
                            }
                        },
                    }
                },
            },
            connect_close_request[sender] => move |_| {
                sender.input(TimerPopupInput::Cancel);
                gtk::glib::Propagation::Stop
            },
        }
    }

    fn init(
        _params: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = TimerPopupModel::new();
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update_with_view(
            &mut self,
            widgets: &mut Self::Widgets,
            message: Self::Input,
            sender: ComponentSender<Self>,
            root: &Self::Root,
    ) {
        match message {
            TimerPopupInput::Show => {
                self.hidden = false;
            },
            TimerPopupInput::Cancel => {
                self.hidden = true;
                self.exit = true;
            }
            TimerPopupInput::Submit(timer_form) => {
                self.hidden = true;
                self.exit = true;
                sender.output(TimerPopupOutput::Submit(timer_form)).unwrap();
            }
        }

        // Set the visibility of the modal.
        root.set_visible(!self.hidden);

        if self.exit {
            widgets.url.set_text(DEFAULT_URL);
            widgets.url.set_position(-1);
            widgets.limit.set_value(DEFAULT_LIMIT as f64);

            let days = vec![
                &widgets.sun,
                &widgets.mon,
                &widgets.tue,
                &widgets.wed,
                &widgets.thu,
                &widgets.fri,
                &widgets.sat,
            ];
            for (btn, &val) in days.iter().zip(DEFAULT_DAYS.iter()) {
                btn.set_active(val);
            }

            self.exit = false;
        }
    }
}
