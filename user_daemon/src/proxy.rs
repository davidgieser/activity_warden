use zbus::proxy;

#[proxy(interface="com.activity_warden.Watcher")]
pub trait FirefoxWatcher {
    async fn request_close(&self, metadata: &String) -> zbus::fdo::Result<()>;
}

#[proxy(
    interface = "org.gnome.ScreenSaver",
    default_service = "org.gnome.ScreenSaver",
    default_path = "/org/gnome/ScreenSaver"
)]
pub trait ScreenSaver {
    #[zbus(signal)]
    fn ActiveChanged(active: bool);
}

#[proxy(
    interface="org.freedesktop.login1.Manager",
    default_service="org.freedesktop.login1",
    default_path="/org/freedesktop/login1",
)]
pub trait SuspendListener {
    /// `start == true`  => about to go to sleep
    /// `start == false` => just resumed
    #[zbus(signal)]
    fn prepare_for_sleep(&self, start: bool) -> zbus::fdo::Result<()>;
}