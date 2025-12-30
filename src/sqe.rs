use core::ffi::c_void;

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
