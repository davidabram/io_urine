use core::sync::atomic::{AtomicU32, Ordering};

use rustix::io_uring::io_cqring_offsets;

use crate::io_uring_cqe;

pub struct CompletionQueue {
    khead: *mut AtomicU32,
    ktail: *const AtomicU32,
    kring_mask: u32,
    kring_entries: u32,
    kflags: *const AtomicU32,
    koverflow: *const AtomicU32,
    cqe_ptr: *mut io_uring_cqe,
    head: AtomicU32,
    tail: AtomicU32,
}

impl CompletionQueue {
    #[must_use]
    pub unsafe fn new(cq_ptr: *mut u8, offsets: &io_cqring_offsets) -> Self {
        let khead = unsafe { cq_ptr.add(offsets.head as usize) as *mut AtomicU32 };
        let ktail = unsafe { cq_ptr.add(offsets.tail as usize) as *const AtomicU32 };
        let kflags = unsafe { cq_ptr.add(offsets.flags as usize) as *const AtomicU32 };
        let koverflow = unsafe { cq_ptr.add(offsets.overflow as usize) as *const AtomicU32 };
        let cqe_ptr = unsafe { cq_ptr.add(offsets.cqes as usize) as *mut io_uring_cqe };

        Self {
            khead,
            ktail,
            kring_mask: offsets.ring_mask,
            kring_entries: offsets.ring_entries,
            kflags,
            koverflow,
            cqe_ptr,
            head: AtomicU32::new(0),
            tail: AtomicU32::new(0),
        }
    }

    #[must_use]
    pub fn ring_mask(&self) -> u32 {
        self.kring_mask
    }

    #[must_use]
    pub fn ring_entries(&self) -> u32 {
        self.kring_entries
    }

    pub(crate) fn get_khead(&self) -> u32 {
        unsafe { (*self.khead).load(Ordering::Acquire) }
    }

    fn get_ktail(&self) -> u32 {
        unsafe { (*self.ktail).load(Ordering::Acquire) }
    }

    pub fn set_khead(&self, value: u32) {
        unsafe { (*self.khead).store(value, Ordering::Release) }
    }

    pub fn update_kernel_tail(&self) {
        let ktail = self.get_ktail();
        self.tail.store(ktail, Ordering::Release);
    }

    #[must_use]
    pub fn events_available(&self) -> u32 {
        let tail = self.tail.load(Ordering::Acquire);
        let head = self.head.load(Ordering::Relaxed);
        tail.wrapping_sub(head)
    }

    #[must_use]
    pub fn peek(&self) -> Option<&io_uring_cqe> {
        let tail = self.tail.load(Ordering::Acquire);
        let head = self.head.load(Ordering::Relaxed);

        if tail == head {
            return None;
        }

        let index = head & self.kring_mask;
        let cqe = unsafe { &*self.cqe_ptr.add(index as usize) };
        Some(cqe)
    }

    #[must_use]
    pub fn peek_mut(&mut self) -> Option<&mut io_uring_cqe> {
        let tail = self.tail.load(Ordering::Acquire);
        let head = self.head.load(Ordering::Relaxed);

        if tail == head {
            return None;
        }

        let index = head & self.kring_mask;
        let cqe = unsafe { &mut *self.cqe_ptr.add(index as usize) };
        Some(cqe)
    }

    pub fn advance(&mut self, count: u32) {
        let head = self.head.load(Ordering::Relaxed);
        self.head.store(head.wrapping_add(count), Ordering::Release);
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.events_available() == 0
    }

    #[must_use]
    pub fn overflow_count(&self) -> u32 {
        unsafe { (*self.koverflow).load(Ordering::Relaxed) }
    }

    pub(crate) fn cqe_ptr(&self) -> *mut io_uring_cqe {
        self.cqe_ptr
    }
}
