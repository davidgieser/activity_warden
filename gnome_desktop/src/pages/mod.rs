pub mod settings;
pub mod data;
pub mod home;


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    /// Display all current timers.
    Timers,
    /// Display a graph of the current time spent on websites.
    Data,
    /// Provide a place to change the locking password.
    Settings,
}

impl Page {
    pub fn name(self) -> &'static str {
        match self {
            Page::Timers => "timers",
            Page::Data => "data",
            Page::Settings => "settings",
        }
    }
}