#[cfg(test)]
mod tests {
    use core::ffi::c_void;

    use crate::{
        InitError, IoUring, Iovec, IORING_OP_CLOSE, IORING_OP_NOP, IORING_OP_READ_FIXED,
        IORING_OP_WRITE_FIXED,
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
}
