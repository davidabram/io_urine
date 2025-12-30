use core::ffi::c_void;
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
        let result = self.enter(to_submit, wait_count as u32, 0, None);
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
    pub fn close(&mut self, fd: i32) -> Option<&mut io_uring_sqe> {
        self.prepare(&crate::sqe::Close::new(fd))
    }
}

impl Drop for IoUring {
    fn drop(&mut self) {
        // The file descriptor and memory mappings will be automatically
        // cleaned up when OwnedFd and RwMmap are dropped
    }
}
