use relm4::{Component, ComponentSender, ComponentParts};
use relm4::gtk;
use relm4::gtk::prelude::*;
use relm4::adw::prelude::*;
use zbus::blocking::Connection as BlockingConnection;

use shared::dbus::{DBus, Host, Interface};
use crate::proxy::DaemonContextProxyBlocking;

#[derive(Debug)]
pub struct SettingsPage {
    dbus_conn: BlockingConnection,
    is_locked: bool,
}

#[derive(Debug)]
pub enum SettingsOut {
    LockStatusChange(bool),
}

#[derive(Debug)]
pub enum SettingsInput {
    /// Submit a newly entered password to the backend.
    SubmitPassword,
    /// Query the daemon to determine if there is a current password.
    LoadLockStatus,
}

#[derive(Debug)]
pub enum SettingsCmd {
    LockStatusLoaded(bool)
}

#[derive(Debug)]
pub struct SettingsInit {
    pub dbus_conn: BlockingConnection,
}

#[relm4::component(pub)]
impl Component for SettingsPage {
    type Init = SettingsInit;
    type Input = SettingsInput;
    type CommandOutput = SettingsCmd;
    type Output = SettingsOut;

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 14,
            set_margin_top: 18,
            set_margin_bottom: 18,
            set_margin_start: 18,
            set_margin_end: 18,

            // Header:
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 6,

                gtk::Label {
                    set_xalign: 0.0,
                    set_label: "Session Lock",
                    add_css_class: "title-2",
                },

                gtk::Label {
                    set_xalign: 0.0,
                    set_wrap: true,
                    set_label: "Enter your password to lock or unlock the session.",
                    add_css_class: "dim-label",
                },
            },

            // Password Entry Box:
            #[name = "password"]
            adw::PasswordEntryRow {
                set_title: "Password",
            },

            // Submit Button:
            gtk::Button {
                add_css_class: "pill",
                set_hexpand: true,
                set_halign: gtk::Align::Fill,

                #[name = "lock_btn_content"]
                adw::ButtonContent {
                    #[watch]
                    set_label: if model.is_locked { "Unlock" } else { "Lock" },
                    #[watch]
                    set_icon_name: if model.is_locked {
                        "changes-allow-symbolic"
                    } else {
                        "changes-prevent-symbolic"
                    },
                },
                set_child: Some(&lock_btn_content),

                connect_clicked[sender] => move |_| {
                    sender.input(SettingsInput::SubmitPassword);
                },
            },
        }
    }


    fn init(
        params: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = SettingsPage {
            dbus_conn: params.dbus_conn,
            is_locked: false,
        };
        let widgets = view_output!();

        sender.input(SettingsInput::LoadLockStatus);
        
        ComponentParts { model, widgets }
    }


    fn update_with_view(
            &mut self,
            widgets: &mut Self::Widgets,
            message: Self::Input,
            sender: ComponentSender<Self>,
            _root: &Self::Root,
    ) {
        match message {
            SettingsInput::SubmitPassword => {
                let dbus_conn = self.dbus_conn.clone();
                let password = widgets
                    .password
                    .text()
                    .to_string();
                widgets.password.set_text("");
                
                sender.spawn_oneshot_command(move || {
                    let proxy = DaemonContextProxyBlocking::builder(&dbus_conn)
                        .destination(DBus::host_name(&Host::UserDaemon)).unwrap()
                        .path(DBus::object_path(&Host::UserDaemon, &Interface::DaemonContext)).unwrap()
                        .build().unwrap();

                    let do_unlock = proxy.process_password_submission(password).unwrap();
                    SettingsCmd::LockStatusLoaded(!do_unlock)
                });
            },
            SettingsInput::LoadLockStatus => {
                let dbus_conn = self.dbus_conn.clone();
                sender.spawn_oneshot_command(move || {
                    let proxy = DaemonContextProxyBlocking::builder(&dbus_conn)
                        .destination(DBus::host_name(&Host::UserDaemon)).unwrap()
                        .path(DBus::object_path(&Host::UserDaemon, &Interface::DaemonContext)).unwrap()
                        .build().unwrap();

                    let is_locked = proxy.is_locked().unwrap();
                    SettingsCmd::LockStatusLoaded(is_locked)
                });
            }
        }    
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            SettingsCmd::LockStatusLoaded(is_locked) => {
                self.is_locked = is_locked;
                sender.output(SettingsOut::LockStatusChange(is_locked)).unwrap();
            }
        }
    }
}