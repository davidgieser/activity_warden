use relm4::{Component, ComponentParts, ComponentSender, Controller};
use relm4::gtk;
use relm4::prelude::*;
use relm4::gtk::prelude::*;
use shared::types::schema::Timer;
use shared::dbus::{DBus, Host, Interface};
use shared::types::daemon::DurationMap;
use zbus::blocking::Connection as BlockingConnection;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::DurationId;
use crate::components::timer_display::{TimerDisplayInput, TimerDisplayModel, TimerDisplayOutput, TimerInit};
use crate::components::timer_popup::{TimerPopupModel, TimerPopupInput, TimerPopupOutput};
use crate::proxy::{DaemonContextProxyBlocking};

#[derive(Debug)]
pub enum HomeInput {
    LoadTimers,
    CreateTimer(Timer),
    DeleteTimer(DynamicIndex),
    UpdateTimer(DynamicIndex, Timer),
    /// Transmit the old duration ID with the new duration delta.
    DurationUpdate(DurationId),
    DurationsLoaded
}

#[derive(Debug)]
pub enum HomeCmd {
    TimersLoaded(Vec<Timer>),
    TimerCreated(Timer),
    TimerDeleted(usize),
    TimerUpdated(Timer, usize),
}

#[derive(Debug)]
pub enum HomeOut { }

pub struct HomeInit {
    pub dbus_conn: BlockingConnection,
    pub timers: Rc<RefCell<Vec<Timer>>>,
    pub timer_durations: Rc<RefCell<DurationMap>>,
    pub is_locked: Rc<RefCell<bool>>,
}

pub struct HomePage {
    dbus_conn: BlockingConnection,

    // UI Components:
    timer_popup: Controller<TimerPopupModel>,
    timer_factory: FactoryVecDeque<TimerDisplayModel>,

    // Internal State:
    timers: Rc<RefCell<Vec<Timer>>>,
    timer_durations: Rc<RefCell<DurationMap>>,
    is_locked: Rc<RefCell<bool>>,
}

#[relm4::component(pub)]
impl Component for HomePage {
    type Init = HomeInit;
    type Input = HomeInput;
    type Output = HomeOut;
    type CommandOutput = HomeCmd;

    view! {
        #[root]
        gtk::Overlay {
            set_vexpand: true,
            set_hexpand: true,

            // Timer List:
            gtk::ScrolledWindow {
                set_vexpand: true,
                set_hexpand: true,
                #[local_ref] timer_display_box -> gtk::ListBox { }
            },

            // Overlay Button:
            add_overlay = &gtk::Button {
                set_child: Some(&gtk::Image::from_icon_name("list-add-symbolic")),
                add_css_class: "circular",
                set_width_request: 48,
                set_height_request: 48,
                set_halign: gtk::Align::End,
                set_valign: gtk::Align::End,
                set_margin_start: 12,
                set_margin_end: 16,
                set_margin_top: 12,
                set_margin_bottom: 16,

                connect_clicked[sender = model.timer_popup.sender().clone()] => move |_| {
                    sender.send(TimerPopupInput::Show).ok();
                },
            }
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        // Create the popup for creating / editing a timer.
        let timer_popup = TimerPopupModel::builder()
            .transient_for(&root)
            .launch(())
            .forward(sender.input_sender(), |o| match o {
                TimerPopupOutput::Submit(timer) => HomeInput::CreateTimer(timer),
            });

        // Disable selection of any of the timers.
        let list_box = gtk::ListBox::new();
        list_box.set_selection_mode(gtk::SelectionMode::None);

        let timer_factory = FactoryVecDeque::<TimerDisplayModel>::builder()
            .launch(list_box)
            .forward(sender.input_sender(), |o| match o {
                TimerDisplayOutput::Delete(idx) => HomeInput::DeleteTimer(idx),
                TimerDisplayOutput::Edit(idx, new_timer) => HomeInput::UpdateTimer(idx, new_timer),
            });

        let model = HomePage {
            dbus_conn: init.dbus_conn,
            timer_popup,
            timer_factory,
            timers: init.timers,
            timer_durations: init.timer_durations,
            is_locked: init.is_locked,
        };

        let timer_display_box = model.timer_factory.widget();
        let widgets = view_output!();

        // Load the initial timers for the frontend.
        sender.input(HomeInput::LoadTimers);
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: HomeInput, sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            HomeInput::DurationUpdate(dur_id) => {
                self.timer_factory.broadcast(TimerDisplayInput::UpdateDuration(dur_id));
            },
            HomeInput::DurationsLoaded => {
                let timers = self.timers.borrow();
                let dur_map = self.timer_durations.borrow();
                let tmp_hash_map = HashMap::new();
                for t in timers.iter() {
                    let duration = dur_map.get(&t.host)
                        .unwrap_or(&tmp_hash_map)
                        .get(&t.display_name)
                        .unwrap_or(&0);
                    let dur_id = DurationId {
                        duration: *duration as usize,
                        display_name: t.display_name.clone(),
                        host: t.host.clone(),
                    };
                    self.timer_factory.broadcast(TimerDisplayInput::UpdateDuration(dur_id));
                }
            },
            HomeInput::LoadTimers => {
                let dbus_conn = self.dbus_conn.clone();
                sender.spawn_oneshot_command(move || {
                    let proxy = DaemonContextProxyBlocking::builder(&dbus_conn)
                        .destination(DBus::host_name(&Host::UserDaemon)).unwrap()
                        .path(DBus::object_path(&Host::UserDaemon, &Interface::DaemonContext)).unwrap()
                        .build().unwrap();

                    let snapshot = proxy.get_daemon_snapshot().unwrap();
                    HomeCmd::TimersLoaded(snapshot.timers)
                });
            }
            HomeInput::CreateTimer(timer) => {
                let dbus_conn = self.dbus_conn.clone();
                sender.spawn_oneshot_command(move || {
                    let proxy = DaemonContextProxyBlocking::builder(&dbus_conn)
                        .destination(DBus::host_name(&Host::UserDaemon)).unwrap()
                        .path(DBus::object_path(&Host::UserDaemon, &Interface::DaemonContext)).unwrap()
                        .build().unwrap();

                    proxy.insert_timer(timer.clone()).unwrap();
                    HomeCmd::TimerCreated(timer)
                });
            }
            HomeInput::DeleteTimer(idx) => {
                // Only allow a timer to be deleted if the application is unlocked.
                if !*self.is_locked.borrow() {
                    let i = idx.current_index();
                    let del_timer = (*self.timers).borrow()[i].clone();
                    let dbus_conn = self.dbus_conn.clone();
                    sender.spawn_oneshot_command(move || {
                        let proxy = DaemonContextProxyBlocking::builder(&dbus_conn)
                            .destination(DBus::host_name(&Host::UserDaemon)).unwrap()
                            .path(DBus::object_path(&Host::UserDaemon, &Interface::DaemonContext)).unwrap()
                            .build().unwrap();

                        proxy.delete_timer(del_timer).unwrap();
                        HomeCmd::TimerDeleted(i)
                    });
                }
            }
            HomeInput::UpdateTimer(idx, timer) => {
                // Only allow a timer to be updated if the application is unlocked.
                if !*self.is_locked.borrow() {
                    let i = idx.current_index();
                    let dbus_conn = self.dbus_conn.clone();
                    sender.spawn_oneshot_command(move || {
                        let proxy = DaemonContextProxyBlocking::builder(&dbus_conn)
                            .destination(DBus::host_name(&Host::UserDaemon)).unwrap()
                            .path(DBus::object_path(&Host::UserDaemon, &Interface::DaemonContext)).unwrap()
                            .build().unwrap();

                        proxy.update_timer(timer.clone()).unwrap();
                        HomeCmd::TimerUpdated(timer, i)
                    });
                }
            }
        }
    }

    fn update_cmd(
        &mut self,
        msg: HomeCmd,
        _sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            HomeCmd::TimersLoaded(timers) => {
                let mut guard = self.timer_factory.guard();
                guard.clear();

                let map = HashMap::new();
                let durations = (*self.timer_durations).borrow();
                for t in &timers {
                    let duration = durations.get(&t.host)
                        .unwrap_or(&map)
                        .get(&t.display_name)
                        .unwrap_or(&0);
                    
                    let init = TimerInit {
                        timer: t.clone(),
                        duration: *duration as usize,
                    };
                    guard.push_back(init);
                }
                (*self.timers).replace(timers);
            }
            HomeCmd::TimerCreated(timer) => {
                (*self.timers).borrow_mut().push(timer.clone());

                let durations = (*self.timer_durations).borrow();
                let map = HashMap::new();
                let duration = durations.get(&timer.host)
                    .unwrap_or(&map)
                    .get(&timer.display_name)
                    .unwrap_or(&0);
                let init = TimerInit {
                    timer,
                    duration: *duration as usize,
                };
                self.timer_factory.guard().push_back(init);
            }
            HomeCmd::TimerDeleted(i) => {
                (*self.timers).borrow_mut().remove(i);
                self.timer_factory.guard().remove(i);
            }
            HomeCmd::TimerUpdated(timer, i) => {
                (*self.timers).borrow_mut()[i] = timer.clone();
                let mut g = self.timer_factory.guard();
                g.remove(i);

                let durations = (*self.timer_durations).borrow();
                let map = HashMap::new();
                let duration = durations.get(&timer.host)
                    .unwrap_or(&map)
                    .get(&timer.display_name)
                    .unwrap_or(&0);
                let init = TimerInit {
                    timer,
                    duration: *duration as usize,
                };
                g.insert(i, init);
            }
        }
    }
}