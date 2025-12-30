use core::ffi::c_void;
use core::ptr::NonNull;

use rustix::fd::{AsFd, BorrowedFd, RawFd};
use rustix::io::{self, Errno};
use rustix::mm::{MapFlags, ProtFlags};

use crate::err::InitError;

pub struct RwMmap {
    ptr: NonNull<c_void>,
    size: usize,
    writable: bool,
}

impl RwMmap {
    pub fn new(fd: RawFd, offset: u64, size: usize, writable: bool) -> Result<Self, InitError> {
        let prot = if writable {
            ProtFlags::READ | ProtFlags::WRITE
        } else {
            ProtFlags::READ
        };

        let flags = MapFlags::SHARED;

        // SAFETY: mmap is safe to call with these parameters for io_uring ring setup
        let addr = unsafe {
            rustix::mm::mmap(
                core::ptr::null_mut(),
                size,
                prot,
                flags,
                BorrowedFd::<'_>::borrow_raw(fd),
                offset,
            )
        }
        .map_err(InitError::MmapFailed)?;

        Ok(Self {
            ptr: NonNull::new(addr).ok_or(InitError::MmapFailed(Errno::INVAL))?,
            size,
            writable,
        })
    }

    #[must_use]
    pub fn as_ptr(&self) -> *mut c_void {
        self.ptr.as_ptr()
    }

    #[must_use]
    pub fn as_slice(&self, offset: usize, size: usize) -> &[u8] {
        // SAFETY: bounds checking is caller's responsibility
        unsafe {
            let base = self.ptr.as_ptr() as *const u8;
            core::slice::from_raw_parts(base.add(offset), size)
        }
    }

    #[must_use]
    pub fn as_slice_mut(&mut self, offset: usize, size: usize) -> &mut [u8] {
        // SAFETY: bounds checking is caller's responsibility, writable flag ensures we can mutate
        unsafe {
            let base = self.ptr.as_ptr() as *mut u8;
            core::slice::from_raw_parts_mut(base.add(offset), size)
        }
    }

    #[must_use]
    pub fn size(&self) -> usize {
        self.size
    }

    #[must_use]
    pub fn is_writable(&self) -> bool {
        self.writable
    }
}

impl Drop for RwMmap {
    fn drop(&mut self) {
        // SAFETY: ptr is valid and was obtained from mmap
        // munmap errors are ignored as there's nothing we can do about them in Drop
        let _ = unsafe { rustix::mm::munmap(self.ptr.as_ptr(), self.size) };
    }
}
