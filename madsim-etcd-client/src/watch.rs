/// The kind of event.
#[repr(i32)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EventType {
    #[default]
    Put = 0,
    Delete = 1,
}
