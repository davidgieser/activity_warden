mod components;
mod proxy;
mod pages;

use std::collections::HashMap;
use std::cell::RefCell;
use std::rc::Rc;

use relm4::{ComponentParts, ComponentSender, Controller, Component};
use relm4::{gtk, ComponentController};
use relm4::gtk::prelude::*;
use relm4::prelude::*;
use shared::dbus::{DBus, Host, Interface};
use futures_util::stream::StreamExt;
use zbus::Connection;
use zbus::blocking::{connection::Builder, Connection as BlockingConnection};
use shared::types::schema::FocusChange;
use shared::types::daemon::DurationMap;

use crate::components::header_model::{HeaderModel, HeaderModelOutput};
use crate::proxy::DaemonContextProxy;
use crate::proxy::DaemonContextProxyBlocking;
use crate::pages::home::{HomeInit, HomePage, HomeInput};
use crate::pages::data::{DataInit, DataPage, DataInput};
use crate::pages::settings::{SettingsInit, SettingsOut, SettingsPage};
use crate::pages::Page;


pub type Duration = usize;

#[derive(Clone, Debug, PartialEq, PartialOrd, Eq, Ord)]
pub struct DurationId {
    duration: Duration,
    display_name: String,
    host: Host,
}

#[derive(Debug)]
enum AWMsg {
    /// Close the entire application.
    Close,
    /// Set the active page of the application.
    SetMode(Page),
    /// Receive updates on duration changes.
    DurationUpdate(FocusChange),
    /// Change whether or not the application is locked.
    LockStatusChange(bool),
    /// Load the initial durations to populate state.
    LoadDurations,
    /// A non operation for pages without output messages.
    NoOp,
}

#[derive(Debug)]
enum AWCommandMsg {
    DurationsLoaded(DurationMap)
}

struct AWModelInit {
    mode: Page,
    dbus_conn: BlockingConnection,
}

struct AWModel {
    // UI State:
    page: Page,
    header: Controller<HeaderModel>,
    home: Controller<HomePage>,
    data: Controller<DataPage>,
    settings: Controller<SettingsPage>,

    // Internal State:
    timer_durations: Rc<RefCell<DurationMap>>,
    is_locked: Rc<RefCell<bool>>,

    // DBus Proxies:
    dbus_conn: BlockingConnection, 
}


#[relm4::component]
impl Component for AWModel {
    type Init = AWModelInit;
    type Input = AWMsg;
    type Output = ();
    type CommandOutput = AWCommandMsg;
    
    view! {
        main_window = adw::ApplicationWindow {
            set_default_width: 600,
            set_default_height: 350,
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,

                model.header.widget(),
                gtk::Stack {
                    set_vexpand: true,
                    set_hexpand: true,
                    set_transition_type: gtk::StackTransitionType::SlideLeftRight,
                    set_transition_duration: 200,

                    add_named[Some("timers")] = model.home.widget(),
                    add_named[Some("data")] = model.data.widget(),
                    add_named[Some("settings")] = model.settings.widget(),
                    
                    // Determine which page in the stack should be visible.
                    #[watch]
                    set_visible_child_name: model.page.name(),
                },
            },

            connect_close_request[sender] => move |_| {
                sender.input(AWMsg::Close);
                gtk::glib::Propagation::Stop
            }
        }
    }

    fn init(
        params: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let header = HeaderModel::builder()
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                HeaderModelOutput::Timers => AWMsg::SetMode(Page::Timers),
                HeaderModelOutput::Data => AWMsg::SetMode(Page::Data),
                HeaderModelOutput::Settings => AWMsg::SetMode(Page::Settings),
            });

        let timers = Rc::new(RefCell::new(Vec::new()));
        let timer_durations = Rc::new(RefCell::new(HashMap::new()));
        let is_locked = Rc::new(RefCell::new(false));

        // Create the controllers for each of the application pages.
        let home = HomePage::builder()
            .launch(HomeInit {
                dbus_conn: params.dbus_conn.clone(),
                timers,
                timer_durations: timer_durations.clone(),
                is_locked: is_locked.clone(),
            })
            .forward(sender.input_sender(), |_o| AWMsg::NoOp); 

        let data = DataPage::builder()
            .launch(DataInit {
                dbus_conn: params.dbus_conn.clone(),
                timer_durations: timer_durations.clone(),
            })
            .forward(sender.input_sender(), |_o| AWMsg::NoOp);

        let settings = SettingsPage::builder()
            .launch(SettingsInit {
                dbus_conn: params.dbus_conn.clone(),
            })
            .forward(sender.input_sender(), |o| {
                match o {
                    SettingsOut::LockStatusChange(is_locked) => AWMsg::LockStatusChange(is_locked)
                }
            });

        let list_box = gtk::ListBox::new();
        list_box.set_selection_mode(gtk::SelectionMode::None);

        // Spawn the thread to listen for duration updates from the daemon.
        let signal_sender = sender.clone();
        relm4::tokio::spawn(async move {
            let conn = Connection::session().await.unwrap();

            let proxy = DaemonContextProxy::builder(&conn)
                .destination(DBus::host_name(&Host::UserDaemon)).unwrap()
                .path(DBus::object_path(&Host::UserDaemon, &Interface::DaemonContext)).unwrap()
                .build().await.unwrap();

            let mut stream = proxy.receive_duration_changed().await.unwrap();
            while let Some(sig) = stream.next().await {
                let args = sig.args().unwrap();
                let changes: &FocusChange = args.change();
                signal_sender.input(AWMsg::DurationUpdate(changes.clone()));
            }
        });
        
        // Load the initial timer durations.
        sender.input(AWMsg::LoadDurations);

        let model = AWModel { 
            page: params.mode, 
            header, 
            home,
            data,
            settings,
            timer_durations,
            is_locked,
            dbus_conn: params.dbus_conn,
        };

        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(
        &mut self,
        msg: Self::Input,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            AWMsg::Close => {
                relm4::main_adw_application().quit();
            },
            AWMsg::SetMode(mode) => {
                self.page = mode;
            },
            AWMsg::DurationUpdate(fc) => {
                // Update the complete set of durations maintained per host.
                let mut timer_durations = (*self.timer_durations).borrow_mut();
                let host_map = timer_durations.entry(fc.host.clone()).or_default();
                let cur_duration = host_map.entry(fc.display_name.clone()).or_default();
                let mut dur_id = DurationId { 
                    duration: *cur_duration as usize, 
                    display_name: fc.display_name,
                    host: fc.host,
                };
                *cur_duration += fc.duration;

                // Inform the pertinent pages of the update.
                self.data.sender().send(DataInput::DurationUpdate(
                    dur_id.clone(), 
                    *cur_duration as usize
                )).unwrap();

                dur_id.duration = *cur_duration as usize;
                self.home.sender().send(HomeInput::DurationUpdate(dur_id)).unwrap();
            },
            AWMsg::LockStatusChange(is_locked) => {
                (*self.is_locked).replace(is_locked);
            }
            AWMsg::LoadDurations => {
                let dbus_conn = self.dbus_conn.clone();
                sender.spawn_oneshot_command(move || {
                    let proxy = DaemonContextProxyBlocking::builder(&dbus_conn)
                        .destination(DBus::host_name(&Host::UserDaemon)).unwrap()
                        .path(DBus::object_path(&Host::UserDaemon, &Interface::DaemonContext)).unwrap()
                        .build().unwrap();

                    let snapshot = proxy.get_daemon_snapshot().unwrap();
                    AWCommandMsg::DurationsLoaded(snapshot.durations)
                });
            },
            AWMsg::NoOp => { }
        }
    }


    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        _sender: ComponentSender<Self>,
        _: &Self::Root,
    ) {
        match message {
            AWCommandMsg::DurationsLoaded(duration_map) => {
                (*self.timer_durations).replace(duration_map);

                self.data.sender().send(DataInput::DurationsLoaded).unwrap();
                self.home.sender().send(HomeInput::DurationsLoaded).unwrap();
            }
        }
    }
}

fn main() {
    let dbus_conn = Builder::session().unwrap()
        .name(DBus::host_name(&Host::GnomeApplication)).unwrap()
        .build()
        .unwrap();

    let relm = relm4::RelmApp::new("com.activity_warden.gui");
    let init_state = AWModelInit {
        mode: Page::Timers,
        dbus_conn,
    };
    relm.run::<AWModel>(init_state);
}
