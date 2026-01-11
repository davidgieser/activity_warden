use relm4::{SimpleComponent, ComponentSender, ComponentParts};
use relm4::gtk;
use relm4::gtk::prelude::*;

pub struct HeaderModel {}

#[derive(Debug)]
pub enum HeaderModelOutput {
    Timers,
    Data,
    Settings
}

#[relm4::component(pub)]
impl SimpleComponent for HeaderModel {
    type Init = ();
    type Input = ();
    type Output = HeaderModelOutput;

    view! {
        // Display the possible pages.
        #[root]
        adw::HeaderBar {
            #[wrap(Some)]
            set_title_widget = &gtk::Box {
                add_css_class: "linked",
                #[name = "group"]
                gtk::ToggleButton {
                    set_label: "Timers",
                    set_active: true,
                    connect_toggled[sender] => move |btn| {
                        if btn.is_active() {
                            sender.output(HeaderModelOutput::Timers).unwrap();
                        }
                    }
                },
                gtk::ToggleButton {
                    set_label: "Data",
                    set_group: Some(&group),
                    set_active: false,
                    connect_toggled[sender] => move |btn| {
                        if btn.is_active() {
                            sender.output(HeaderModelOutput::Data).unwrap();
                        }
                    }
                },
                gtk::ToggleButton {
                    set_label: "Settings",
                    set_group: Some(&group),
                    set_active: false,
                    connect_toggled[sender] => move |btn| {
                        if btn.is_active() {
                            sender.output(HeaderModelOutput::Settings).unwrap();
                        }
                    }
                },
            }
        }
    }

    fn init(
        _params: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = HeaderModel {};
        let widgets = view_output!();
        
        ComponentParts { model, widgets }
    }
}