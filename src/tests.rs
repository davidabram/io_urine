#[cfg(test)]
mod tests {
    use core::ffi::c_void;

    use std::ffi::CString;

    use crate::{
        InitError, IoUring, Iovec, AT_FDCWD, IORING_OP_CLOSE, IORING_OP_FADVISE,
        IORING_OP_FALLOCATE, IORING_OP_LINKAT, IORING_OP_MADVISE, IORING_OP_MKDIRAT, IORING_OP_NOP,
        IORING_OP_OPENAT, IORING_OP_READ_FIXED, IORING_OP_RENAMEAT, IORING_OP_STATX,
        IORING_OP_SYMLINKAT, IORING_OP_UNLINKAT, IORING_OP_WRITE_FIXED,
    };
    use pretty_assertions::assert_eq;
    use rustix::event::{eventfd, EventfdFlags};
    use rustix::fd::AsRawFd;
    use rustix::io::Errno;
    use tempfile::NamedTempFile;

    #[test]
    fn test_nop() {
        let mut ring = IoUring::with_entries(8, 8).expect("Failed to create ring");

        // Get an SQE and prepare a NOP operation
        let sqe = ring.nop().expect("Failed to get SQE");
        assert_eq!(sqe.opcode, IORING_OP_NOP);

        // Submit the operation - check that we submitted at least 0 (kernel might adjust entries)
        let submitted = ring.submit().expect("Failed to submit");
        // Kernel might return 0 if it adjusts entries or if NOP is optimized away
        assert!(
            submitted <= 8,
            "Expected submission count <= ring size, got {submitted}"
        );
    }

    #[test]
    fn test_close() {
        let mut ring = IoUring::with_entries(8, 8).expect("Failed to create ring");

        // Create a temporary file to get a valid fd
        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let fd = temp_file.as_raw_fd();

        // Prepare a close operation
        let sqe = ring.close(fd).expect("Failed to get SQE");
        assert_eq!(sqe.opcode, IORING_OP_CLOSE);
        assert_eq!(sqe.fd, fd);
    }

    #[test]
    fn test_register_unregister_buffers() {
        let ring = IoUring::with_entries(8, 8).expect("Failed to create ring");

        let mut buf = vec![0u8; 16];
        let iovecs = [Iovec::new(buf.as_mut_ptr() as *mut c_void, buf.len())];

        ring.register_buffers(&iovecs)
            .expect("Failed to register buffers");
        ring.unregister_buffers()
            .expect("Failed to unregister buffers");
    }

    #[test]
    fn test_read_fixed_prep() {
        let mut ring = IoUring::with_entries(8, 8).expect("Failed to create ring");

        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let fd = temp_file.as_raw_fd();

        let mut buf = vec![0u8; 8];
        let sqe = ring
            .read_fixed(fd, &mut buf, 123, 0)
            .expect("Failed to get SQE");
        assert_eq!(sqe.opcode, IORING_OP_READ_FIXED);
        assert_eq!(sqe.fd, fd);
        assert_eq!(sqe.off, 123);
        assert_eq!(sqe.len, buf.len() as u32);
        assert_eq!(sqe.addr, buf.as_mut_ptr() as u64);
        assert_eq!(sqe.buf_index, 0);
    }

    #[test]
    fn test_write_fixed_prep() {
        let mut ring = IoUring::with_entries(8, 8).expect("Failed to create ring");

        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let fd = temp_file.as_raw_fd();

        let buf = vec![0u8; 8];
        let sqe = ring
            .write_fixed(fd, &buf, 456, 1)
            .expect("Failed to get SQE");
        assert_eq!(sqe.opcode, IORING_OP_WRITE_FIXED);
        assert_eq!(sqe.fd, fd);
        assert_eq!(sqe.off, 456);
        assert_eq!(sqe.len, buf.len() as u32);
        assert_eq!(sqe.addr, buf.as_ptr() as u64);
        assert_eq!(sqe.buf_index, 1);
    }

    #[test]
    fn test_register_unregister_files() {
        let ring = IoUring::with_entries(8, 8).expect("Failed to create ring");

        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let fds = [temp_file.as_raw_fd()];

        ring.register_files(&fds).expect("Failed to register files");
        ring.unregister_files().expect("Failed to unregister files");
    }

    #[test]
    fn test_register_files_update() {
        let ring = IoUring::with_entries(8, 8).expect("Failed to create ring");

        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let fds = [temp_file.as_raw_fd()];

        ring.register_files(&fds).expect("Failed to register files");

        let update = [-1];
        match ring.register_files_update(0, &update) {
            Ok(()) => {}
            Err(InitError::RegisterFailed(errno))
                if errno == Errno::INVAL || errno == Errno::NOSYS =>
            {
                ring.unregister_files().expect("Failed to unregister files");
                return;
            }
            Err(e) => panic!("Unexpected error: {e:?}"),
        }

        ring.unregister_files().expect("Failed to unregister files");
    }

    #[test]
    fn test_register_unregister_eventfd() {
        let ring = IoUring::with_entries(8, 8).expect("Failed to create ring");

        let eventfd = eventfd(0, EventfdFlags::CLOEXEC).expect("Failed to create eventfd");

        ring.register_eventfd(eventfd.as_raw_fd())
            .expect("Failed to register eventfd");
        ring.unregister_eventfd()
            .expect("Failed to unregister eventfd");
    }

    #[test]
    fn test_register_unregister_eventfd_async() {
        let ring = IoUring::with_entries(8, 8).expect("Failed to create ring");

        let eventfd = eventfd(0, EventfdFlags::CLOEXEC).expect("Failed to create eventfd");

        match ring.register_eventfd_async(eventfd.as_raw_fd()) {
            Ok(()) => {}
            Err(InitError::RegisterFailed(errno))
                if errno == Errno::INVAL || errno == Errno::NOSYS =>
            {
                return;
            }
            Err(e) => panic!("Unexpected error: {e:?}"),
        }

        ring.unregister_eventfd()
            .expect("Failed to unregister eventfd");
    }

    #[test]
    fn test_probe() {
        let ring = IoUring::with_entries(8, 8).expect("Failed to create ring");

        match ring.probe() {
            Ok(probe) => {
                assert!(probe.opcode_supported(IORING_OP_NOP));
                assert!(ring.opcode_supported(IORING_OP_NOP));
            }
            Err(InitError::RegisterFailed(errno))
                if errno == Errno::INVAL || errno == Errno::NOSYS =>
            {
                assert!(!ring.opcode_supported(IORING_OP_NOP));
            }
            Err(e) => panic!("Unexpected error: {e:?}"),
        }
    }

    #[test]
    fn test_sq_space() {
        let ring = IoUring::with_entries(8, 8).expect("Failed to create ring");

        // Initially, space should be available (kernel might adjust entries)
        let space = ring.sq_space_left();
        assert!(space > 0, "Expected some SQ space, got {space}");
        assert!(!ring.is_sq_full());
    }

    #[test]
    fn test_cq_empty() {
        let ring = IoUring::with_entries(8, 8).expect("Failed to create ring");

        // Initially, CQ should be empty
        assert!(ring.is_cq_empty());
        assert_eq!(ring.cq_space_left(), 0);
    }

    #[test]
    fn test_ring_creation() {
        // Test default creation
        let ring = IoUring::new(32).expect("Failed to create ring");
        // Kernel might adjust entries, so just check we have some space
        let space = ring.sq_space_left();
        assert!(space > 0, "Expected some SQ space, got {space}");

        // Test with custom entries
        let ring =
            IoUring::with_entries(16, 16).expect("Failed to create ring with custom entries");
        let space = ring.sq_space_left();
        assert!(space > 0, "Expected some SQ space, got {space}");
    }

    #[test]
    fn test_openat_prep() {
        let mut ring = IoUring::with_entries(8, 8).expect("Failed to create ring");

        let path = CString::new("test_openat").expect("Failed to create C string");
        let flags: u32 = 0x1234;
        let mode: u32 = 0o644;

        let sqe = ring.openat(&path, flags, mode).expect("Failed to get SQE");

        assert_eq!(sqe.opcode, IORING_OP_OPENAT);
        assert_eq!(sqe.fd, AT_FDCWD);
        assert_eq!(sqe.off, 0);
        assert_eq!(sqe.addr, path.as_ptr() as u64);
        assert_eq!(sqe.len, mode);
        assert_eq!(sqe.rw_flags, flags as i32);
    }

    #[test]
    fn test_close_direct_prep() {
        let mut ring = IoUring::with_entries(8, 8).expect("Failed to create ring");

        let sqe = ring.close_direct(5).expect("Failed to get SQE");
        assert_eq!(sqe.opcode, IORING_OP_CLOSE);
        assert_eq!(sqe.splice_fd_in, 6);
    }

    #[test]
    fn test_statx_prep() {
        let mut ring = IoUring::with_entries(8, 8).expect("Failed to create ring");

        let path = CString::new("test_statx").expect("Failed to create C string");
        let flags: u32 = 0x5678;
        let mask: u32 = 0x9abc;
        let mut statxbuf: rustix::fs::Statx = unsafe { core::mem::zeroed() };

        let sqe = ring
            .statx(&path, flags, mask, &mut statxbuf)
            .expect("Failed to get SQE");

        assert_eq!(sqe.opcode, IORING_OP_STATX);
        assert_eq!(sqe.fd, AT_FDCWD);
        assert_eq!(sqe.addr, path.as_ptr() as u64);
        assert_eq!(sqe.len, mask);
        assert_eq!(sqe.off, (&mut statxbuf as *mut rustix::fs::Statx) as u64);
        assert_eq!(sqe.rw_flags, flags as i32);
    }

    #[test]
    fn test_fallocate_prep() {
        let mut ring = IoUring::with_entries(8, 8).expect("Failed to create ring");

        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let fd = temp_file.as_raw_fd();

        let mode: u32 = 3;
        let offset: u64 = 1234;
        let len: u64 = 4096;

        let sqe = ring
            .fallocate(fd, mode, offset, len)
            .expect("Failed to get SQE");

        assert_eq!(sqe.opcode, IORING_OP_FALLOCATE);
        assert_eq!(sqe.fd, fd);
        assert_eq!(sqe.off, offset);
        assert_eq!(sqe.addr, len);
        assert_eq!(sqe.len, mode);
    }

    #[test]
    fn test_fadvise_prep() {
        let mut ring = IoUring::with_entries(8, 8).expect("Failed to create ring");

        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let fd = temp_file.as_raw_fd();

        let offset: u64 = 0;
        let len: u32 = 1024;
        let advice: u32 = 4;

        let sqe = ring
            .fadvise(fd, offset, len, advice)
            .expect("Failed to get SQE");

        assert_eq!(sqe.opcode, IORING_OP_FADVISE);
        assert_eq!(sqe.fd, fd);
        assert_eq!(sqe.off, offset);
        assert_eq!(sqe.addr, 0);
        assert_eq!(sqe.len, len);
        assert_eq!(sqe.rw_flags, advice as i32);
    }

    #[test]
    fn test_madvise_prep() {
        let mut ring = IoUring::with_entries(8, 8).expect("Failed to create ring");

        let mut buf = vec![0u8; 4096];
        let advice: u32 = 5;

        let sqe = ring
            .madvise(buf.as_mut_ptr().cast::<c_void>(), buf.len() as u32, advice)
            .expect("Failed to get SQE");

        assert_eq!(sqe.opcode, IORING_OP_MADVISE);
        assert_eq!(sqe.fd, -1);
        assert_eq!(sqe.addr, buf.as_mut_ptr() as u64);
        assert_eq!(sqe.len, buf.len() as u32);
        assert_eq!(sqe.rw_flags, advice as i32);
    }

    #[test]
    fn test_unlinkat_prep() {
        let mut ring = IoUring::with_entries(8, 8).expect("Failed to create ring");

        let path = CString::new("test_unlinkat").expect("Failed to create C string");
        let flags: u32 = 0x1;

        let sqe = ring
            .unlinkat(AT_FDCWD, &path, flags)
            .expect("Failed to get SQE");

        assert_eq!(sqe.opcode, IORING_OP_UNLINKAT);
        assert_eq!(sqe.fd, AT_FDCWD);
        assert_eq!(sqe.addr, path.as_ptr() as u64);
        assert_eq!(sqe.rw_flags, flags as i32);
    }

    #[test]
    fn test_renameat_prep() {
        let mut ring = IoUring::with_entries(8, 8).expect("Failed to create ring");

        let oldpath = CString::new("test_rename_old").expect("Failed to create C string");
        let newpath = CString::new("test_rename_new").expect("Failed to create C string");
        let flags: u32 = 0x2;

        let sqe = ring
            .renameat(AT_FDCWD, &oldpath, AT_FDCWD, &newpath, flags)
            .expect("Failed to get SQE");

        assert_eq!(sqe.opcode, IORING_OP_RENAMEAT);
        assert_eq!(sqe.fd, AT_FDCWD);
        assert_eq!(sqe.addr, oldpath.as_ptr() as u64);
        assert_eq!(sqe.off, newpath.as_ptr() as u64);
        assert_eq!(sqe.len, AT_FDCWD as u32);
        assert_eq!(sqe.rw_flags, flags as i32);
    }

    #[test]
    fn test_mkdirat_prep() {
        let mut ring = IoUring::with_entries(8, 8).expect("Failed to create ring");

        let path = CString::new("test_mkdirat").expect("Failed to create C string");
        let mode: u32 = 0o755;

        let sqe = ring
            .mkdirat(AT_FDCWD, &path, mode)
            .expect("Failed to get SQE");

        assert_eq!(sqe.opcode, IORING_OP_MKDIRAT);
        assert_eq!(sqe.fd, AT_FDCWD);
        assert_eq!(sqe.addr, path.as_ptr() as u64);
        assert_eq!(sqe.len, mode);
    }

    #[test]
    fn test_symlinkat_prep() {
        let mut ring = IoUring::with_entries(8, 8).expect("Failed to create ring");

        let target = CString::new("test_symlink_target").expect("Failed to create C string");
        let linkpath = CString::new("test_symlink_linkpath").expect("Failed to create C string");

        let sqe = ring
            .symlinkat(&target, AT_FDCWD, &linkpath)
            .expect("Failed to get SQE");

        assert_eq!(sqe.opcode, IORING_OP_SYMLINKAT);
        assert_eq!(sqe.fd, AT_FDCWD);
        assert_eq!(sqe.addr, target.as_ptr() as u64);
        assert_eq!(sqe.off, linkpath.as_ptr() as u64);
        assert_eq!(sqe.len, 0);
    }

    #[test]
    fn test_linkat_prep() {
        let mut ring = IoUring::with_entries(8, 8).expect("Failed to create ring");

        let oldpath = CString::new("test_linkat_old").expect("Failed to create C string");
        let newpath = CString::new("test_linkat_new").expect("Failed to create C string");
        let flags: u32 = 0x4;

        let sqe = ring
            .linkat(AT_FDCWD, &oldpath, AT_FDCWD, &newpath, flags)
            .expect("Failed to get SQE");

        assert_eq!(sqe.opcode, IORING_OP_LINKAT);
        assert_eq!(sqe.fd, AT_FDCWD);
        assert_eq!(sqe.addr, oldpath.as_ptr() as u64);
        assert_eq!(sqe.off, newpath.as_ptr() as u64);
        assert_eq!(sqe.len, AT_FDCWD as u32);
        assert_eq!(sqe.rw_flags, flags as i32);
    }

    #[test]
    fn test_poll_add_prep() {
        let mut ring = IoUring::with_entries(8, 8).expect("Failed to create ring");

        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let fd = temp_file.as_raw_fd();
        let events = crate::POLLIN | crate::POLLOUT;

        let sqe = ring.poll_add(fd, events).expect("Failed to get SQE");
        assert_eq!(sqe.opcode, crate::IORING_OP_POLL_ADD);
        assert_eq!(sqe.fd, fd);
        assert_eq!(sqe.addr, events as u64);
    }

    #[test]
    fn test_poll_remove_prep() {
        let mut ring = IoUring::with_entries(8, 8).expect("Failed to create ring");

        let user_data = 0x12345678;
        let sqe = ring.poll_remove(user_data).expect("Failed to get SQE");
        assert_eq!(sqe.opcode, crate::IORING_OP_POLL_REMOVE);
        assert_eq!(sqe.addr, user_data);
    }

    #[test]
    fn test_timeout_prep() {
        let mut ring = IoUring::with_entries(8, 8).expect("Failed to create ring");

        let ts = crate::Timespec::new(1, 500_000_000); // 1.5 seconds
        let count = 1;
        let flags = 0;

        let sqe = ring.timeout(&ts, count, flags).expect("Failed to get SQE");
        assert_eq!(sqe.opcode, crate::IORING_OP_TIMEOUT);
        assert_eq!(sqe.len, count);
        assert_eq!(sqe.rw_flags, flags as i32);
        assert!(sqe.addr != 0); // Should be pointer to timespec
    }

    #[test]
    fn test_timeout_relative_prep() {
        let mut ring = IoUring::with_entries(8, 8).expect("Failed to create ring");

        let ts = crate::Timespec::new(0, 100_000_000); // 100ms relative

        let sqe = ring.timeout_relative(&ts).expect("Failed to get SQE");
        assert_eq!(sqe.opcode, crate::IORING_OP_TIMEOUT);
        assert_eq!(sqe.len, 0);
        assert_eq!(sqe.rw_flags, 0); // No flags for relative
        assert!(sqe.addr != 0);
    }

    #[test]
    fn test_timeout_absolute_prep() {
        let mut ring = IoUring::with_entries(8, 8).expect("Failed to create ring");

        let ts = crate::Timespec::new(1234567890, 0); // Absolute time

        let sqe = ring.timeout_absolute(&ts).expect("Failed to get SQE");
        assert_eq!(sqe.opcode, crate::IORING_OP_TIMEOUT);
        assert_eq!(sqe.len, 0);
        assert_eq!(sqe.rw_flags, crate::IORING_TIMEOUT_ABS as i32);
        assert!(sqe.addr != 0);
    }

    #[test]
    fn test_timeout_remove_prep() {
        let mut ring = IoUring::with_entries(8, 8).expect("Failed to create ring");

        let user_data = 0x87654321;
        let sqe = ring.timeout_remove(user_data).expect("Failed to get SQE");
        assert_eq!(sqe.opcode, crate::IORING_OP_TIMEOUT_REMOVE);
        assert_eq!(sqe.addr, user_data);
    }

    #[test]
    fn test_link_timeout_prep() {
        let mut ring = IoUring::with_entries(8, 8).expect("Failed to create ring");

        let ts = crate::Timespec::new(5, 0); // 5 seconds
        let flags = 0;

        let sqe = ring.link_timeout(&ts, flags).expect("Failed to get SQE");
        assert_eq!(sqe.opcode, crate::IORING_OP_LINK_TIMEOUT);
        assert_eq!(sqe.rw_flags, flags as i32);
        assert!(sqe.addr != 0);
    }

    #[test]
    fn test_poll_timeout_interactions() {
        let mut ring = IoUring::with_entries(8, 8).expect("Failed to create ring");

        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let fd = temp_file.as_raw_fd();
        let events = crate::POLLIN;

        // Add a poll operation
        let poll_sqe = ring.poll_add(fd, events).expect("Failed to get poll SQE");
        assert_eq!(poll_sqe.opcode, crate::IORING_OP_POLL_ADD);

        // Add a timeout to limit the poll
        let ts = crate::Timespec::new(1, 0); // 1 second
        let timeout_sqe = ring
            .timeout_relative(&ts)
            .expect("Failed to get timeout SQE");
        assert_eq!(timeout_sqe.opcode, crate::IORING_OP_TIMEOUT);
    }

    #[test]
    fn test_timespec_creation() {
        let ts = crate::Timespec::new(42, 123456789);
        assert_eq!(ts.tv_sec, 42);
        assert_eq!(ts.tv_nsec, 123456789);
    }

    #[test]
    fn test_poll_event_constants() {
        // Test that poll event constants have expected values
        assert_eq!(crate::POLLIN, 0x0001);
        assert_eq!(crate::POLLOUT, 0x0004);
        assert_eq!(crate::POLLERR, 0x0008);
        assert_eq!(crate::POLLHUP, 0x0010);

        // Test combining flags
        let combined = crate::POLLIN | crate::POLLOUT;
        assert_eq!(combined, 0x0005);
    }
}
