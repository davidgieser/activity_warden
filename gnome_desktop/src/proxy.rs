use zbus::{fdo, proxy};
use shared::types::schema::{FocusChange, Timer};
use shared::types::daemon::DaemonSnapshot;

#[proxy(interface = "com.activity_warden.DaemonContext")]
pub trait DaemonContext {
    #[zbus(signal)]
    async fn duration_changed(&self, change: FocusChange) -> Result<()>;

    fn get_daemon_snapshot(&self) -> fdo::Result<DaemonSnapshot>;
    fn insert_timer(&self, timer: Timer) -> fdo::Result<()>;
    fn delete_timer(&self, timer: Timer) -> fdo::Result<()>;
    fn update_timer(&self, timer: Timer) -> fdo::Result<()>;
    fn is_locked(&self) -> fdo::Result<bool>;
    fn process_password_submission(&self, password: String) -> fdo::Result<bool>;
}