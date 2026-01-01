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
pub use io_uring::{IoUring, Probe, SetupBuilder};
pub use mmap::RwMmap;
pub use sq::SubmissionQueue;
pub use sqe::{
    sqe_flags, Accept, Connect, Iovec, MsgHdr, Recv, RecvMsg, Send, SendMsg, Shutdown, SqeFlags,
};

pub const IORING_SETUP_IOPOLL: u32 = 1 << 0;
pub const IORING_SETUP_SQPOLL: u32 = 1 << 1;
pub const IORING_SETUP_SQ_AFF: u32 = 1 << 2;
pub const IORING_SETUP_CQSIZE: u32 = 1 << 3;
pub const IORING_SETUP_CLAMP: u32 = 1 << 4;
pub const IORING_SETUP_ATTACH_WQ: u32 = 1 << 5;
pub const IORING_SETUP_R_DISABLED: u32 = 1 << 6;
pub const IORING_SETUP_SUBMIT_ALL: u32 = 1 << 7;
pub const IORING_SETUP_COOP_TASKRUN: u32 = 1 << 8;
pub const IORING_SETUP_TASKRUN_FLAG: u32 = 1 << 9;
pub const IORING_SETUP_SQE128: u32 = 1 << 10;
pub const IORING_SETUP_CQE32: u32 = 1 << 11;

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

// Async cancel flags
pub const IORING_ASYNC_CANCEL_ALL: u32 = 1 << 0;
pub const IORING_ASYNC_CANCEL_ANY: u32 = 1 << 1;
pub const IORING_ASYNC_CANCEL_FD: u32 = 1 << 2;

// Buffer ring flags
pub const IORING_SETUP_BUFFER_RING: u64 = 1 << 3;

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

// Advanced registration features - restriction management
pub const IORING_REGISTER_RESTRICTIONS: u32 = 1 << 0;
pub const IORING_REGISTER_RESTRICTION_REGISTER_OP: u32 = 1 << 1;
pub const IORING_REGISTER_RESTRICTION_SQE_FLAGS: u32 = 1 << 2;

// Argument restriction flags
pub const IORING_REGISTER_RESTRICTION_ARG: u32 = 1 << 10;
pub const IORING_REGISTER_RESTRICTION_SQE_GROUP: u32 = 1 << 11;

// io_uring feature flags (IORING_FEAT_*)
pub const IORING_FEAT_SINGLE_MMAP: u32 = 1 << 0;
pub const IORING_FEAT_NODROP: u32 = 1 << 1;
pub const IORING_FEAT_SUBMIT_STABLE: u32 = 1 << 2;
pub const IORING_FEAT_RW_CUR_POS: u32 = 1 << 3;
pub const IORING_FEAT_CUR_PERSONALITY: u32 = 1 << 4;
pub const IORING_FEAT_FAST_POLL: u32 = 1 << 5;
pub const IORING_FEAT_POLL_32BITS: u32 = 1 << 6;
pub const IORING_FEAT_SQPOLL_FIXED: u32 = 1 << 7;
pub const IORING_FEAT_EXT_ARG: u32 = 1 << 8;
pub const IORING_FEAT_NATIVE_WORKERS: u32 = 1 << 9;
pub const IORING_FEAT_RSRC_TAGS: u32 = 1 << 10;
pub const IORING_FEAT_CQE_SKIP: u32 = 1 << 11;
pub const IORING_FEAT_LINKED_FILE: u32 = 1 << 12;
pub const IORING_FEAT_REG_REG_RING: u32 = 1 << 13;

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

// Socket types
pub const SOCK_STREAM: i32 = 1;
pub const SOCK_DGRAM: i32 = 2;
pub const SOCK_RAW: i32 = 3;
pub const SOCK_RDM: i32 = 4;
pub const SOCK_SEQPACKET: i32 = 5;
pub const SOCK_NONBLOCK: i32 = 0o0004000;
pub const SOCK_CLOEXEC: i32 = 0o2000000;

// Address families
pub const AF_UNSPEC: i32 = 0;
pub const AF_UNIX: i32 = 1;
pub const AF_INET: i32 = 2;
pub const AF_INET6: i32 = 10;
pub const AF_NETLINK: i32 = 16;

// Protocol families
pub const PF_UNSPEC: i32 = AF_UNSPEC;
pub const PF_UNIX: i32 = AF_UNIX;
pub const PF_INET: i32 = AF_INET;
pub const PF_INET6: i32 = AF_INET6;
pub const PF_NETLINK: i32 = AF_NETLINK;

// Message flags
pub const MSG_OOB: i32 = 0x0001;
pub const MSG_PEEK: i32 = 0x0002;
pub const MSG_DONTROUTE: i32 = 0x0004;
pub const MSG_CTRUNC: i32 = 0x0008;
pub const MSG_PROXY: i32 = 0x0010;
pub const MSG_TRUNC: i32 = 0x0020;
pub const MSG_DONTWAIT: i32 = 0x0040;
pub const MSG_EOR: i32 = 0x0080;
pub const MSG_WAITALL: i32 = 0x0100;
pub const MSG_FIN: i32 = 0x0200;
pub const MSG_SYN: i32 = 0x0400;
pub const MSG_CONFIRM: i32 = 0x0800;
pub const MSG_RST: i32 = 0x1000;
pub const MSG_ERRQUEUE: i32 = 0x2000;
pub const MSG_NOSIGNAL: i32 = 0x4000;
pub const MSG_MORE: i32 = 0x8000;
pub const MSG_WAITFORONE: i32 = 0x10000;
pub const MSG_ZEROCOPY: i32 = 0x40000000;
pub const MSG_BATCH: i32 = 0x40000;

// Socket level
pub const SOL_SOCKET: i32 = 1;

// Shutdown flags
pub const SHUT_RD: i32 = 0;
pub const SHUT_WR: i32 = 1;
pub const SHUT_RDWR: i32 = 2;

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

// Restriction structures for advanced registration
#[repr(C)]
#[derive(Debug, Clone, Default)]
pub struct Restriction {
    pub opcode: u16,
    pub flags: u32,
    pub resv: [u32; 2],
}

#[repr(C)]
#[derive(Debug, Clone, Default)]
pub struct Restrictions {
    pub resv: u32,
    pub allowed_ops: Option<&'static [Restriction]>,
    pub disallowed_ops: Option<&'static [Restriction]>,
    pub allowed_sqe_flags: Option<u32>,
    pub allowed_file_flags: Option<u32>,
    pub registerd_files: Option<Vec<i32>>,
}

// Per-buffer ring entry
#[repr(C)]
#[derive(Debug, Clone, Default)]
pub struct PbufRingEntry {
    pub user_addr: u64,
    pub user_data: u64,
    pub len: u16,
    pub bgid: u16,
    pub bid: u16,
}

// Per-buffer ring registration arguments
#[repr(C)]
#[derive(Debug, Clone, Default)]
pub struct PbufRingReg {
    pub ring_addr: u64,
    pub ring_len: u32,
    pub bgid: u16,
    pub entry_count: u32,
    pub entry_size: u32,
}

// Worker thread configuration arguments
#[repr(C)]
#[derive(Debug, Clone, Default)]
pub struct IowqMaxWorkers {
    pub count: u32,
    val: u32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct io_uring_getevents_arg {
    pub mask: u32,
    pub pad: u32,
    pub ts: u64,
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
