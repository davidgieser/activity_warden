pub mod identifiers;

pub use identifiers::{Host, Interface};

pub struct DBus;
impl DBus {
    pub const BASE_URL: &'static str = "com.activity_warden";
    pub const BASE_PATH: &'static str = "/com/activity_warden";

    pub fn object_path(host: &Host, interface: &Interface) -> String {
        format!("{}/{}/{}", Self::BASE_PATH, host, interface)
    }

    pub fn host_name(host: &Host) -> String {
        format!("{}.{}", Self::BASE_URL, host)
    }

    pub fn interface_name(interface: &Interface) -> String {
        format!("{}.{}", Self::BASE_URL, interface)
    }
}