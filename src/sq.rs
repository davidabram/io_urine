use core::sync::atomic::{AtomicU32, Ordering};

use rustix::io_uring::io_sqring_offsets;

use crate::io_uring_sqe;

pub struct SubmissionQueue {
    khead: *const AtomicU32,
    ktail: *mut AtomicU32,
    kring_mask: u32,
    kring_entries: u32,
    kflags: *const AtomicU32,
    kdropped: *const AtomicU32,
    array: *mut u32,
    sqe_ptr: *mut io_uring_sqe,
    sqe_mask: u32,
    sqe_entries: u32,
    head: AtomicU32,
    tail: AtomicU32,
    // SQE cache for performance optimization
    sqe_cache: core::cell::RefCell<Vec<*mut io_uring_sqe>>,
}

impl SubmissionQueue {
    #[must_use]
    pub unsafe fn new(
        sq_ptr: *mut u8,
        offsets: &io_sqring_offsets,
        sqe_ptr: *mut io_uring_sqe,
        sq_entries: u32,
    ) -> Self {
        let khead = unsafe { sq_ptr.add(offsets.head as usize) as *const AtomicU32 };
        let ktail = unsafe { sq_ptr.add(offsets.tail as usize) as *mut AtomicU32 };
        let kflags = unsafe { sq_ptr.add(offsets.flags as usize) as *const AtomicU32 };
        let kdropped = unsafe { sq_ptr.add(offsets.dropped as usize) as *const AtomicU32 };
        let array = unsafe { sq_ptr.add(offsets.array as usize) as *mut u32 };

        let kring_mask = offsets.ring_mask;
        let kring_entries = offsets.ring_entries;

        Self {
            khead,
            ktail,
            kring_mask,
            kring_entries,
            kflags,
            kdropped,
            array,
            sqe_ptr,
            sqe_mask: sq_entries - 1,
            sqe_entries: sq_entries,
            head: AtomicU32::new(0),
            tail: AtomicU32::new(0),
            sqe_cache: core::cell::RefCell::new(Vec::new()),
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

    fn get_khead(&self) -> u32 {
        unsafe { (*self.khead).load(Ordering::Acquire) }
    }

    fn get_ktail(&self) -> u32 {
        unsafe { (*self.ktail).load(Ordering::Acquire) }
    }

    fn set_ktail(&self, value: u32) {
        unsafe { (*self.ktail).store(value, Ordering::Release) }
    }

    #[must_use]
    pub fn space_left(&self) -> u32 {
        let tail = self.tail.load(Ordering::Acquire);
        let head = self.head.load(Ordering::Relaxed);
        self.kring_entries - tail.wrapping_sub(head)
    }

    #[must_use]
    pub fn peek_sqe(&mut self) -> Option<&mut io_uring_sqe> {
        let tail = self.tail.load(Ordering::Acquire);
        let head = self.head.load(Ordering::Relaxed);

        if tail == head.wrapping_add(self.kring_entries) {
            return None;
        }

        let index = tail & self.kring_mask;

        // Tell the kernel which SQE entry we filled.
        unsafe {
            core::ptr::write_volatile(self.array.add(index as usize), index);
        }

        self.tail.store(tail.wrapping_add(1), Ordering::Release);

        let sqe = unsafe { &mut *self.sqe_ptr.add(index as usize) };
        Some(sqe)
    }

    pub fn advance(&mut self, count: u32) {
        let tail = self.tail.load(Ordering::Acquire);
        self.tail.store(tail.wrapping_add(count), Ordering::Release);
    }

    #[must_use]
    pub fn needs_flush(&self) -> bool {
        unsafe { (*self.kflags).load(Ordering::Relaxed) & crate::IORING_SQ_NEED_WAKEUP != 0 }
    }

    #[must_use]
    pub fn is_full(&self) -> bool {
        self.space_left() == 0
    }

    pub fn write_sqe(&mut self, sqe: &io_uring_sqe) {
        let tail = self.tail.load(Ordering::Acquire);
        let index = tail & self.kring_mask;
        let array_index = tail & self.kring_mask;

        unsafe {
            let target = self.sqe_ptr.add(index as usize);
            (*target).opcode = sqe.opcode;
            (*target).flags = sqe.flags;
            (*target).ioprio = sqe.ioprio;
            (*target).fd = sqe.fd;
            (*target).off = sqe.off;
            (*target).addr = sqe.addr;
            (*target).len = sqe.len;
            (*target).rw_flags = sqe.rw_flags;
            (*target).user_data = sqe.user_data;
            (*target).buf_index = sqe.buf_index;
            (*target).personality = sqe.personality;
            (*target).splice_fd_in = sqe.splice_fd_in;
            (*target).addr3 = sqe.addr3;
            (*target).__pad2 = sqe.__pad2;
        }
        unsafe {
            core::ptr::write_volatile(self.array.add(array_index as usize), index);
        }

        let new_tail = tail.wrapping_add(1);
        self.tail.store(new_tail, Ordering::Release);
    }

    pub fn update_kernel_tail(&self) -> u32 {
        let tail = self.tail.load(Ordering::Acquire);
        let head = self.head.load(Ordering::Relaxed);
        let to_submit = tail.wrapping_sub(head);
        self.set_ktail(tail);
        to_submit
    }

    pub fn update_from_kernel(&self) {
        let khead = self.get_khead();
        self.head.store(khead, Ordering::Release);
    }

    /// Reclaim a completed SQE back to the cache for reuse
    ///
    /// This method should be called after an operation is completed
    /// to avoid the overhead of SQE initialization for frequent operations.
    pub fn reclaim_sqe(&self, sqe_ptr: *mut io_uring_sqe) {
        // SAFETY: sqe_ptr must be a valid pointer within the SQE array
        // and not currently in use by the kernel
        unsafe {
            // Reset the SQE to default values for reuse
            (*sqe_ptr) = io_uring_sqe::default();
            // Add to cache for later reuse
            self.sqe_cache.borrow_mut().push(sqe_ptr);
        }
    }

    /// Get a cached SQE if available, or return None if cache is empty
    pub fn get_cached_sqe(&self) -> Option<*mut io_uring_sqe> {
        self.sqe_cache.borrow_mut().pop()
    }

    /// Clear the SQE cache
    pub fn clear_sqe_cache(&self) {
        self.sqe_cache.borrow_mut().clear();
    }

    /// Get the number of cached SQEs
    pub fn cached_sqe_count(&self) -> usize {
        self.sqe_cache.borrow().len()
    }
}
