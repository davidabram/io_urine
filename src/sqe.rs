use core::ffi::{c_void, CStr};

use crate::{
    io_uring_sqe, PrepSqe, PrepSqeMut, IORING_OP_NOP, IOSQE_ASYNC, IOSQE_BUFFER_SELECT,
    IOSQE_CQE_SKIP_SUCCESS, IOSQE_FIXED_FILE, IOSQE_IO_DRAIN, IOSQE_IO_HARDLINK, IOSQE_IO_LINK,
    IOSQE_SELECT_GROUP,
};

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Iovec {
    pub iov_base: *mut c_void,
    pub iov_len: usize,
}

impl Iovec {
    #[must_use]
    pub fn new(base: *mut c_void, len: usize) -> Self {
        Self {
            iov_base: base,
            iov_len: len,
        }
    }
}

// Message header for sendmsg/recvmsg operations
#[repr(C)]
#[derive(Debug)]
pub struct MsgHdr<'a> {
    pub msg_name: *mut c_void,
    pub msg_namelen: u32,
    pub msg_iov: &'a mut [Iovec],
    pub msg_control: *mut c_void,
    pub msg_controllen: u32,
    pub msg_flags: i32,
}

impl<'a> MsgHdr<'a> {
    #[must_use]
    pub fn new() -> Self {
        Self {
            msg_name: core::ptr::null_mut(),
            msg_namelen: 0,
            msg_iov: &mut [],
            msg_control: core::ptr::null_mut(),
            msg_controllen: 0,
            msg_flags: 0,
        }
    }

    #[must_use]
    pub fn with_addr(addr: &'a mut [u8]) -> Self {
        Self {
            msg_name: addr.as_mut_ptr() as *mut c_void,
            msg_namelen: addr.len() as u32,
            msg_iov: &mut [],
            msg_control: core::ptr::null_mut(),
            msg_controllen: 0,
            msg_flags: 0,
        }
    }

    #[must_use]
    pub fn with_iov(iov: &'a mut [Iovec]) -> Self {
        Self {
            msg_name: core::ptr::null_mut(),
            msg_namelen: 0,
            msg_iov: iov,
            msg_control: core::ptr::null_mut(),
            msg_controllen: 0,
            msg_flags: 0,
        }
    }

    #[must_use]
    pub fn with_addr_and_iov(addr: &'a mut [u8], iov: &'a mut [Iovec]) -> Self {
        Self {
            msg_name: addr.as_mut_ptr() as *mut c_void,
            msg_namelen: addr.len() as u32,
            msg_iov: iov,
            msg_control: core::ptr::null_mut(),
            msg_controllen: 0,
            msg_flags: 0,
        }
    }

    pub fn set_control(&mut self, control: *mut c_void, controllen: u32) {
        self.msg_control = control;
        self.msg_controllen = controllen;
    }

    pub fn set_flags(&mut self, flags: i32) {
        self.msg_flags = flags;
    }
}

#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum SqeFlags {
    FixedFile = IOSQE_FIXED_FILE,
    IoDrain = IOSQE_IO_DRAIN,
    IoLink = IOSQE_IO_LINK,
    IoHardlink = IOSQE_IO_HARDLINK,
    Async = IOSQE_ASYNC,
    BufferSelect = IOSQE_BUFFER_SELECT,
    CqeSkipSuccess = IOSQE_CQE_SKIP_SUCCESS,
}

impl SqeFlags {
    #[must_use]
    pub fn bits(self) -> u8 {
        self as u8
    }
}

#[must_use]
pub fn sqe_flags() -> SqeFlagsBuilder {
    SqeFlagsBuilder(0)
}

pub struct SqeFlagsBuilder(u8);

impl SqeFlagsBuilder {
    #[must_use]
    pub fn with(mut self, flag: SqeFlags) -> Self {
        self.0 |= flag.bits();
        self
    }

    #[must_use]
    pub fn bits(self) -> u8 {
        self.0
    }
}

pub struct Nop;

impl PrepSqe for Nop {
    fn prep(&self, sqe: &mut io_uring_sqe) {
        sqe.opcode = IORING_OP_NOP;
    }
}

pub struct Readv<'a> {
    fd: i32,
    iovec: &'a [Iovec],
    offset: u64,
    flags: u32,
}

impl<'a> Readv<'a> {
    #[must_use]
    pub fn new(fd: i32, iovec: &'a [Iovec], offset: u64) -> Self {
        Self {
            fd,
            iovec,
            offset,
            flags: 0,
        }
    }

    pub fn set_flags(&mut self, flags: u32) {
        self.flags = flags;
    }
}

impl PrepSqe for Readv<'_> {
    fn prep(&self, sqe: &mut io_uring_sqe) {
        sqe.opcode = crate::IORING_OP_READV;
        sqe.fd = self.fd;
        sqe.off = self.offset;
        sqe.addr = self.iovec.as_ptr() as u64;
        sqe.len = self.iovec.len() as u32;
        sqe.rw_flags = self.flags as i32;
    }
}

pub struct Writev<'a> {
    fd: i32,
    iovec: &'a [Iovec],
    offset: u64,
    flags: u32,
}

impl<'a> Writev<'a> {
    #[must_use]
    pub fn new(fd: i32, iovec: &'a [Iovec], offset: u64) -> Self {
        Self {
            fd,
            iovec,
            offset,
            flags: 0,
        }
    }

    pub fn set_flags(&mut self, flags: u32) {
        self.flags = flags;
    }
}

impl PrepSqe for Writev<'_> {
    fn prep(&self, sqe: &mut io_uring_sqe) {
        sqe.opcode = crate::IORING_OP_WRITEV;
        sqe.fd = self.fd;
        sqe.off = self.offset;
        sqe.addr = self.iovec.as_ptr() as u64;
        sqe.len = self.iovec.len() as u32;
        sqe.rw_flags = self.flags as i32;
    }
}

pub struct Read<'a> {
    fd: i32,
    buf: &'a mut [u8],
    offset: u64,
}

impl<'a> Read<'a> {
    #[must_use]
    pub fn new(fd: i32, buf: &'a mut [u8], offset: u64) -> Self {
        Self { fd, buf, offset }
    }
}

impl PrepSqeMut for Read<'_> {
    fn prep(&mut self, sqe: &mut io_uring_sqe) {
        sqe.opcode = crate::IORING_OP_READ;
        sqe.fd = self.fd;
        sqe.off = self.offset;
        sqe.addr = self.buf.as_mut_ptr() as u64;
        sqe.len = self.buf.len() as u32;
    }
}

pub struct ReadFixed<'a> {
    fd: i32,
    buf: &'a mut [u8],
    offset: u64,
    buf_index: u16,
}

impl<'a> ReadFixed<'a> {
    #[must_use]
    pub fn new(fd: i32, buf: &'a mut [u8], offset: u64, buf_index: u16) -> Self {
        Self {
            fd,
            buf,
            offset,
            buf_index,
        }
    }
}

impl PrepSqeMut for ReadFixed<'_> {
    fn prep(&mut self, sqe: &mut io_uring_sqe) {
        sqe.opcode = crate::IORING_OP_READ_FIXED;
        sqe.fd = self.fd;
        sqe.off = self.offset;
        sqe.addr = self.buf.as_mut_ptr() as u64;
        sqe.len = self.buf.len() as u32;
        sqe.buf_index = self.buf_index;
    }
}

pub struct WriteFixed<'a> {
    fd: i32,
    buf: &'a [u8],
    offset: u64,
    buf_index: u16,
}

impl<'a> WriteFixed<'a> {
    #[must_use]
    pub fn new(fd: i32, buf: &'a [u8], offset: u64, buf_index: u16) -> Self {
        Self {
            fd,
            buf,
            offset,
            buf_index,
        }
    }
}

impl PrepSqe for WriteFixed<'_> {
    fn prep(&self, sqe: &mut io_uring_sqe) {
        sqe.opcode = crate::IORING_OP_WRITE_FIXED;
        sqe.fd = self.fd;
        sqe.off = self.offset;
        sqe.addr = self.buf.as_ptr() as u64;
        sqe.len = self.buf.len() as u32;
        sqe.buf_index = self.buf_index;
    }
}

pub struct Write<'a> {
    fd: i32,
    buf: &'a [u8],
    offset: u64,
}

impl<'a> Write<'a> {
    #[must_use]
    pub fn new(fd: i32, buf: &'a [u8], offset: u64) -> Self {
        Self { fd, buf, offset }
    }
}

impl PrepSqe for Write<'_> {
    fn prep(&self, sqe: &mut io_uring_sqe) {
        sqe.opcode = crate::IORING_OP_WRITE;
        sqe.fd = self.fd;
        sqe.off = self.offset;
        sqe.addr = self.buf.as_ptr() as u64;
        sqe.len = self.buf.len() as u32;
    }
}

pub struct Fsync {
    fd: i32,
    flags: u32,
}

impl Fsync {
    #[must_use]
    pub fn new(fd: i32) -> Self {
        Self { fd, flags: 0 }
    }

    pub fn set_flags(&mut self, flags: u32) {
        self.flags = flags;
    }
}

impl PrepSqe for Fsync {
    fn prep(&self, sqe: &mut io_uring_sqe) {
        sqe.opcode = crate::IORING_OP_FSYNC;
        sqe.fd = self.fd;
        sqe.rw_flags = self.flags as i32;
    }
}

pub struct Close {
    fd: i32,
}

impl Close {
    #[must_use]
    pub fn new(fd: i32) -> Self {
        Self { fd }
    }
}

impl PrepSqe for Close {
    fn prep(&self, sqe: &mut io_uring_sqe) {
        sqe.opcode = crate::IORING_OP_CLOSE;
        sqe.fd = self.fd;
    }
}

pub struct OpenAt<'a> {
    dirfd: i32,
    path: &'a CStr,
    flags: u32,
    mode: u32,
}

impl<'a> OpenAt<'a> {
    #[must_use]
    pub fn new(dirfd: i32, path: &'a CStr, flags: u32, mode: u32) -> Self {
        Self {
            dirfd,
            path,
            flags,
            mode,
        }
    }

    pub fn set_flags(&mut self, flags: u32) {
        self.flags = flags;
    }
}

impl PrepSqe for OpenAt<'_> {
    fn prep(&self, sqe: &mut io_uring_sqe) {
        sqe.opcode = crate::IORING_OP_OPENAT;
        sqe.fd = self.dirfd;
        sqe.off = 0;
        sqe.addr = self.path.as_ptr() as u64;
        sqe.len = self.mode;
        sqe.rw_flags = self.flags as i32;
    }
}

pub struct CloseDirect {
    file_index: u32,
}

impl CloseDirect {
    #[must_use]
    pub fn new(file_index: u32) -> Self {
        Self { file_index }
    }
}

impl PrepSqe for CloseDirect {
    fn prep(&self, sqe: &mut io_uring_sqe) {
        sqe.opcode = crate::IORING_OP_CLOSE;
        sqe.fd = 0;
        sqe.off = 0;
        sqe.addr = 0;
        sqe.len = 0;

        // Encode the fixed file index as "index + 1" (0 means none).
        sqe.splice_fd_in = self.file_index.wrapping_add(1) as i32;
    }
}

pub struct Statx<'a> {
    dirfd: i32,
    path: &'a CStr,
    flags: u32,
    mask: u32,
    statxbuf: &'a mut rustix::fs::Statx,
}

impl<'a> Statx<'a> {
    #[must_use]
    pub fn new(
        dirfd: i32,
        path: &'a CStr,
        flags: u32,
        mask: u32,
        statxbuf: &'a mut rustix::fs::Statx,
    ) -> Self {
        Self {
            dirfd,
            path,
            flags,
            mask,
            statxbuf,
        }
    }

    pub fn set_flags(&mut self, flags: u32) {
        self.flags = flags;
    }
}

impl PrepSqeMut for Statx<'_> {
    fn prep(&mut self, sqe: &mut io_uring_sqe) {
        sqe.opcode = crate::IORING_OP_STATX;
        sqe.fd = self.dirfd;
        sqe.off = (&mut *self.statxbuf as *mut rustix::fs::Statx) as u64;
        sqe.addr = self.path.as_ptr() as u64;
        sqe.len = self.mask;
        sqe.rw_flags = self.flags as i32;
    }
}

pub struct Fallocate {
    fd: i32,
    mode: u32,
    offset: u64,
    len: u64,
}

impl Fallocate {
    #[must_use]
    pub fn new(fd: i32, mode: u32, offset: u64, len: u64) -> Self {
        Self {
            fd,
            mode,
            offset,
            len,
        }
    }

    pub fn set_mode(&mut self, mode: u32) {
        self.mode = mode;
    }
}

impl PrepSqe for Fallocate {
    fn prep(&self, sqe: &mut io_uring_sqe) {
        sqe.opcode = crate::IORING_OP_FALLOCATE;
        sqe.fd = self.fd;
        sqe.off = self.offset;
        sqe.addr = self.len;
        sqe.len = self.mode;
    }
}

pub struct Fadvise {
    fd: i32,
    offset: u64,
    len: u32,
    advice: u32,
}

impl Fadvise {
    #[must_use]
    pub fn new(fd: i32, offset: u64, len: u32, advice: u32) -> Self {
        Self {
            fd,
            offset,
            len,
            advice,
        }
    }

    pub fn set_advice(&mut self, advice: u32) {
        self.advice = advice;
    }
}

impl PrepSqe for Fadvise {
    fn prep(&self, sqe: &mut io_uring_sqe) {
        sqe.opcode = crate::IORING_OP_FADVISE;
        sqe.fd = self.fd;
        sqe.off = self.offset;
        sqe.addr = 0;
        sqe.len = self.len;
        sqe.rw_flags = self.advice as i32;
    }
}

pub struct Madvise {
    addr: *mut c_void,
    len: u32,
    advice: u32,
}

impl Madvise {
    #[must_use]
    pub fn new(addr: *mut c_void, len: u32, advice: u32) -> Self {
        Self { addr, len, advice }
    }

    pub fn set_advice(&mut self, advice: u32) {
        self.advice = advice;
    }
}

impl PrepSqe for Madvise {
    fn prep(&self, sqe: &mut io_uring_sqe) {
        sqe.opcode = crate::IORING_OP_MADVISE;
        sqe.fd = -1;
        sqe.off = 0;
        sqe.addr = self.addr as u64;
        sqe.len = self.len;
        sqe.rw_flags = self.advice as i32;
    }
}

pub struct UnlinkAt<'a> {
    dirfd: i32,
    path: &'a CStr,
    flags: u32,
}

impl<'a> UnlinkAt<'a> {
    #[must_use]
    pub fn new(dirfd: i32, path: &'a CStr, flags: u32) -> Self {
        Self { dirfd, path, flags }
    }

    pub fn set_flags(&mut self, flags: u32) {
        self.flags = flags;
    }
}

impl PrepSqe for UnlinkAt<'_> {
    fn prep(&self, sqe: &mut io_uring_sqe) {
        sqe.opcode = crate::IORING_OP_UNLINKAT;
        sqe.fd = self.dirfd;
        sqe.off = 0;
        sqe.addr = self.path.as_ptr() as u64;
        sqe.len = 0;
        sqe.rw_flags = self.flags as i32;
    }
}

pub struct RenameAt<'a> {
    olddirfd: i32,
    oldpath: &'a CStr,
    newdirfd: i32,
    newpath: &'a CStr,
    flags: u32,
}

impl<'a> RenameAt<'a> {
    #[must_use]
    pub fn new(
        olddirfd: i32,
        oldpath: &'a CStr,
        newdirfd: i32,
        newpath: &'a CStr,
        flags: u32,
    ) -> Self {
        Self {
            olddirfd,
            oldpath,
            newdirfd,
            newpath,
            flags,
        }
    }

    pub fn set_flags(&mut self, flags: u32) {
        self.flags = flags;
    }
}

impl PrepSqe for RenameAt<'_> {
    fn prep(&self, sqe: &mut io_uring_sqe) {
        sqe.opcode = crate::IORING_OP_RENAMEAT;
        sqe.fd = self.olddirfd;
        sqe.off = self.newpath.as_ptr() as u64;
        sqe.addr = self.oldpath.as_ptr() as u64;
        sqe.len = self.newdirfd as u32;
        sqe.rw_flags = self.flags as i32;
    }
}

pub struct MkdirAt<'a> {
    dirfd: i32,
    path: &'a CStr,
    mode: u32,
}

impl<'a> MkdirAt<'a> {
    #[must_use]
    pub fn new(dirfd: i32, path: &'a CStr, mode: u32) -> Self {
        Self { dirfd, path, mode }
    }
}

impl PrepSqe for MkdirAt<'_> {
    fn prep(&self, sqe: &mut io_uring_sqe) {
        sqe.opcode = crate::IORING_OP_MKDIRAT;
        sqe.fd = self.dirfd;
        sqe.off = 0;
        sqe.addr = self.path.as_ptr() as u64;
        sqe.len = self.mode;
    }
}

pub struct SymlinkAt<'a> {
    target: &'a CStr,
    newdirfd: i32,
    linkpath: &'a CStr,
}

impl<'a> SymlinkAt<'a> {
    #[must_use]
    pub fn new(target: &'a CStr, newdirfd: i32, linkpath: &'a CStr) -> Self {
        Self {
            target,
            newdirfd,
            linkpath,
        }
    }
}

impl PrepSqe for SymlinkAt<'_> {
    fn prep(&self, sqe: &mut io_uring_sqe) {
        sqe.opcode = crate::IORING_OP_SYMLINKAT;
        sqe.fd = self.newdirfd;
        sqe.off = self.linkpath.as_ptr() as u64;
        sqe.addr = self.target.as_ptr() as u64;
        sqe.len = 0;
    }
}

pub struct LinkAt<'a> {
    olddirfd: i32,
    oldpath: &'a CStr,
    newdirfd: i32,
    newpath: &'a CStr,
    flags: u32,
}

impl<'a> LinkAt<'a> {
    #[must_use]
    pub fn new(
        olddirfd: i32,
        oldpath: &'a CStr,
        newdirfd: i32,
        newpath: &'a CStr,
        flags: u32,
    ) -> Self {
        Self {
            olddirfd,
            oldpath,
            newdirfd,
            newpath,
            flags,
        }
    }

    pub fn set_flags(&mut self, flags: u32) {
        self.flags = flags;
    }
}

impl PrepSqe for LinkAt<'_> {
    fn prep(&self, sqe: &mut io_uring_sqe) {
        sqe.opcode = crate::IORING_OP_LINKAT;
        sqe.fd = self.olddirfd;
        sqe.off = self.newpath.as_ptr() as u64;
        sqe.addr = self.oldpath.as_ptr() as u64;
        sqe.len = self.newdirfd as u32;
        sqe.rw_flags = self.flags as i32;
    }
}

pub struct PollAdd {
    fd: i32,
    events: u16,
}

impl PollAdd {
    #[must_use]
    pub fn new(fd: i32, events: u16) -> Self {
        Self { fd, events }
    }
}

impl PrepSqe for PollAdd {
    fn prep(&self, sqe: &mut io_uring_sqe) {
        sqe.opcode = crate::IORING_OP_POLL_ADD;
        sqe.fd = self.fd;
        sqe.addr = self.events as u64;
    }
}

pub struct PollRemove {
    user_data: u64,
}

impl PollRemove {
    #[must_use]
    pub fn new(user_data: u64) -> Self {
        Self { user_data }
    }
}

impl PrepSqe for PollRemove {
    fn prep(&self, sqe: &mut io_uring_sqe) {
        sqe.opcode = crate::IORING_OP_POLL_REMOVE;
        sqe.addr = self.user_data;
    }
}

pub struct Timeout<'a> {
    ts: &'a crate::Timespec,
    count: u32,
    flags: u32,
}

impl<'a> Timeout<'a> {
    #[must_use]
    pub fn new(ts: &'a crate::Timespec, count: u32, flags: u32) -> Self {
        Self { ts, count, flags }
    }

    #[must_use]
    pub fn relative(ts: &'a crate::Timespec) -> Self {
        Self::new(ts, 0, 0)
    }

    #[must_use]
    pub fn absolute(ts: &'a crate::Timespec) -> Self {
        Self::new(ts, 0, crate::IORING_TIMEOUT_ABS)
    }
}

impl PrepSqe for Timeout<'_> {
    fn prep(&self, sqe: &mut io_uring_sqe) {
        sqe.opcode = crate::IORING_OP_TIMEOUT;
        sqe.addr = self.ts as *const crate::Timespec as u64;
        sqe.len = self.count;
        sqe.rw_flags = self.flags as i32;
    }
}

pub struct TimeoutRemove {
    user_data: u64,
}

impl TimeoutRemove {
    #[must_use]
    pub fn new(user_data: u64) -> Self {
        Self { user_data }
    }
}

impl PrepSqe for TimeoutRemove {
    fn prep(&self, sqe: &mut io_uring_sqe) {
        sqe.opcode = crate::IORING_OP_TIMEOUT_REMOVE;
        sqe.addr = self.user_data;
    }
}

pub struct LinkTimeout<'a> {
    ts: &'a crate::Timespec,
    flags: u32,
}

impl<'a> LinkTimeout<'a> {
    #[must_use]
    pub fn new(ts: &'a crate::Timespec, flags: u32) -> Self {
        Self { ts, flags }
    }
}

impl PrepSqe for LinkTimeout<'_> {
    fn prep(&self, sqe: &mut io_uring_sqe) {
        sqe.opcode = crate::IORING_OP_LINK_TIMEOUT;
        sqe.addr = self.ts as *const crate::Timespec as u64;
        sqe.rw_flags = self.flags as i32;
    }
}

// Networking operations

pub struct Send<'a> {
    fd: i32,
    buf: &'a [u8],
    flags: i32,
}

impl<'a> Send<'a> {
    #[must_use]
    pub fn new(fd: i32, buf: &'a [u8], flags: i32) -> Self {
        Self { fd, buf, flags }
    }
}

impl PrepSqe for Send<'_> {
    fn prep(&self, sqe: &mut io_uring_sqe) {
        sqe.opcode = crate::IORING_OP_SEND;
        sqe.fd = self.fd;
        sqe.addr = self.buf.as_ptr() as u64;
        sqe.len = self.buf.len() as u32;
        sqe.rw_flags = self.flags;
    }
}

pub struct Recv<'a> {
    fd: i32,
    buf: &'a mut [u8],
    flags: i32,
}

impl<'a> Recv<'a> {
    #[must_use]
    pub fn new(fd: i32, buf: &'a mut [u8], flags: i32) -> Self {
        Self { fd, buf, flags }
    }
}

impl PrepSqeMut for Recv<'_> {
    fn prep(&mut self, sqe: &mut io_uring_sqe) {
        sqe.opcode = crate::IORING_OP_RECV;
        sqe.fd = self.fd;
        sqe.addr = self.buf.as_mut_ptr() as u64;
        sqe.len = self.buf.len() as u32;
        sqe.rw_flags = self.flags;
    }
}

pub struct SendMsg<'a> {
    fd: i32,
    msg: &'a MsgHdr<'a>,
    flags: i32,
}

impl<'a> SendMsg<'a> {
    #[must_use]
    pub fn new(fd: i32, msg: &'a MsgHdr<'a>, flags: i32) -> Self {
        Self { fd, msg, flags }
    }
}

impl PrepSqe for SendMsg<'_> {
    fn prep(&self, sqe: &mut io_uring_sqe) {
        sqe.opcode = crate::IORING_OP_SENDMSG;
        sqe.fd = self.fd;
        sqe.addr = (self.msg as *const MsgHdr) as u64;
        sqe.len = 1;
        sqe.rw_flags = self.flags;
    }
}

pub struct RecvMsg<'a> {
    fd: i32,
    msg: &'a mut MsgHdr<'a>,
    flags: i32,
}

impl<'a> RecvMsg<'a> {
    #[must_use]
    pub fn new(fd: i32, msg: &'a mut MsgHdr<'a>, flags: i32) -> Self {
        Self { fd, msg, flags }
    }
}

impl PrepSqeMut for RecvMsg<'_> {
    fn prep(&mut self, sqe: &mut io_uring_sqe) {
        sqe.opcode = crate::IORING_OP_RECVMSG;
        sqe.fd = self.fd;
        sqe.addr = (self.msg as *mut MsgHdr) as u64;
        sqe.len = 1;
        sqe.rw_flags = self.flags;
    }
}

// Connection management operations

pub struct Accept<'a> {
    fd: i32,
    addr: Option<&'a mut [u8]>,
    addrlen: Option<*mut u32>,
    flags: i32,
    file_index: u32,
}

impl<'a> Accept<'a> {
    #[must_use]
    pub fn new(fd: i32, flags: i32) -> Self {
        Self {
            fd,
            addr: None,
            addrlen: None,
            flags,
            file_index: 0,
        }
    }

    #[must_use]
    pub fn with_addr(fd: i32, addr: &'a mut [u8], addrlen: &'a mut u32, flags: i32) -> Self {
        Self {
            fd,
            addr: Some(addr),
            addrlen: Some(addrlen as *mut u32),
            flags,
            file_index: 0,
        }
    }

    #[must_use]
    pub fn with_file_index(fd: i32, file_index: u32, flags: i32) -> Self {
        Self {
            fd,
            addr: None,
            addrlen: None,
            flags,
            file_index,
        }
    }

    #[must_use]
    pub fn with_addr_and_file_index(
        fd: i32,
        addr: &'a mut [u8],
        addrlen: &'a mut u32,
        file_index: u32,
        flags: i32,
    ) -> Self {
        Self {
            fd,
            addr: Some(addr),
            addrlen: Some(addrlen as *mut u32),
            flags,
            file_index,
        }
    }
}

impl PrepSqeMut for Accept<'_> {
    fn prep(&mut self, sqe: &mut io_uring_sqe) {
        sqe.opcode = crate::IORING_OP_ACCEPT;
        sqe.fd = self.fd;

        match (self.addr.as_mut(), self.addrlen) {
            (Some(addr), Some(addrlen)) => {
                sqe.addr = addr.as_mut_ptr() as u64;
                // addr2 field is in union with off field
                // SAFETY: addrlen is a valid pointer to u32 provided by the caller
                unsafe {
                    sqe.off = *addrlen as u64;
                }
            }
            _ => {
                sqe.addr = 0;
                sqe.off = 0;
            }
        }

        sqe.len = 0;
        sqe.rw_flags = self.flags;
        sqe.splice_fd_in = self.file_index as i32;
    }
}

pub struct Connect<'a> {
    fd: i32,
    addr: &'a [u8],
    addrlen: u32,
}

impl<'a> Connect<'a> {
    #[must_use]
    pub fn new(fd: i32, addr: &'a [u8], addrlen: u32) -> Self {
        Self { fd, addr, addrlen }
    }
}

impl PrepSqe for Connect<'_> {
    fn prep(&self, sqe: &mut io_uring_sqe) {
        sqe.opcode = crate::IORING_OP_CONNECT;
        sqe.fd = self.fd;
        sqe.addr = self.addr.as_ptr() as u64;
        sqe.len = self.addrlen;
    }
}

pub struct Shutdown {
    fd: i32,
    how: i32,
}

impl Shutdown {
    #[must_use]
    pub fn new(fd: i32, how: i32) -> Self {
        Self { fd, how }
    }
}

impl PrepSqe for Shutdown {
    fn prep(&self, sqe: &mut io_uring_sqe) {
        sqe.opcode = crate::IORING_OP_SHUTDOWN;
        sqe.fd = self.fd;
        sqe.len = 0;
        sqe.rw_flags = self.how;
    }
}

// Advanced I/O operations

pub struct Splice {
    fd_in: i32,
    off_in: u64,
    fd_out: i32,
    off_out: u64,
    len: u32,
    flags: u32,
}

impl Splice {
    #[must_use]
    pub fn new(fd_in: i32, off_in: u64, fd_out: i32, off_out: u64, len: u32, flags: u32) -> Self {
        Self {
            fd_in,
            off_in,
            fd_out,
            off_out,
            len,
            flags,
        }
    }
}

impl PrepSqe for Splice {
    fn prep(&self, sqe: &mut io_uring_sqe) {
        sqe.opcode = crate::IORING_OP_SPLICE;
        sqe.fd = self.fd_out;
        sqe.off = self.off_out;
        sqe.len = self.len;
        sqe.splice_fd_in = self.fd_in;
        sqe.addr3 = self.off_in;
        sqe.rw_flags = self.flags as i32;
    }
}

pub struct Tee {
    fd_in: i32,
    fd_out: i32,
    len: u32,
    flags: u32,
}

impl Tee {
    #[must_use]
    pub fn new(fd_in: i32, fd_out: i32, len: u32, flags: u32) -> Self {
        Self {
            fd_in,
            fd_out,
            len,
            flags,
        }
    }
}

impl PrepSqe for Tee {
    fn prep(&self, sqe: &mut io_uring_sqe) {
        sqe.opcode = crate::IORING_OP_TEE;
        sqe.fd = self.fd_out;
        sqe.off = 0;
        sqe.len = self.len;
        sqe.splice_fd_in = self.fd_in;
        sqe.addr3 = 0;
        sqe.rw_flags = self.flags as i32;
    }
}

// Buffer allocation is done via PROVIE_BUFFERS with specific flags
// ALLOC_BUFFERS doesn't exist as a separate opcode in recent kernels

pub struct FreeBuffers {
    bgid: u16,
}

impl FreeBuffers {
    #[must_use]
    pub fn new(bgid: u16) -> Self {
        Self { bgid }
    }
}

impl PrepSqe for FreeBuffers {
    fn prep(&self, sqe: &mut io_uring_sqe) {
        sqe.opcode = crate::IORING_OP_REMOVE_BUFFERS;
        sqe.addr = self.bgid as u64;
        sqe.len = 0;
    }
}

pub struct ProvideBuffers {
    addr: *mut c_void,
    len: u32,
    bgid: u16,
    bid: u16,
    nbufs: u32,
}

impl ProvideBuffers {
    #[must_use]
    pub fn new(addr: *mut c_void, len: u32, bgid: u16, bid: u16, nbufs: u32) -> Self {
        Self {
            addr,
            len,
            bgid,
            bid,
            nbufs,
        }
    }
}

impl PrepSqe for ProvideBuffers {
    fn prep(&self, sqe: &mut io_uring_sqe) {
        sqe.opcode = crate::IORING_OP_PROVIDE_BUFFERS;
        sqe.addr = self.addr as u64;
        sqe.len = self.len;
        sqe.off = ((self.bgid as u64) << 32) | (self.bid as u64);
        sqe.buf_index = self.nbufs as u16;
    }
}

pub struct RemoveBuffers {
    bgid: u16,
    nr: u32,
}

impl RemoveBuffers {
    #[must_use]
    pub fn new(bgid: u16, nr: u32) -> Self {
        Self { bgid, nr }
    }
}

impl PrepSqe for RemoveBuffers {
    fn prep(&self, sqe: &mut io_uring_sqe) {
        sqe.opcode = crate::IORING_OP_REMOVE_BUFFERS;
        sqe.addr = self.bgid as u64;
        sqe.len = self.nr;
    }
}

pub struct AsyncCancel {
    user_data: u64,
    flags: u32,
}

impl AsyncCancel {
    #[must_use]
    pub fn new(user_data: u64, flags: u32) -> Self {
        Self { user_data, flags }
    }

    #[must_use]
    pub fn all() -> Self {
        Self {
            user_data: 0,
            flags: crate::IORING_ASYNC_CANCEL_ALL,
        }
    }

    #[must_use]
    pub fn any() -> Self {
        Self {
            user_data: 0,
            flags: crate::IORING_ASYNC_CANCEL_ANY,
        }
    }
}

impl PrepSqe for AsyncCancel {
    fn prep(&self, sqe: &mut io_uring_sqe) {
        sqe.opcode = crate::IORING_OP_ASYNC_CANCEL;
        sqe.addr = self.user_data;
        sqe.len = self.flags;
    }
}

pub struct MsgRing {
    fd: i32,
    user_data: u64,
    flags: u32,
    len: u32,
}

impl MsgRing {
    #[must_use]
    pub fn new(fd: i32, user_data: u64, flags: u32, len: u32) -> Self {
        Self {
            fd,
            user_data,
            flags,
            len,
        }
    }
}

impl PrepSqe for MsgRing {
    fn prep(&self, sqe: &mut io_uring_sqe) {
        sqe.opcode = crate::IORING_OP_MSG_RING;
        sqe.fd = self.fd;
        sqe.addr = self.user_data;
        sqe.len = self.len;
        sqe.rw_flags = self.flags as i32;
    }
}
