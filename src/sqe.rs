use core::ffi::{c_void, CStr};

use crate::{
    io_uring_sqe, PrepSqe, PrepSqeMut, IORING_OP_NOP, IOSQE_ASYNC, IOSQE_FIXED_FILE,
    IOSQE_IO_DRAIN, IOSQE_IO_HARDLINK, IOSQE_IO_LINK, IOSQE_SELECT_GROUP,
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

#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum SqeFlags {
    FixedFile = IOSQE_FIXED_FILE,
    IoDrain = IOSQE_IO_DRAIN,
    IoLink = IOSQE_IO_LINK,
    IoHardlink = IOSQE_IO_HARDLINK,
    Async = IOSQE_ASYNC,
    SelectGroup = IOSQE_SELECT_GROUP,
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
