use core::ffi::{c_void, CStr};
use core::ptr::{null, null_mut};

use rustix::fd::{AsFd, AsRawFd, OwnedFd};
use rustix::io::{self, Errno};
use rustix::io_uring::{self, io_uring_files_update, sigset_t, IoringEnterFlags, IoringRegisterOp};

use crate::cq::CompletionQueue;
use crate::err::{EnterError, InitError};
use crate::mmap::RwMmap;
use crate::sq::SubmissionQueue;
use crate::{
    io_uring_sqe, Iovec, PrepSqe, PrepSqeMut, IORING_OFF_CQ_RING, IORING_OFF_SQES,
    IORING_OFF_SQ_RING, IORING_OP_NOP,
};

pub struct IoUring {
    fd: OwnedFd,
    _sq_mmap: RwMmap,
    _cq_mmap: RwMmap,
    _sqe_mmap: RwMmap,
    sq: SubmissionQueue,
    cq: CompletionQueue,
    // User data allocator for automatic user_data management
    next_user_data: core::sync::atomic::AtomicU64,
    free_user_data: core::cell::RefCell<Vec<u64>>,
}

const PROBE_OPS: usize = 128;

#[repr(C)]
#[derive(Debug)]
pub struct Probe {
    probe: io_uring::io_uring_probe,
    ops: [io_uring::io_uring_probe_op; PROBE_OPS],
}

impl Probe {
    #[must_use]
    fn new() -> Self {
        Self {
            probe: io_uring::io_uring_probe::default(),
            ops: [io_uring::io_uring_probe_op::default(); PROBE_OPS],
        }
    }

    #[must_use]
    fn ops_len(&self) -> usize {
        (self.probe.ops_len as usize).min(self.ops.len())
    }

    #[must_use]
    fn ops_slice(&self) -> &[io_uring::io_uring_probe_op] {
        &self.ops[..self.ops_len()]
    }

    #[must_use]
    pub fn opcode_supported(&self, opcode: u8) -> bool {
        self.ops_slice().iter().any(|op| {
            op.op as u8 == opcode && op.flags.contains(io_uring::IoringOpFlags::SUPPORTED)
        })
    }
}

impl IoUring {
    pub fn new(entries: u32) -> Result<Self, InitError> {
        let mut params = io_uring::io_uring_params::default();

        let sq_entries = if entries == 0 {
            32
        } else {
            entries.clamp(1, 4096)
        };
        let cq_entries = sq_entries;

        params.sq_entries = sq_entries;
        params.cq_entries = cq_entries;

        let fd = rustix::io_uring::io_uring_setup(sq_entries, &mut params)
            .map_err(InitError::SyscallFailed)?;

        let ring = Self::create_ring(fd, params)?;

        Ok(ring)
    }

    pub fn with_entries(sq_entries: u32, cq_entries: u32) -> Result<Self, InitError> {
        let mut params = io_uring::io_uring_params::default();

        params.sq_entries = sq_entries.clamp(1, 4096);
        params.cq_entries = cq_entries.clamp(1, 4096);

        let fd = rustix::io_uring::io_uring_setup(params.sq_entries, &mut params)
            .map_err(InitError::SyscallFailed)?;

        let ring = Self::create_ring(fd, params)?;

        Ok(ring)
    }

    fn create_ring(fd: OwnedFd, params: io_uring::io_uring_params) -> Result<Self, InitError> {
        let sq_ring_size = params.sq_off.array as usize
            + (params.sq_entries as usize * core::mem::size_of::<u32>());
        let cq_ring_size = params.cq_off.cqes as usize
            + (params.cq_entries as usize * core::mem::size_of::<crate::io_uring_cqe>());
        let sqe_size = (params.sq_entries as usize) * core::mem::size_of::<io_uring_sqe>();

        let sq_mmap = RwMmap::new(fd.as_raw_fd(), IORING_OFF_SQ_RING, sq_ring_size, true)?;
        let cq_mmap = RwMmap::new(fd.as_raw_fd(), IORING_OFF_CQ_RING, cq_ring_size, true)?;
        let sqe_mmap_mapping = RwMmap::new(fd.as_raw_fd(), IORING_OFF_SQES, sqe_size, true)?;

        let sq = unsafe {
            SubmissionQueue::new(
                sq_mmap.as_ptr() as *mut u8,
                &params.sq_off,
                sqe_mmap_mapping.as_ptr() as *mut io_uring_sqe,
                params.sq_entries,
            )
        };
        let cq = unsafe { CompletionQueue::new(cq_mmap.as_ptr() as *mut u8, &params.cq_off) };

        Ok(Self {
            fd,
            _sq_mmap: sq_mmap,
            _cq_mmap: cq_mmap,
            _sqe_mmap: sqe_mmap_mapping,
            sq,
            cq,
            next_user_data: core::sync::atomic::AtomicU64::new(1),
            free_user_data: core::cell::RefCell::new(Vec::new()),
        })
    }

    fn register(
        &self,
        opcode: IoringRegisterOp,
        arg: *const c_void,
        nr_args: u32,
    ) -> Result<u32, InitError> {
        // SAFETY: `io_uring_register` doesn't retain `arg`; it only reads it
        // for the duration of the syscall.
        unsafe { io_uring::io_uring_register(self.fd.as_fd(), opcode, arg, nr_args) }
            .map_err(InitError::RegisterFailed)
    }

    pub fn register_buffers(&self, iovecs: &[Iovec]) -> Result<(), InitError> {
        if iovecs.is_empty() {
            return Err(InitError::InvalidParameters);
        }

        let nr_args: u32 = iovecs
            .len()
            .try_into()
            .map_err(|_| InitError::InvalidParameters)?;

        self.register(
            IoringRegisterOp::RegisterBuffers,
            iovecs.as_ptr() as *const c_void,
            nr_args,
        )?;

        Ok(())
    }

    pub fn unregister_buffers(&self) -> Result<(), InitError> {
        self.register(IoringRegisterOp::UnregisterBuffers, null(), 0)?;
        Ok(())
    }

    pub fn register_files(&self, fds: &[i32]) -> Result<(), InitError> {
        if fds.is_empty() {
            return Err(InitError::InvalidParameters);
        }

        let nr_args: u32 = fds
            .len()
            .try_into()
            .map_err(|_| InitError::InvalidParameters)?;

        self.register(
            IoringRegisterOp::RegisterFiles,
            fds.as_ptr() as *const c_void,
            nr_args,
        )?;

        Ok(())
    }

    pub fn unregister_files(&self) -> Result<(), InitError> {
        self.register(IoringRegisterOp::UnregisterFiles, null(), 0)?;
        Ok(())
    }

    pub fn register_files_update(&self, offset: u32, fds: &[i32]) -> Result<(), InitError> {
        if fds.is_empty() {
            return Err(InitError::InvalidParameters);
        }

        let nr_args: u32 = fds
            .len()
            .try_into()
            .map_err(|_| InitError::InvalidParameters)?;
        let update = io_uring_files_update {
            offset,
            resv: 0,
            fds: fds.as_ptr() as u64,
        };

        self.register(
            IoringRegisterOp::RegisterFilesUpdate,
            (&update as *const io_uring_files_update).cast::<c_void>(),
            nr_args,
        )?;

        Ok(())
    }

    pub fn register_eventfd(&self, eventfd: i32) -> Result<(), InitError> {
        let eventfd = eventfd;
        self.register(
            IoringRegisterOp::RegisterEventfd,
            (&eventfd as *const i32).cast::<c_void>(),
            1,
        )?;
        Ok(())
    }

    pub fn unregister_eventfd(&self) -> Result<(), InitError> {
        self.register(IoringRegisterOp::UnregisterEventfd, null(), 0)?;
        Ok(())
    }

    pub fn register_eventfd_async(&self, eventfd: i32) -> Result<(), InitError> {
        let eventfd = eventfd;
        self.register(
            IoringRegisterOp::RegisterEventfdAsync,
            (&eventfd as *const i32).cast::<c_void>(),
            1,
        )?;
        Ok(())
    }

    pub fn probe(&self) -> Result<Probe, InitError> {
        let mut probe = Probe::new();
        let nr_args = PROBE_OPS as u32;
        let arg = &mut probe as *mut Probe as *const c_void;

        self.register(IoringRegisterOp::RegisterProbe, arg, nr_args)?;

        Ok(probe)
    }

    #[must_use]
    pub fn opcode_supported(&self, opcode: u8) -> bool {
        match self.probe() {
            Ok(probe) => probe.opcode_supported(opcode),
            Err(_) => false,
        }
    }

    #[must_use]
    pub fn get_sqe(&mut self) -> Option<&mut io_uring_sqe> {
        let sqe = self.sq.peek_sqe()?;
        *sqe = io_uring_sqe::default();
        Some(sqe)
    }

    /// Get an SQE with reclaim support for better performance
    ///
    /// This method first tries to get a cached SQE, falling back to
    /// a fresh SQE if the cache is empty. The returned SQE should be
    /// reclaimed using `reclaim_sqe()` when the operation completes.
    #[must_use]
    pub fn get_sqe_with_reclaim(&mut self) -> Option<&mut io_uring_sqe> {
        // Try to get a cached SQE first
        if let Some(cached_sqe_ptr) = self.sq.get_cached_sqe() {
            // SAFETY: cached_sqe_ptr is a valid pointer within the SQE array
            unsafe {
                return Some(&mut *cached_sqe_ptr);
            }
        }

        // Fall back to regular SQE allocation
        self.get_sqe()
    }

    /// Reclaim a completed SQE back to the cache
    ///
    /// This should be called after processing the corresponding CQE
    /// to reuse the SQE for future operations and avoid initialization overhead.
    pub fn reclaim_sqe(&mut self, sqe: &mut io_uring_sqe) {
        // Take a pointer to SQE before reclaiming it to avoid borrow issues
        let sqe_ptr = sqe as *mut io_uring_sqe;
        self.sq.reclaim_sqe(sqe_ptr);
    }

    // Linked operation helpers

    /// Mark an SQE as linked with the next SQE
    ///
    /// The next SQE submitted will only execute if this SQE succeeds.
    /// This creates a dependency chain between operations.
    #[must_use]
    pub fn link_sqe(sqe: &mut io_uring_sqe) -> &mut io_uring_sqe {
        sqe.flags |= crate::IOSQE_IO_LINK;
        sqe
    }

    /// Mark an SQE as hard-linked with the next SQE
    ///
    /// Unlike normal links, hard links are not broken on failure.
    /// The next SQE will execute regardless of this SQE's result.
    #[must_use]
    pub fn hardlink_sqe(sqe: &mut io_uring_sqe) -> &mut io_uring_sqe {
        sqe.flags |= crate::IOSQE_IO_HARDLINK;
        sqe
    }

    /// Mark an SQE for drain mode
    ///
    /// When set, the kernel will drain the submission queue before
    /// executing this operation, ensuring all previously submitted
    /// operations complete first.
    #[must_use]
    pub fn drain_sqe(sqe: &mut io_uring_sqe) -> &mut io_uring_sqe {
        sqe.flags |= crate::IOSQE_IO_DRAIN;
        sqe
    }

    /// Mark an SQE as asynchronous
    ///
    /// This hints to the kernel that the operation should be
    /// executed asynchronously when possible.
    #[must_use]
    pub fn make_async(sqe: &mut io_uring_sqe) -> &mut io_uring_sqe {
        sqe.flags |= crate::IOSQE_ASYNC;
        sqe
    }

    /// Clear all flags from an SQE
    #[must_use]
    pub fn clear_sqe_flags(sqe: &mut io_uring_sqe) -> &mut io_uring_sqe {
        sqe.flags = 0;
        sqe
    }

    /// Get current flags of an SQE
    #[must_use]
    pub fn get_sqe_flags(sqe: &io_uring_sqe) -> u8 {
        sqe.flags
    }

    // User data allocator methods

    /// Allocate a unique user_data value for SQE tracking
    ///
    /// Returns a unique value that can be used to identify operations
    /// through their corresponding CQEs. The allocator reuses freed
    /// values to avoid overflow.
    #[must_use]
    pub fn alloc_user_data(&self) -> u64 {
        // Try to reuse a freed user_data first
        if let Some(reused) = self.free_user_data.borrow_mut().pop() {
            return reused;
        }

        // Allocate new user_data, wrapping around if we reach u64::MAX
        let current = self
            .next_user_data
            .fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        if current == 0 {
            // If we wrapped back to 0, use 1 to avoid reserved value
            self.next_user_data
                .store(2, core::sync::atomic::Ordering::Relaxed);
            1
        } else {
            current
        }
    }

    /// Free a user_data value for reuse
    ///
    /// This should be called after processing the corresponding CQE
    /// to allow the user_data value to be reused for future operations.
    pub fn free_user_data(&self, user_data: u64) {
        // Don't add 0 to free list as it's a reserved value
        if user_data != 0 {
            self.free_user_data.borrow_mut().push(user_data);
        }
    }

    /// Set user_data on an SQE using the allocator
    ///
    /// Convenience method that allocates a user_data value
    /// and sets it on the provided SQE.
    #[must_use]
    pub fn set_sqe_user_data(sqe: &mut io_uring_sqe, user_data: u64) -> &mut io_uring_sqe {
        sqe.user_data = user_data;
        sqe
    }

    /// Get the number of currently allocated user_data values
    #[must_use]
    pub fn allocated_user_data_count(&self) -> usize {
        // Total allocated = next_user_data - freed_count
        let next_val = self
            .next_user_data
            .load(core::sync::atomic::Ordering::Relaxed);
        let freed_count = self.free_user_data.borrow().len();

        if next_val == 0 {
            0
        } else if next_val == 1 {
            0
        } else {
            (next_val as usize).saturating_sub(freed_count)
        }
    }

    /// Get the number of freed user_data values available for reuse
    #[must_use]
    pub fn available_user_data_count(&self) -> usize {
        self.free_user_data.borrow().len()
    }

    // Multi-shot operation support

    /// Check if a CQE has the IORING_CQE_F_MORE flag set
    ///
    /// This flag indicates that the operation is a multi-shot operation
    /// and more completions will be generated for this operation.
    #[must_use]
    pub fn cqe_has_more(&self, cqe: &crate::io_uring_cqe) -> bool {
        cqe.flags & crate::IORING_CQE_F_MORE != 0
    }

    /// Check if a CQE has any flags set
    #[must_use]
    pub fn cqe_has_flags(&self, cqe: &crate::io_uring_cqe, flags: u32) -> bool {
        cqe.flags & flags != 0
    }

    /// Get all flags from a CQE
    #[must_use]
    pub fn cqe_get_flags(&self, cqe: &crate::io_uring_cqe) -> u32 {
        cqe.flags
    }

    /// Setup a multi-shot poll operation
    ///
    /// Multi-shot operations generate multiple CQEs without resubmission.
    /// The operation continues until explicitly cancelled.
    #[must_use]
    pub fn poll_add_multishot(&mut self, fd: i32, events: u16) -> Option<&mut io_uring_sqe> {
        let sqe = self.get_sqe()?;
        crate::sqe::PollAdd::new(fd, events).prep(sqe);
        // Multi-shot poll requires IOSQE_ASYNC flag
        sqe.flags |= crate::IOSQE_ASYNC;
        Some(sqe)
    }

    /// Setup a multi-shot accept operation
    ///
    /// Multi-shot accept generates multiple CQEs for each incoming connection
    /// without resubmission. The operation continues until explicitly cancelled.
    #[must_use]
    pub fn accept_multishot(&mut self, fd: i32, flags: i32) -> Option<&mut io_uring_sqe> {
        let sqe = self.get_sqe()?;
        crate::sqe::Accept::new(fd, flags | crate::SOCK_CLOEXEC).prep(sqe);
        // Multi-shot accept requires IOSQE_ASYNC flag
        sqe.flags |= crate::IOSQE_ASYNC;
        Some(sqe)
    }

    /// Cancel a multi-shot operation
    ///
    /// Cancels a multi-shot operation identified by its user_data.
    /// After cancellation, a final CQE will be generated.
    pub fn cancel_multishot(&mut self, user_data: u64) -> Option<&mut io_uring_sqe> {
        self.prepare(&crate::sqe::AsyncCancel::new(user_data, 0))
    }

    #[doc = "Submit all pending SQEs to the kernel."]
    #[doc = ""]
    #[doc = "## Errors"]
    #[doc = "Returns `EnterError` if the submit fails."]
    pub fn submit(&mut self) -> Result<usize, EnterError> {
        let to_submit = self.sq.update_kernel_tail();
        let result = self.enter(to_submit, 0, 0, None);
        self.sq.update_from_kernel();
        self.cq.update_kernel_tail();
        result
    }

    #[doc = "Submit pending SQEs and wait for the specified number of CQEs."]
    #[doc = ""]
    #[doc = "## Errors"]
    #[doc = "Returns `EnterError` if the submit fails."]
    pub fn submit_and_wait(&mut self, wait_count: usize) -> Result<usize, EnterError> {
        let to_submit = self.sq.update_kernel_tail();
        let result = self.enter(
            to_submit,
            wait_count as u32,
            crate::IORING_ENTER_GETEVENTS,
            None,
        );
        self.sq.update_from_kernel();
        self.cq.update_kernel_tail();
        result
    }

    #[doc = "Enter the io_uring with the specified parameters."]
    #[doc = ""]
    #[doc = "## Errors"]
    #[doc = "Returns `EnterError` if the enter fails."]
    pub fn enter(
        &mut self,
        to_submit: u32,
        wait_count: u32,
        flags: u32,
        sig: Option<&sigset_t>,
    ) -> Result<usize, EnterError> {
        let sigmask_ptr: *mut sigset_t =
            sig.map_or(null_mut(), |s| s as *const sigset_t as *mut sigset_t);
        let sigmask_size = sig.map_or(0, |_| core::mem::size_of::<sigset_t>());

        // SAFETY: io_uring_enter is safe to call with these parameters
        let submitted: u32 = unsafe {
            rustix::io_uring::io_uring_enter(
                self.fd.as_fd(),
                to_submit,
                wait_count,
                IoringEnterFlags::from_bits(flags).unwrap_or(IoringEnterFlags::empty()),
                sigmask_ptr.cast::<core::ffi::c_void>(),
                sigmask_size,
            )
        }?;

        Ok(submitted as usize)
    }

    #[must_use]
    pub fn peek_cqe(&mut self) -> Option<&crate::io_uring_cqe> {
        self.cq.update_kernel_tail();
        self.cq.peek()
    }

    pub fn copy_cqes(&mut self, count: usize) -> &[crate::io_uring_cqe] {
        self.cq.update_kernel_tail();
        let available = self.cq.events_available() as usize;
        let to_copy = count.min(available);

        if to_copy == 0 {
            return &[];
        }

        let head = self.cq.get_khead() as usize;

        // SAFETY: We're reading from our own mapped memory
        unsafe { core::slice::from_raw_parts(self.cq.cqe_ptr().add(head), to_copy) }
    }

    pub fn cqe_seen(&mut self, _cqe: &crate::io_uring_cqe) {
        #[allow(unused_unsafe)]
        self.cq.advance(1);
    }

    #[must_use]
    pub fn sq_space_left(&self) -> u32 {
        self.sq.space_left()
    }

    #[must_use]
    pub fn cq_space_left(&self) -> u32 {
        self.cq.events_available()
    }

    #[must_use]
    pub fn is_sq_full(&self) -> bool {
        self.sq.is_full()
    }

    #[must_use]
    pub fn is_cq_empty(&self) -> bool {
        self.cq.is_empty()
    }

    pub fn prepare<P: PrepSqe>(&mut self, op: &P) -> Option<&mut io_uring_sqe> {
        let sqe = self.get_sqe()?;
        op.prep(sqe);
        Some(sqe)
    }

    pub fn prepare_mut<P: PrepSqeMut>(&mut self, op: &mut P) -> Option<&mut io_uring_sqe> {
        let sqe = self.get_sqe()?;
        op.prep(sqe);
        Some(sqe)
    }

    #[must_use]
    pub fn nop(&mut self) -> Option<&mut io_uring_sqe> {
        self.prepare(&crate::sqe::Nop)
    }

    #[must_use]
    pub fn read(&mut self, fd: i32, buf: &mut [u8], offset: u64) -> Option<&mut io_uring_sqe> {
        self.prepare_mut(&mut crate::sqe::Read::new(fd, buf, offset))
    }

    #[must_use]
    pub fn write(&mut self, fd: i32, buf: &[u8], offset: u64) -> Option<&mut io_uring_sqe> {
        self.prepare(&crate::sqe::Write::new(fd, buf, offset))
    }

    #[must_use]
    pub fn read_fixed(
        &mut self,
        fd: i32,
        buf: &mut [u8],
        offset: u64,
        buf_index: u16,
    ) -> Option<&mut io_uring_sqe> {
        self.prepare_mut(&mut crate::sqe::ReadFixed::new(fd, buf, offset, buf_index))
    }

    #[must_use]
    pub fn write_fixed(
        &mut self,
        fd: i32,
        buf: &[u8],
        offset: u64,
        buf_index: u16,
    ) -> Option<&mut io_uring_sqe> {
        self.prepare(&crate::sqe::WriteFixed::new(fd, buf, offset, buf_index))
    }

    #[must_use]
    pub fn openat(&mut self, path: &CStr, flags: u32, mode: u32) -> Option<&mut io_uring_sqe> {
        self.prepare(&crate::sqe::OpenAt::new(crate::AT_FDCWD, path, flags, mode))
    }

    #[must_use]
    pub fn statx(
        &mut self,
        path: &CStr,
        flags: u32,
        mask: u32,
        statxbuf: &mut rustix::fs::Statx,
    ) -> Option<&mut io_uring_sqe> {
        self.prepare_mut(&mut crate::sqe::Statx::new(
            crate::AT_FDCWD,
            path,
            flags,
            mask,
            statxbuf,
        ))
    }

    #[must_use]
    pub fn fallocate(
        &mut self,
        fd: i32,
        mode: u32,
        offset: u64,
        len: u64,
    ) -> Option<&mut io_uring_sqe> {
        self.prepare(&crate::sqe::Fallocate::new(fd, mode, offset, len))
    }

    #[must_use]
    pub fn fadvise(
        &mut self,
        fd: i32,
        offset: u64,
        len: u32,
        advice: u32,
    ) -> Option<&mut io_uring_sqe> {
        self.prepare(&crate::sqe::Fadvise::new(fd, offset, len, advice))
    }

    #[must_use]
    pub fn madvise(
        &mut self,
        addr: *mut c_void,
        len: u32,
        advice: u32,
    ) -> Option<&mut io_uring_sqe> {
        self.prepare(&crate::sqe::Madvise::new(addr, len, advice))
    }

    #[must_use]
    pub fn unlinkat(&mut self, dirfd: i32, path: &CStr, flags: u32) -> Option<&mut io_uring_sqe> {
        self.prepare(&crate::sqe::UnlinkAt::new(dirfd, path, flags))
    }

    #[must_use]
    pub fn unlink(&mut self, path: &CStr, flags: u32) -> Option<&mut io_uring_sqe> {
        self.unlinkat(crate::AT_FDCWD, path, flags)
    }

    #[must_use]
    pub fn renameat(
        &mut self,
        olddirfd: i32,
        oldpath: &CStr,
        newdirfd: i32,
        newpath: &CStr,
        flags: u32,
    ) -> Option<&mut io_uring_sqe> {
        self.prepare(&crate::sqe::RenameAt::new(
            olddirfd, oldpath, newdirfd, newpath, flags,
        ))
    }

    #[must_use]
    pub fn rename(
        &mut self,
        oldpath: &CStr,
        newpath: &CStr,
        flags: u32,
    ) -> Option<&mut io_uring_sqe> {
        self.renameat(crate::AT_FDCWD, oldpath, crate::AT_FDCWD, newpath, flags)
    }

    #[must_use]
    pub fn mkdirat(&mut self, dirfd: i32, path: &CStr, mode: u32) -> Option<&mut io_uring_sqe> {
        self.prepare(&crate::sqe::MkdirAt::new(dirfd, path, mode))
    }

    #[must_use]
    pub fn mkdir(&mut self, path: &CStr, mode: u32) -> Option<&mut io_uring_sqe> {
        self.mkdirat(crate::AT_FDCWD, path, mode)
    }

    #[must_use]
    pub fn symlinkat(
        &mut self,
        target: &CStr,
        newdirfd: i32,
        linkpath: &CStr,
    ) -> Option<&mut io_uring_sqe> {
        self.prepare(&crate::sqe::SymlinkAt::new(target, newdirfd, linkpath))
    }

    #[must_use]
    pub fn symlink(&mut self, target: &CStr, linkpath: &CStr) -> Option<&mut io_uring_sqe> {
        self.symlinkat(target, crate::AT_FDCWD, linkpath)
    }

    #[must_use]
    pub fn linkat(
        &mut self,
        olddirfd: i32,
        oldpath: &CStr,
        newdirfd: i32,
        newpath: &CStr,
        flags: u32,
    ) -> Option<&mut io_uring_sqe> {
        self.prepare(&crate::sqe::LinkAt::new(
            olddirfd, oldpath, newdirfd, newpath, flags,
        ))
    }

    #[must_use]
    pub fn link(
        &mut self,
        oldpath: &CStr,
        newpath: &CStr,
        flags: u32,
    ) -> Option<&mut io_uring_sqe> {
        self.linkat(crate::AT_FDCWD, oldpath, crate::AT_FDCWD, newpath, flags)
    }

    #[must_use]
    pub fn close_direct(&mut self, file_index: u32) -> Option<&mut io_uring_sqe> {
        self.prepare(&crate::sqe::CloseDirect::new(file_index))
    }

    #[must_use]
    pub fn close(&mut self, fd: i32) -> Option<&mut io_uring_sqe> {
        self.prepare(&crate::sqe::Close::new(fd))
    }

    #[must_use]
    pub fn poll_add(&mut self, fd: i32, events: u16) -> Option<&mut io_uring_sqe> {
        self.prepare(&crate::sqe::PollAdd::new(fd, events))
    }

    #[must_use]
    pub fn poll_remove(&mut self, user_data: u64) -> Option<&mut io_uring_sqe> {
        self.prepare(&crate::sqe::PollRemove::new(user_data))
    }

    #[must_use]
    pub fn timeout(
        &mut self,
        ts: &crate::Timespec,
        count: u32,
        flags: u32,
    ) -> Option<&mut io_uring_sqe> {
        self.prepare(&crate::sqe::Timeout::new(ts, count, flags))
    }

    #[must_use]
    pub fn timeout_relative(&mut self, ts: &crate::Timespec) -> Option<&mut io_uring_sqe> {
        self.prepare(&crate::sqe::Timeout::relative(ts))
    }

    #[must_use]
    pub fn timeout_absolute(&mut self, ts: &crate::Timespec) -> Option<&mut io_uring_sqe> {
        self.prepare(&crate::sqe::Timeout::absolute(ts))
    }

    #[must_use]
    pub fn timeout_remove(&mut self, user_data: u64) -> Option<&mut io_uring_sqe> {
        self.prepare(&crate::sqe::TimeoutRemove::new(user_data))
    }

    #[must_use]
    pub fn link_timeout(&mut self, ts: &crate::Timespec, flags: u32) -> Option<&mut io_uring_sqe> {
        self.prepare(&crate::sqe::LinkTimeout::new(ts, flags))
    }

    // Networking convenience methods

    #[must_use]
    pub fn send(&mut self, fd: i32, buf: &[u8], flags: i32) -> Option<&mut io_uring_sqe> {
        self.prepare(&crate::sqe::Send::new(fd, buf, flags))
    }

    #[must_use]
    pub fn recv(&mut self, fd: i32, buf: &mut [u8], flags: i32) -> Option<&mut io_uring_sqe> {
        self.prepare_mut(&mut crate::sqe::Recv::new(fd, buf, flags))
    }

    #[must_use]
    pub fn sendmsg(
        &mut self,
        fd: i32,
        msg: &crate::sqe::MsgHdr,
        flags: i32,
    ) -> Option<&mut io_uring_sqe> {
        self.prepare(&crate::sqe::SendMsg::new(fd, msg, flags))
    }

    #[must_use]
    pub fn recvmsg<'a>(
        &mut self,
        fd: i32,
        msg: &'a mut crate::sqe::MsgHdr<'a>,
        flags: i32,
    ) -> Option<&mut io_uring_sqe> {
        self.prepare_mut(&mut crate::sqe::RecvMsg::new(fd, msg, flags))
    }

    #[must_use]
    pub fn accept(&mut self, fd: i32, flags: i32) -> Option<&mut io_uring_sqe> {
        self.prepare_mut(&mut crate::sqe::Accept::new(fd, flags))
    }

    #[must_use]
    pub fn accept_with_addr(
        &mut self,
        fd: i32,
        addr: &mut [u8],
        addrlen: &mut u32,
        flags: i32,
    ) -> Option<&mut io_uring_sqe> {
        self.prepare_mut(&mut crate::sqe::Accept::with_addr(fd, addr, addrlen, flags))
    }

    #[must_use]
    pub fn accept_with_file_index(
        &mut self,
        fd: i32,
        file_index: u32,
        flags: i32,
    ) -> Option<&mut io_uring_sqe> {
        self.prepare_mut(&mut crate::sqe::Accept::with_file_index(
            fd, file_index, flags,
        ))
    }

    #[must_use]
    pub fn accept_with_addr_and_file_index(
        &mut self,
        fd: i32,
        addr: &mut [u8],
        addrlen: &mut u32,
        file_index: u32,
        flags: i32,
    ) -> Option<&mut io_uring_sqe> {
        self.prepare_mut(&mut crate::sqe::Accept::with_addr_and_file_index(
            fd, addr, addrlen, file_index, flags,
        ))
    }

    #[must_use]
    pub fn connect(&mut self, fd: i32, addr: &[u8], addrlen: u32) -> Option<&mut io_uring_sqe> {
        self.prepare(&crate::sqe::Connect::new(fd, addr, addrlen))
    }

    #[must_use]
    pub fn shutdown(&mut self, fd: i32, how: i32) -> Option<&mut io_uring_sqe> {
        self.prepare(&crate::sqe::Shutdown::new(fd, how))
    }

    // Advanced I/O convenience methods

    #[must_use]
    pub fn splice(
        &mut self,
        fd_in: i32,
        off_in: u64,
        fd_out: i32,
        off_out: u64,
        len: u32,
        flags: u32,
    ) -> Option<&mut io_uring_sqe> {
        self.prepare(&crate::sqe::Splice::new(
            fd_in, off_in, fd_out, off_out, len, flags,
        ))
    }

    #[must_use]
    pub fn tee(
        &mut self,
        fd_in: i32,
        fd_out: i32,
        len: u32,
        flags: u32,
    ) -> Option<&mut io_uring_sqe> {
        self.prepare(&crate::sqe::Tee::new(fd_in, fd_out, len, flags))
    }

    #[must_use]
    pub fn provide_buffers(
        &mut self,
        addr: *mut c_void,
        len: u32,
        bgid: u16,
        bid: u16,
        nbufs: u32,
    ) -> Option<&mut io_uring_sqe> {
        self.prepare(&crate::sqe::ProvideBuffers::new(
            addr, len, bgid, bid, nbufs,
        ))
    }

    #[must_use]
    pub fn remove_buffers(&mut self, bgid: u16, nr: u32) -> Option<&mut io_uring_sqe> {
        self.prepare(&crate::sqe::RemoveBuffers::new(bgid, nr))
    }

    #[must_use]
    pub fn free_buffers(&mut self, bgid: u16) -> Option<&mut io_uring_sqe> {
        self.prepare(&crate::sqe::FreeBuffers::new(bgid))
    }

    #[must_use]
    pub fn cancel(&mut self, user_data: u64, flags: u32) -> Option<&mut io_uring_sqe> {
        self.prepare(&crate::sqe::AsyncCancel::new(user_data, flags))
    }

    #[must_use]
    pub fn cancel_all(&mut self) -> Option<&mut io_uring_sqe> {
        self.prepare(&crate::sqe::AsyncCancel::all())
    }

    #[must_use]
    pub fn cancel_any(&mut self) -> Option<&mut io_uring_sqe> {
        self.prepare(&crate::sqe::AsyncCancel::any())
    }

    #[must_use]
    pub fn msg_ring(
        &mut self,
        fd: i32,
        user_data: u64,
        flags: u32,
        len: u32,
    ) -> Option<&mut io_uring_sqe> {
        self.prepare(&crate::sqe::MsgRing::new(fd, user_data, flags, len))
    }
}

impl Drop for IoUring {
    fn drop(&mut self) {
        // The file descriptor and memory mappings will be automatically
        // cleaned up when OwnedFd and RwMmap are dropped
    }
}
