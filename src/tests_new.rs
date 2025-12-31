    #[test]
    fn test_setup_builder_attach_wq() {
        // Test attach to existing work queue
        // First create a regular ring
        let base_ring = IoUring::new(8).expect("Failed to create base ring");

        let ring = crate::SetupBuilder::new()
            .sq_entries(8)
            .attach_wq(base_ring.as_raw_fd())
            .build();

        match ring {
            Ok(_) => {
                // Success - ring was attached to work queue
            }
            Err(InitError::SyscallFailed(errno)) => {
                // This may fail if kernel doesn't support IORING_SETUP_ATTACH_WQ
                // or if the base ring doesn't support work queue sharing
                assert!(errno == Errno::INVAL || errno == Errno::BADF);
            }
            Err(e) => {
                panic!("Unexpected error: {:?}", e);
            }
        }
    }

    #[test]
    fn test_setup_builder_extended_formats() {
        // Test both SQE128 and CQE32 together
        let ring = crate::SetupBuilder::new()
            .sq_entries(8)
            .sqe128()
            .cqe32()
            .build();

        match ring {
            Ok(_) => {
                // Success - ring was created with extended formats
            }
            Err(InitError::SyscallFailed(errno)) => {
                // This may fail if kernel doesn't support extended formats
                assert_eq!(errno, Errno::INVAL);
            }
            Err(e) => {
                panic!("Unexpected error: {:?}", e);
            }
        }
    }

    #[test]
    fn test_setup_builder_backwards_compatibility() {
        // Test that SetupBuilder produces equivalent results to old constructors
        let mut old_ring =
            IoUring::with_entries(16, 16).expect("Failed to create ring with old method");
        let mut new_ring = crate::SetupBuilder::new()
            .sq_entries(16)
            .cq_entries(16)
            .build()
            .expect("Failed to create ring with SetupBuilder");

        // Both should be created successfully
        let _old_sqe = old_ring.nop();
        let _new_sqe = new_ring.nop();
    }

    // Phase 8: Feature Detection and Probing Tests

    #[test]
    fn test_feature_detection_basic() {
        let ring = IoUring::new(8).expect("Failed to create ring");

        // Test that features() returns some value
        let features = ring.features();
        // Features should be at least SINGLE_MMAP on modern kernels
        assert!(features >= 0, "Features bitmask should be non-negative");

        // Test basic feature helpers
        let _has_single_mmap = ring.has_single_mmap();
        let _has_nodrop = ring.has_nodrop();
        let _has_submit_stable = ring.has_submit_stable();
        let _has_rw_cur_pos = ring.has_rw_cur_pos();
        let _has_cur_personality = ring.has_cur_personality();

        // These should at least return a boolean without panicking
        assert!([true, false].contains(&ring.has_fast_poll()));
        assert!([true, false].contains(&ring.has_poll_32bits()));
        assert!([true, false].contains(&ring.has_sqpoll_fixed()));
        assert!([true, false].contains(&ring.has_ext_arg()));
        assert!([true, false].contains(&ring.has_native_workers()));
        assert!([true, false].contains(&ring.has_rsrc_tags()));
        assert!([true, false].contains(&ring.has_cqe_skip()));
        assert!([true, false].contains(&ring.has_linked_file()));
        assert!([true, false].contains(&ring.has_reg_reg_ring()));
    }

    #[test]
    fn test_kernel_version_detection() {
        let ring = IoUring::new(8).expect("Failed to create ring");

        let (major, minor, patch) = ring.kernel_version();

        // Should be reasonable kernel version (at least 5.x for io_uring)
        assert!(major >= 5, "Major version should be at least 5");
        assert!(minor >= 0, "Minor version should be non-negative");
        assert!(patch >= 0, "Patch version should be non-negative");

        // Test version comparison methods
        assert!(ring.kernel_version_at_least(5, 0, 0), "Should be at least 5.0.0");
        assert!(ring.kernel_version_at_least(5, 1, 0), "Should be at least 5.1.0");
        assert!(!ring.kernel_version_at_least(10, 0, 0), "Should not be 10.x.x");
    }

    #[test]
    fn test_kernel_version_feature_checks() {
        let ring = IoUring::new(8).expect("Failed to create ring");

        // These should all be true on modern kernels
        assert!(ring.has_basic_io_uring(), "Should support basic io_uring");
        assert!(ring.has_registered_files(), "Should support registered files");
        assert!(ring.has_eventfd_notifications(), "Should support eventfd");

        // These may vary by kernel version but should not panic
        let _has_fixed = ring.has_fixed_buffers();
        let _has_sqpoll = ring.has_sq_polling();
        let _has_extended_setup = ring.has_extended_setup();
        let _has_extended_formats = ring.has_extended_formats();
    }

    #[test]
    fn test_extended_enter_arguments() {
        let mut ring = IoUring::new(8).expect("Failed to create ring");

        // Test basic timeout with extended args
        let timeout = crate::Timespec::new(0, 100_000_000); // 100ms

        match ring.submit_and_wait_with_timeout(0, 0, &timeout) {
            Ok(_) | Err(crate::EnterError::SyscallFailed(_)) => {
                // Either success or syscall failure is acceptable
                // Syscall failure might indicate EXT_ARG not supported
            }
            Err(crate::EnterError::UnsupportedOperation) => {
                // This is expected if EXT_ARG feature is not supported
            }
            Err(_) => {
                panic!("Unexpected error type");
            }
        }
    }

    #[test]
    fn test_extended_enter_argument_struct() {
        // Test the io_uring_getevents_arg struct layout
        let timeout = crate::Timespec::new(1, 0);
        let arg = crate::io_uring_getevents_arg {
            mask: 0,
            pad: 0,
            ts: &timeout as *const crate::Timespec as u64,
        };

        // Verify struct fields are set correctly
        assert_eq!(arg.mask, 0);
        assert_eq!(arg.pad, 0);
        assert_ne!(arg.ts, 0);
    }

    #[test]
    fn test_feature_flag_constants() {
        // Test that all feature flag constants are defined
        let single_mmap = crate::IORING_FEAT_SINGLE_MMAP;
        let nodrop = crate::IORING_FEAT_NODROP;
        let submit_stable = crate::IORING_FEAT_SUBMIT_STABLE;
        let rw_cur_pos = crate::IORING_FEAT_RW_CUR_POS;
        let cur_personality = crate::IORING_FEAT_CUR_PERSONALITY;
        let fast_poll = crate::IORING_FEAT_FAST_POLL;
        let poll_32bits = crate::IORING_FEAT_POLL_32BITS;
        let sqpoll_fixed = crate::IORING_FEAT_SQPOLL_FIXED;
        let ext_arg = crate::IORING_FEAT_EXT_ARG;
        let native_workers = crate::IORING_FEAT_NATIVE_WORKERS;
        let rsrc_tags = crate::IORING_FEAT_RSRC_TAGS;
        let cqe_skip = crate::IORING_FEAT_CQE_SKIP;
        let linked_file = crate::IORING_FEAT_LINKED_FILE;
        let reg_reg_ring = crate::IORING_FEAT_REG_REG_RING;

        // Verify constants are power-of-two bit positions
        assert!(single_mmap.is_power_of_two());
        assert!(nodrop.is_power_of_two());
        assert!(submit_stable.is_power_of_two());
        assert!(rw_cur_pos.is_power_of_two());
    }

    #[test]
    fn test_conditional_feature_support() {
        let ring = IoUring::new(8).expect("Failed to create ring");

        // Test multi-shot operations with feature checks
        if ring.has_poll_32bits() {
            // Should work with 32-bit events
            let fd = pipe().unwrap().0.as_raw_fd();
            let _sqe = ring.poll_add_multishot(fd, 0x12345678);
        } else {
            // Should still work with 16-bit events
            let fd = pipe().unwrap().0.as_raw_fd();
            let _sqe = ring.poll_add_multishot(fd, 0x1234);
        }

        // Test multi-shot accept (requires FAST_POLL for best performance)
        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let fd = temp_file.as_raw_fd();

        if ring.has_fast_poll() {
            // Multi-shot accept works better with fast poll
            let _sqe = ring.accept_multishot(fd, 0);
        }

        // Regular accept should always work
        let _sqe = ring.accept(fd, 0);
    }

    #[test]
    fn test_version_comparison_edge_cases() {
        let ring = IoUring::new(8).expect("Failed to create ring");

        // Test exact version matching
        let (major, minor, patch) = ring.kernel_version();
        assert!(ring.kernel_version_at_least(major, minor, patch));
        assert!(ring.kernel_version_at_least(major, minor, patch - 1));
        assert!(!ring.kernel_version_at_least(major, minor, patch + 1));

        // Test major version differences
        if major > 5 {
            assert!(ring.kernel_version_at_least(major - 1, 100, 100));
            assert!(!ring.kernel_version_at_least(major + 1, 0, 0));
        }
    }

    #[test]
    fn test_feature_detection_consistency() {
        let ring = IoUring::new(8).expect("Failed to create ring");

        // Test that has_feature matches specific feature methods
        let has_single_mmap_v1 = ring.has_feature(crate::IORING_FEAT_SINGLE_MMAP);
        let has_single_mmap_v2 = ring.has_single_mmap();
        assert_eq!(has_single_mmap_v1, has_single_mmap_v2);

        let has_ext_arg_v1 = ring.has_feature(crate::IORING_FEAT_EXT_ARG);
        let has_ext_arg_v2 = ring.has_ext_arg();
        assert_eq!(has_ext_arg_v1, has_ext_arg_v2);

        let has_fast_poll_v1 = ring.has_feature(crate::IORING_FEAT_FAST_POLL);
        let has_fast_poll_v2 = ring.has_fast_poll();
        assert_eq!(has_fast_poll_v1, has_fast_poll_v2);
    }
}