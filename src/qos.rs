use std::mem;

use ffi::*;

/// An abstract thread quality of service (QOS) classification.
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub enum QosClass {
    /// A QOS class which indicates work performed by this thread is interactive with the user.
    UserInteractive = QOS_CLASS_USER_INTERACTIVE,
    /// A QOS class which indicates work performed by this thread was initiated by the user
    /// and that the user is likely waiting for the results.
    UserInitiated = QOS_CLASS_USER_INITIATED,
    /// A default QOS class used by the system in cases where more specific QOS class information is not available.
    Default = QOS_CLASS_DEFAULT,
    /// A QOS class which indicates work performed by this thread may or may not be initiated by the user
    /// and that the user is unlikely to be immediately waiting for the results.
    Utility = QOS_CLASS_UTILITY,
    /// A QOS class which indicates work performed by this thread was not initiated by the user
    /// and that the user may be unaware of the results.
    Background = QOS_CLASS_BACKGROUND,
    /// A QOS class value which indicates the absence or removal of QOS class information.
    Unspecified = QOS_CLASS_UNSPECIFIED,
}

impl Default for QosClass {
    fn default() -> Self {
        QosClass::Default
    }
}

impl From<u32> for QosClass {
    fn from(v: u32) -> Self {
        unsafe { mem::transmute(v) }
    }
}

impl QosClass {
    /// Returns the requested QOS class of the current thread.
    pub fn current() -> Self {
        unsafe { qos_class_self() }.into()
    }

    /// Returns the initial requested QOS class of the main thread.
    pub fn main() -> Self {
        unsafe { qos_class_main() }.into()
    }
}
