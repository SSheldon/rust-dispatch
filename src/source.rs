use ffi::*;
use libc::{uintptr_t};
use std::os::raw::{c_uint, c_int, c_ulong};

/// A `mach_port_t`.
pub type MachPort = c_uint;

/// A C file descriptor.
pub type FileDescriptor = c_int;

bitflags! {
    /// Flags for the MachSend source type.
    pub flags MachSendFlags: dispatch_source_mach_send_flags_t {
        /// If set, generates an event if the port destination's
        /// receive right is destroyed.
        const MACH_SEND_DEAD = DISPATCH_MACH_SEND_DEAD,
    }
}

bitflags! {
    /// Event flags for the MemoryPressure source type.
    pub flags MemoryPressureFlags: dispatch_source_memorypressure_flags_t {
        /// System memory pressure is normal.
        const MEMORYPRESSURE_NORMAL   = DISPATCH_MEMORYPRESSURE_NORMAL,
        /// System memory pressure warning.
        const MEMORYPRESSURE_WARN     = DISPATCH_MEMORYPRESSURE_WARN,
        /// System memory pressure is critical.
        const MEMORYPRESSURE_CRITICAL = DISPATCH_MEMORYPRESSURE_CRITICAL,
    }
}

bitflags! {
    /// Event flags for the Proc source type.
    pub flags ProcFlags: dispatch_source_proc_flags_t {
        /// The process exited.
        const PROC_EXIT   = DISPATCH_PROC_EXIT,
        /// The process forked.
        const PROC_FORK   = DISPATCH_PROC_FORK,
        /// The process exec'd.
        const PROC_EXEC   = DISPATCH_PROC_EXEC,
        /// The process received a signal.
        const PROC_SIGNAL = DISPATCH_PROC_SIGNAL,
    }
}

bitflags! {
    /// Flags for the Timer source type.
    pub flags TimerFlags: dispatch_source_timer_flags_t {
        /// Force stricter adherence to the specified leeway.
        const TIMER_STRICT = DISPATCH_TIMER_STRICT,
    }
}

bitflags! {
    /// Event flags for the Vnode source type.
    pub flags VnodeFlags: dispatch_source_vnode_flags_t {
        /// A filesystem object was deleted.
        const VNODE_DELETE = DISPATCH_VNODE_DELETE,
        /// The data of a filesystem object changed.
        const VNODE_WRITE  = DISPATCH_VNODE_WRITE,
        /// The size of a filesystem object changed.
        const VNODE_EXTEND = DISPATCH_VNODE_EXTEND,
        /// The attributes of a filesystem object changed.
        const VNODE_ATTRIB = DISPATCH_VNODE_ATTRIB,
        /// The link count of a filesystem object changed.
        const VNODE_LINK   = DISPATCH_VNODE_LINK,
        /// A filesystem object was renamed.
        const VNODE_RENAME = DISPATCH_VNODE_RENAME,
        /// A filesystem object was revoked.
        const VNODE_REVOKE = DISPATCH_VNODE_REVOKE,
    }
}

/// Types of dispatch sources.
pub trait SourceType {
    /// The underlying dispatch source configuration.
    fn as_raw(&self) -> (dispatch_source_type_t, uintptr_t, c_ulong);
}

macro_rules! impl_source_type {
    (@handle, $s:expr, @) => (0);
    (@handle, $s:expr, $handle:ident) => ($s.$handle as _);
    (@mask, $s:expr, @) => (0);
    (@mask, $s:expr, $mask:ident) => ($s.$mask.bits());
    ($t:ty, $dt:expr, $handle:tt, $mask:tt) => {
        impl SourceType for $t {
            fn as_raw(&self) -> (dispatch_source_type_t, uintptr_t, c_ulong) {
                ($dt,
                 impl_source_type!(@handle, self, $handle),
                 impl_source_type!(@mask, self, $mask))
            }
        }
    };
}

/// Arithmetic add accumulator source type.
#[derive(Copy, Clone)]
pub struct DataAdd;
impl_source_type!(DataAdd, DISPATCH_SOURCE_TYPE_DATA_ADD, @, @);

/// Bitwise or accumulator source type.
#[derive(Copy, Clone)]
pub struct DataOr;
impl_source_type!(DataOr, DISPATCH_SOURCE_TYPE_DATA_OR, @, @);

/// Mach message send readyness event source type.
#[derive(Copy, Clone)]
pub struct MachSend {
    /// The Mach port to send to.
    pub port: MachPort,
    /// Mach send event options.
    pub flags: MachSendFlags,
}
impl_source_type!(MachSend, DISPATCH_SOURCE_TYPE_MACH_SEND, port, flags);

/// Mach message receive readyness event source type.
#[derive(Copy, Clone)]
pub struct MachRecv {
    /// The Mach port to receive from.
    pub port: MachPort,
}
impl_source_type!(MachRecv, DISPATCH_SOURCE_TYPE_MACH_RECV, port, @);

/// System memory pressure event source type.
#[derive(Copy, Clone)]
pub struct MemoryPressure {
    /// Event mask.
    pub flags: MemoryPressureFlags,
}
impl_source_type!(MemoryPressure, DISPATCH_SOURCE_TYPE_MEMORYPRESSURE, @, flags);

/// Process lifecycle event source type.
#[derive(Copy, Clone)]
pub struct Proc {
    /// Process ID to receive events from.
    pub pid: ::libc::pid_t,
    /// Event mask.
    pub flags: ProcFlags,
}
impl_source_type!(Proc, DISPATCH_SOURCE_TYPE_PROC, pid, flags);

/// File read readness event source type.
#[derive(Copy, Clone)]
pub struct Read {
    /// The file descriptor to read from.
    pub fd: FileDescriptor,
}
impl_source_type!(Read, DISPATCH_SOURCE_TYPE_READ, fd, @);

/// Current process signal event source type.
#[derive(Copy, Clone)]
pub struct Signal {
    /// The signal number to receive events for.
    pub signal: c_int,
}
impl_source_type!(Signal, DISPATCH_SOURCE_TYPE_SIGNAL, signal, @);

/// Timer event source type.
#[derive(Copy, Clone)]
pub struct Timer {
    /// Timer event options.
    pub flags: TimerFlags,
}
impl_source_type!(Timer, DISPATCH_SOURCE_TYPE_TIMER, @, flags);

/// Filesystem vnode event source type.
#[derive(Copy, Clone)]
pub struct Vnode {
    /// The vnode to receive events from.
    pub fd: FileDescriptor,
    /// Event mask.
    pub flags: VnodeFlags,
}
impl_source_type!(Vnode, DISPATCH_SOURCE_TYPE_VNODE, fd, flags);

/// File write readyness event source type.
#[derive(Copy, Clone)]
pub struct Write {
    /// The file descriptor to write to.
    pub fd: FileDescriptor,
}
impl_source_type!(Write, DISPATCH_SOURCE_TYPE_WRITE, fd, @);

/// Types of sources that accumulate an integer value between events.
pub trait DataSourceType: SourceType {}
impl DataSourceType for DataAdd {}
impl DataSourceType for DataOr {}
