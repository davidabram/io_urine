#![warn(clippy::all, clippy::pedantic)]
#![allow(dead_code, unused_imports)]
#![allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]

use core::ffi::c_void;
use core::sync::atomic::AtomicU32;

use rustix::fd::RawFd;

pub mod cq;
pub mod cqe;
pub mod err;
pub mod io_uring;
pub mod mmap;
pub mod sq;
pub mod sqe;
#[cfg(test)]
mod tests;

pub use cq::CompletionQueue;
pub use cqe::CqeFlags;
pub use err::{EnterError, InitError, IoUringResult};
pub use io_uring::{IoUring, Probe};
pub use mmap::RwMmap;
pub use sq::SubmissionQueue;
pub use sqe::{Iovec, SqeFlags};

pub const IORING_SETUP_IOPOLL: u32 = 1 << 0;
pub const IORING_SETUP_SQPOLL: u32 = 1 << 1;
pub const IORING_SETUP_SQ_AFF: u32 = 1 << 2;
pub const IORING_SETUP_CQSIZE: u32 = 1 << 3;
pub const IORING_SETUP_CLAMP: u32 = 1 << 4;
pub const IORING_SETUP_ATTACH_WQ: u32 = 1 << 5;
pub const IORING_SETUP_R_DISABLED: u32 = 1 << 6;

pub const IORING_ENTER_GETEVENTS: u32 = 1 << 0;
pub const IORING_ENTER_SQ_WAKEUP: u32 = 1 << 1;
pub const IORING_ENTER_SQ_WAIT: u32 = 1 << 2;
pub const IORING_ENTER_EXT_ARG: u32 = 1 << 3;
pub const IORING_ENTER_REGISTERED_FD: u32 = 1 << 4;

pub const IORING_OFF_SQ_RING: u64 = 0;
pub const IORING_OFF_CQ_RING: u64 = 0x0800_0000;
pub const IORING_OFF_SQES: u64 = 0x1000_0000;

pub const AT_FDCWD: i32 = -100;

pub const IORING_OP_NOP: u8 = 0;
pub const IORING_OP_READV: u8 = 1;
pub const IORING_OP_WRITEV: u8 = 2;
pub const IORING_OP_FSYNC: u8 = 3;
pub const IORING_OP_READ_FIXED: u8 = 4;
pub const IORING_OP_WRITE_FIXED: u8 = 5;
pub const IORING_OP_POLL_ADD: u8 = 6;
pub const IORING_OP_POLL_REMOVE: u8 = 7;
pub const IORING_OP_SYNC_FILE_RANGE: u8 = 8;
pub const IORING_OP_SENDMSG: u8 = 9;
pub const IORING_OP_RECVMSG: u8 = 10;
pub const IORING_OP_TIMEOUT: u8 = 11;
pub const IORING_OP_TIMEOUT_REMOVE: u8 = 12;
pub const IORING_OP_ACCEPT: u8 = 13;
pub const IORING_OP_ASYNC_CANCEL: u8 = 14;
pub const IORING_OP_LINK_TIMEOUT: u8 = 15;
pub const IORING_OP_CONNECT: u8 = 16;
pub const IORING_OP_FALLOCATE: u8 = 17;
pub const IORING_OP_OPENAT: u8 = 18;
pub const IORING_OP_CLOSE: u8 = 19;
pub const IORING_OP_FILES_UPDATE: u8 = 20;
pub const IORING_OP_STATX: u8 = 21;
pub const IORING_OP_READ: u8 = 22;
pub const IORING_OP_WRITE: u8 = 23;
pub const IORING_OP_FADVISE: u8 = 24;
pub const IORING_OP_MADVISE: u8 = 25;
pub const IORING_OP_SEND: u8 = 26;
pub const IORING_OP_RECV: u8 = 27;
pub const IORING_OP_OPENAT2: u8 = 28;
pub const IORING_OP_EPOLL_CTL: u8 = 29;
pub const IORING_OP_SPLICE: u8 = 30;
pub const IORING_OP_PROVIDE_BUFFERS: u8 = 31;
pub const IORING_OP_REMOVE_BUFFERS: u8 = 32;
pub const IORING_OP_TEE: u8 = 33;
pub const IORING_OP_SHUTDOWN: u8 = 34;
pub const IORING_OP_RENAMEAT: u8 = 35;
pub const IORING_OP_UNLINKAT: u8 = 36;
pub const IORING_OP_MKDIRAT: u8 = 37;
pub const IORING_OP_SYMLINKAT: u8 = 38;
pub const IORING_OP_LINKAT: u8 = 39;
pub const IORING_OP_MSG_RING: u8 = 40;
pub const IORING_OP_FSETXATTR: u8 = 41;
pub const IORING_OP_SETXATTR: u8 = 42;
pub const IORING_OP_FGETXATTR: u8 = 43;
pub const IORING_OP_GETXATTR: u8 = 44;
pub const IORING_OP_SOCKET: u8 = 45;
pub const IORING_OP_URING_CMD: u8 = 46;
pub const IORING_OP_SEND_ZC: u8 = 47;
pub const IORING_OP_SENDMSG_ZC: u8 = 48;

// Timeout flags
pub const IORING_TIMEOUT_ABS: u32 = 1 << 0;

pub const IOSQE_FIXED_FILE: u8 = 1 << 0;
pub const IOSQE_IO_DRAIN: u8 = 1 << 1;
pub const IOSQE_IO_LINK: u8 = 1 << 2;
pub const IOSQE_IO_HARDLINK: u8 = 1 << 3;
pub const IOSQE_ASYNC: u8 = 1 << 4;
pub const IOSQE_BUFFER_SELECT: u8 = 1 << 5;
pub const IOSQE_CQE_SKIP_SUCCESS: u8 = 1 << 6;

pub const IOSQE_SELECT_GROUP: u8 = IOSQE_BUFFER_SELECT;

pub const IORING_CQE_F_BUFFER: u32 = 1 << 0;
pub const IORING_CQE_F_MORE: u32 = 1 << 1;
pub const IORING_CQE_F_SOCK_NONEMPTY: u32 = 1 << 2;
pub const IORING_CQE_F_TIMEOUT: u32 = 1 << 3;
pub const IORING_CQE_F_NOTIFICATION: u32 = 1 << 4;

pub const IORING_SQ_NEED_WAKEUP: u32 = 1 << 0;

pub const IORING_F_SQE128: u32 = 1 << 0;
pub const IORING_F_CQE32: u32 = 1 << 1;
pub const IORING_F_CQE32OPT: u32 = 1 << 2;
pub const IORING_F_SGID: u32 = 1 << 3;
pub const IORING_F_NO_MMAP: u32 = 1 << 4;
pub const IORING_F_SINGLE_MMAP: u32 = 1 << 5;
pub const IORING_F_SUBMIT_STABLE: u32 = 1 << 6;
pub const IORING_F_RW_BUF_NODROP: u32 = 1 << 7;
pub const IORING_F_HAVE_SENDZC: u32 = 1 << 8;
pub const IORING_F_RECVSEND_CACHE: u32 = 1 << 9;
pub const IORING_F_SOCKET_SENDRCV_NONE: u32 = 1 << 10;
pub const IORING_F_NATIVE_WORKERS: u32 = 1 << 11;
pub const IORING_F_REG_REG_RING: u32 = 1 << 12;

// Poll event flags
pub const POLLIN: u16 = 0x0001;
pub const POLLPRI: u16 = 0x0002;
pub const POLLOUT: u16 = 0x0004;
pub const POLLERR: u16 = 0x0008;
pub const POLLHUP: u16 = 0x0010;
pub const POLLNVAL: u16 = 0x0020;
pub const POLLRDNORM: u16 = 0x0040;
pub const POLLRDBAND: u16 = 0x0080;
pub const POLLWRNORM: u16 = 0x0100;
pub const POLLWRBAND: u16 = 0x0200;
pub const POLLMSG: u16 = 0x0400;
pub const POLLREMOVE: u16 = 0x1000;
pub const POLLTICK: u16 = 0x2000;

pub use rustix::io_uring::{io_cqring_offsets, io_sqring_offsets};

#[repr(C)]
#[derive(Debug, Default)]
pub struct io_uring_sqe {
    pub opcode: u8,
    pub flags: u8,
    pub ioprio: u16,
    pub fd: i32,
    pub off: u64,
    pub addr: u64,
    pub len: u32,
    pub rw_flags: i32,
    pub user_data: u64,
    pub buf_index: u16,
    pub personality: u16,
    pub splice_fd_in: i32,
    pub addr3: u64,
    pub(crate) __pad2: u64,
}

impl io_uring_sqe {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct io_uring_cqe {
    pub user_data: u64,
    pub res: i32,
    pub flags: u32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Timespec {
    pub tv_sec: i64,
    pub tv_nsec: i64,
}

impl Timespec {
    #[must_use]
    pub fn new(tv_sec: i64, tv_nsec: i64) -> Self {
        Self { tv_sec, tv_nsec }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct iovec {
    pub iov_base: *mut c_void,
    pub iov_len: usize,
}

impl iovec {
    #[must_use]
    pub fn new(base: *mut c_void, len: usize) -> Self {
        Self {
            iov_base: base,
            iov_len: len,
        }
    }
}

pub trait PrepSqe {
    fn prep(&self, sqe: &mut io_uring_sqe);
}

pub trait PrepSqeMut {
    fn prep(&mut self, sqe: &mut io_uring_sqe);
}

#[must_use]
pub fn sq_entries_available(tail: u32, head: u32, ring_entries: u32) -> u32 {
    ring_entries - tail.wrapping_sub(head)
}

#[must_use]
pub fn cq_entries_available(tail: u32, head: u32, ring_entries: u32) -> u32 {
    ring_entries - tail.wrapping_sub(head)
}
