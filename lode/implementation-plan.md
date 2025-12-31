# io_urine Implementation Plan

This document outlines the plan for implementing missing io_uring functionality.

## Plan Philosophy

- **Incremental delivery**: Each phase provides useful functionality
- **Dependency-aware**: Build on what exists
- **Testable**: Each feature includes tests
- **Consistent**: Follow existing code patterns (no_std, rustix, liburing-style API)

## Phase 1: Registration APIs (Week 1-2)

**Priority: HIGH** - Essential for performance, zero-copy I/O

### Dependencies
- None (standalone functionality)

### Tasks

#### 1.1 Register System Call Wrapper
- [ ] Add `io_uring_register()` wrapper to `io_uring.rs`
  - Use `rustix::io_uring::io_uring_register()` if available
  - Fall back to raw syscall if needed
  - Error handling with `InitError::RegisterFailed`
- [ ] Add register error variants to `err.rs`
- [ ] Tests for register/unregister operations

#### 1.2 Buffer Registration
- [ ] Add `IoUring::register_buffers(iovecs)` method
  - Takes slice of `Iovec`
  - Locks buffers in kernel memory
  - Returns `Result<(), InitError>`
- [ ] Add `IoUring::unregister_buffers()` method
- [ ] Implement `IORING_OP_READ_FIXED` in `sqe.rs`
- [ ] Implement `IORING_OP_WRITE_FIXED` in `sqe.rs`
- [ ] Add `ReadFixed` and `WriteFixed` structs implementing `PrepSqe`
- [ ] Tests for fixed buffer read/write

#### 1.3 File Registration
- [ ] Add `IoUring::register_files(fds)` method
  - Takes slice of file descriptors
  - Registers for low-overhead I/O
  - Returns `Result<(), InitError>`
- [ ] Add `IoUring::unregister_files()` method
- [ ] Add `IoUring::register_files_update(index, fds)` for partial updates
- [ ] Implement `IOSQE_FIXED_FILE` flag in `sqe.rs`
- [ ] Update `PrepSqe` trait to support fixed file flag
- [ ] Tests for registered file I/O

#### 1.4 Eventfd Registration
- [ ] Add `IoUring::register_eventfd(fd)` method
  - Register eventfd for CQ notifications
  - Returns `Result<(), InitError>`
- [ ] Add `IoUring::unregister_eventfd()` method
- [ ] Add `IoUring::register_eventfd_async(fd)` for async mode
- [ ] Tests for eventfd notifications

#### 1.5 Capability Probing
- [ ] Add `IoUring::probe()` method
  - Returns probe structure with supported operations
  - Helper: `IoUring::opcode_supported(opcode)`
- [ ] Add `Probe` struct to represent probe results
- [ ] Tests for probing various opcodes

### Files to Modify/Create
- `src/io_uring.rs` - Register methods
- `src/sqe.rs` - Fixed buffer ops, IOSQE_FIXED_FILE
- `src/err.rs` - Register error variants
- `src/tests.rs` - Registration tests

### Deliverables
- ✅ Complete registration API
- ✅ Fixed buffer I/O support
- ✅ Registered file I/O support
- ✅ Eventfd notifications
- ✅ Kernel capability probing

---

## Phase 2: File Operations (Week 3-4)

**Priority: HIGH** - Essential for file I/O applications

### Dependencies
- None (Phase 1 is optional for basic file ops)

### Tasks

#### 2.1 Open and Create
- [ ] Implement `IORING_OP_OPENAT` in `sqe.rs`
  - `OpenAt` struct with path, flags, mode
  - Support `IORING_OPENAT_ASYNC` flag
  - Support file creation (O_CREAT, O_EXCL)
- [ ] Implement `IORING_OP_CLOSE_DIRECT` (fast close)
- [ ] Helper: `IoUring::openat(path, flags, mode)` method
- [ ] Tests for file open/create/close

#### 2.2 File Status and Attributes
- [ ] Implement `IORING_OP_STATX` in `sqe.rs`
  - `Statx` struct with path and flags
  - Support extended stat attributes
- [ ] Add `Statx` completion result parsing
- [ ] Tests for statx operation

#### 2.3 File Allocation
- [ ] Implement `IORING_OP_FALLOCATE` in `sqe.rs`
  - `Fallocate` struct with mode, offset, len
  - Support FALLOC_FL_* flags
- [ ] Tests for fallocate

#### 2.4 File Advice
- [ ] Implement `IORING_OP_FADVISE` in `sqe.rs`
  - `Fadvise` struct with advice flag
  - Support POSIX_FADV_* constants
- [ ] Implement `IORING_OP_MADVISE` in `sqe.rs`
  - `Madvise` struct with advice flag
  - Support MADV_* constants
- [ ] Tests for advice operations

#### 2.5 Filesystem Operations
- [ ] Implement `IORING_OP_UNLINKAT` in `sqe.rs`
  - `UnlinkAt` struct with dirfd, path, flags
- [ ] Implement `IORING_OP_RENAMEAT` in `sqe.rs`
  - `RenameAt` struct with olddirfd, oldpath, newdirfd, newpath, flags
- [ ] Implement `IORING_OP_MKDIRAT` in `sqe.rs`
  - `MkdirAt` struct with dirfd, path, mode
- [ ] Implement `IORING_OP_SYMLINKAT` in `sqe.rs`
  - `SymlinkAt` struct with oldpath, newdirfd, newpath
- [ ] Implement `IORING_OP_LINKAT` in `sqe.rs`
  - `LinkAt` struct with olddirfd, oldpath, newdirfd, newpath, flags
- [ ] Tests for filesystem operations

### Files to Modify/Create
- `src/sqe.rs` - All file operation structs
- `src/io_uring.rs` - Convenience methods
- `src/tests.rs` - File operation tests

### Deliverables
- ✅ Complete file I/O API
- ✅ File creation/opening/deletion
- ✅ File status and attributes
- ✅ File allocation and advice
- ✅ Full filesystem operations

---

## Phase 3: Basic Polling and Timeouts (Week 5)

**Priority: MEDIUM-HIGH** - Required for event-driven applications

### Dependencies
- None

### Tasks

#### 3.1 Poll Operations
- [ ] Implement `IORING_OP_POLL_ADD` in `sqe.rs`
  - `PollAdd` struct with fd and events
  - Support `POLLIN`, `POLLOUT`, `POLLERR`, etc.
- [ ] Implement `IORING_OP_POLL_REMOVE` in `sqe.rs`
  - `PollRemove` struct with user_data to cancel
- [ ] Helper: `IoUring::poll_add(fd, events)` method
- [ ] Helper: `IoUring::poll_remove(user_data)` method
- [ ] Tests for poll operations

#### 3.2 Timeout Operations
- [ ] Implement `IORING_OP_TIMEOUT` in `sqe.rs`
  - `Timeout` struct with timespec, count, flags
  - Support absolute and relative timeouts
- [ ] Implement `IORING_OP_TIMEOUT_REMOVE` in `sqe.rs`
  - `TimeoutRemove` struct with user_data to cancel
- [ ] Implement `IORING_OP_LINK_TIMEOUT` for linked ops
- [ ] Add `Timespec` struct for timeout specification
- [ ] Helper: `IoUring::timeout(timespec, count)` method
- [ ] Tests for timeout operations

### Files to Modify/Create
- `src/sqe.rs` - Poll and timeout structs
- `src/lib.rs` - Add poll event constants (POLLIN, POLLOUT, etc.)
- `src/io_uring.rs` - Convenience methods
- `src/tests.rs` - Poll and timeout tests

### Deliverables
- ✅ Poll-based I/O support
- ✅ Timeout management
- ✅ Linked timeouts
- ✅ Event-driven programming support

---

## Phase 4: Networking Operations (Week 6-7)

**Priority: MEDIUM** - Required for network applications

### Dependencies
- Phase 1 (eventfd useful for network events)
- Phase 3 (polling useful for non-blocking I/O)

### Tasks

#### 4.1 Basic Send/Recv
- [ ] Implement `IORING_OP_SEND` in `sqe.rs`
  - `Send` struct with fd, buf, flags
  - Support MSG_* flags
- [ ] Implement `IORING_OP_RECV` in `sqe.rs`
  - `Recv` struct with fd, buf, flags
- [ ] Implement `IOSQE_BUFFER_SELECT` flag in `sqe.rs`
- [ ] Helper: `IoUring::send(fd, buf, flags)` method
- [ ] Helper: `IoUring::recv(fd, buf, flags)` method
- [ ] Tests for basic send/recv

#### 4.2 Message Send/Recv
- [ ] Implement `IORING_OP_SENDMSG` in `sqe.rs`
  - `SendMsg` struct with fd, msghdr, flags
  - Add `MsgHdr` struct for message headers
  - Add `IoVec` wrapper (different from Iovec)
- [ ] Implement `IORING_OP_RECVMSG` in `sqe.rs`
  - `RecvMsg` struct with fd, msghdr, flags
- [ ] Handle ancillary data (cmsg) in `MsgHdr`
- [ ] Tests for msg send/recv

#### 4.3 Connection Management
- [ ] Implement `IORING_OP_ACCEPT` in `sqe.rs`
  - `Accept` struct with fd, addr, addrlen, flags
  - Support `SOCK_*` flags
  - Support file descriptor allocation
- [ ] Implement `IORING_OP_CONNECT` in `sqe.rs`
  - `Connect` struct with fd, addr, addrlen
- [ ] Implement `IORING_OP_SHUTDOWN` in `sqe.rs`
  - `Shutdown` struct with fd, how
- [ ] Helper: `IoUring::accept(fd, flags)` method
- [ ] Helper: `IoUring::connect(fd, addr)` method
- [ ] Helper: `IoUring::shutdown(fd, how)` method
- [ ] Tests for connection management

### Files to Modify/Create
- `src/sqe.rs` - Network operation structs
- `src/lib.rs` - Add socket, message constants
- `src/io_uring.rs` - Convenience methods
- `src/tests.rs` - Network tests (use loopback for tests)

**Phase 4 Status: ✅ COMPLETE - December 31, 2024**

### Deliverables
- ✅ Complete networking API
- ✅ Socket send/recv with flags
- ✅ Message passing with ancillary data
- ✅ Connection management (accept, connect, shutdown)

---

## Phase 5: Advanced I/O Operations (Week 8)

**Priority: MEDIUM** - Specialized but important operations

### Dependencies
- Phase 2 (file operations)

### Tasks

#### 5.1 Splice and Tee
- [x] Implement `IORING_OP_SPLICE` in `sqe.rs`
  - `Splice` struct with fd_in, off_in, fd_out, off_out, len, flags
  - Support SPLICE_F_* flags
- [x] Implement `IORING_OP_TEE` in `sqe.rs`
  - `Tee` struct with fd_in, fd_out, len, flags
- [x] Helper: `IoUring::splice(fd_in, off_in, fd_out, off_out, len, flags)` method
- [x] Helper: `IoUring::tee(fd_in, fd_out, len, flags)` method
- [x] Tests for splice and tee (use pipes)

#### 5.2 Buffer Management
- [x] Implement `IORING_OP_PROVIDE_BUFFERS` in `sqe.rs`
  - `ProvideBuffers` struct with addr, len, bgid, bid, nbufs
  - Support buffer ring
- [x] Implement `IORING_OP_REMOVE_BUFFERS` in `sqe.rs`
  - `RemoveBuffers` struct with bgid, nr
- [x] Implement `IORING_OP_FREE_BUFFERS` in `sqe.rs`
  - `FreeBuffers` struct with bgid
- [x] Helper methods for buffer management
- [x] Tests for buffer management

#### 5.3 Cancel and Async Management
- [x] Implement `IORING_OP_ASYNC_CANCEL` in `sqe.rs`
  - `AsyncCancel` struct with user_data, flags
  - Support `IORING_ASYNC_CANCEL_*` flags
- [x] Helper: `IoUring::cancel(user_data)` method
- [x] Tests for async cancellation

#### 5.4 Ring Messaging
- [x] Implement `IORING_OP_MSG_RING` in `sqe.rs`
  - `MsgRing` struct with fd, user_data, flags, len
  - Support `IORING_MSG_RING_*` flags
- [x] Helper: `IoUring::msg_ring(fd, user_data, flags, len)` method
- [x] Tests for ring-to-ring messaging

### Files to Modify/Create
- `src/sqe.rs` - Advanced I/O structs
- `src/lib.rs` - Add splice, buffer constants
- `src/io_uring.rs` - Convenience methods
- `src/tests.rs` - Advanced I/O tests

**Phase 5 Status: ✅ COMPLETE - December 31, 2024**

### Deliverables
- ✅ Splice and tee operations
- ✅ Dynamic buffer management
- ✅ Async operation cancellation
- ✅ Ring-to-ring messaging

---

## Phase 6: Advanced Queue Management (Week 9)

**Priority: LOW-MEDIUM** - Performance optimizations and advanced features

### Dependencies
- All previous phases

### Tasks

#### 6.1 SQE Caching and Reuse
- [ ] Add SQE cache to `SubmissionQueue`
  - Reuse previously submitted SQEs to avoid initialization overhead
  - Method: `sq.reclaim_sqe()` to return SQE to cache
- [ ] Add `IoUring::get_sqe_with_reclaim()` method
- [ ] Tests for SQE caching

#### 6.2 Linked Operations
- [ ] Helper: `IoUring::link_sqe(sqe)` to mark as linked
  - Set `IOSQE_IO_LINK` flag
- [ ] Helper: `IoUring::hardlink_sqe(sqe)` to mark as hard-linked
  - Set `IOSQE_IO_HARDLINK` flag
- [ ] Helper: `IoUring::drain_sqe(sqe)` to mark for drain
  - Set `IOSQE_IO_DRAIN` flag
- [ ] Helper: `IoUring::make_async(sqe)` to mark as async
  - Set `IOSQE_ASYNC` flag
- [ ] Tests for linked operations

#### 6.3 User Data Management
- [ ] Add user data allocator to `IoUring`
  - Auto-generate unique user_data values
  - Method: `IoUring::alloc_user_data()`
  - Method: `IoUring::free_user_data(user_data)`
- [ ] Add user_data tracking struct
- [ ] Tests for user_data management

#### 6.4 Multi-Shot Operations
- [ ] Add `IOSQE_ASYNC` multi-shot support
  - Document multi-shot usage
  - Provide examples in tests
- [ ] Add support for `IORING_CQE_F_MORE` flag handling
- [ ] Tests for multi-shot poll, accept, etc.

### Files to Modify/Create
- `src/sq.rs` - SQE caching
- `src/io_uring.rs` - User data allocator, linked op helpers
- `src/sqe.rs` - Additional flag helpers
- `src/tests.rs` - Queue management tests

### Deliverables
- ✅ SQE caching for performance
- ✅ Linked operation helpers
- ✅ User data management
- ✅ Multi-shot operation support

---

## Phase 7: Advanced Setup and Features (Week 10)

**Priority: LOW** - Advanced features and kernel compatibility

### Dependencies
- All previous phases

### Tasks

#### 7.1 Custom Queue Sizes
- [ ] Support `IORING_SETUP_CQSIZE` flag
  - Allow custom CQ size in `IoUring::with_entries()`
  - Validate CQ size constraints
- [ ] Tests for custom CQ sizes

#### 7.2 SQ Polling Configuration
- [ ] Support `IORING_SETUP_SQPOLL` flag
  - Configure SQ poll thread
  - Add `IoUring::new_with_poll()` constructor
- [ ] Support `IORING_SETUP_SQ_AFF` flag
  - Set CPU affinity for poll thread
- [ ] Support `IORING_SETUP_R_DISABLED` flag
  - Create disabled ring, enable later
- [ ] Tests for SQ polling

#### 7.3 Advanced Setup Flags
- [ ] Support `IORING_SETUP_CLAMP` flag usage
- [ ] Support `IORING_SETUP_ATTACH_WQ` flag
  - Attach to existing work queue
- [ ] Support `IORING_SETUP_SUBMIT_ALL` flag
- [ ] Support `IORING_SETUP_COOP_TASKRUN` flag
- [ ] Support `IORING_SETUP_TASKRUN_FLAG` flag
- [ ] Tests for advanced setup

#### 7.4 Extended SQE/CQE Formats
- [ ] Support `IORING_SETUP_SQE128` flag
  - Use 128-byte SQEs with extra data
  - Update `io_uring_sqe` struct to include cmd field
- [ ] Support `IORING_SETUP_CQE32` flag
  - Use 32-byte CQEs with extra data
  - Update `io_uring_cqe` struct to include extra fields
- [ ] Add `IORING_F_SQE128`, `IORING_F_CQE32` feature detection
- [ ] Tests for extended formats

### Files to Modify/Create
- `src/io_uring.rs` - Setup flag support
- `src/lib.rs` - Update struct definitions
- `src/tests.rs` - Advanced setup tests

### Deliverables
- ✅ Custom queue sizing
- ✅ SQ polling configuration
- ✅ Advanced setup flags
- ✅ Extended SQE/CQE format support

---

## Phase 8: Feature Detection and Probing (Week 11)

**Priority: LOW-MEDIUM** - Kernel compatibility and conditional features

### Dependencies
- Phase 1 (basic probing)

### Tasks

#### 8.1 Feature Flags Detection
- [ ] Add `IoUring::features()` method
  - Returns bitmask of supported features
  - Parse `params.features` from setup
- [ ] Add `IoUring::has_feature(flag)` helper
- [ ] Implement feature flag checks:
  - `IORING_FEAT_SINGLE_MMAP` (already used)
  - `IORING_FEAT_NODROP`
  - `IORING_FEAT_SUBMIT_STABLE`
  - `IORING_FEAT_RW_CUR_POS`
  - `IORING_FEAT_CUR_PERSONALITY`
  - `IORING_FEAT_FAST_POLL`
  - `IORING_FEAT_POLL_32BITS`
  - `IORING_FEAT_SQPOLL_FIXED`
  - `IORING_FEAT_EXT_ARG`
  - `IORING_FEAT_NATIVE_WORKERS`
  - `IORING_FEAT_RSRC_TAGS`
  - `IORING_FEAT_CQE_SKIP`
  - `IORING_FEAT_LINKED_FILE`
  - `IORING_FEAT_REG_REG_RING`
  - And more...
- [ ] Tests for feature detection

#### 8.2 Conditional Feature Support
- [ ] Make features conditional based on kernel detection
  - Use features to enable/disable APIs
  - Graceful degradation on older kernels
- [ ] Add kernel version detection helper
- [ ] Document minimum kernel version for each feature
- [ ] Tests for conditional features

#### 8.3 Extended Enter Arguments
- [ ] Support `IORING_ENTER_EXT_ARG` flag
  - Use extended `io_uring_getevents_arg` struct
  - Support timeout with extended args
- [ ] Tests for extended enter arguments

### Files to Modify/Create
- `src/io_uring.rs` - Feature detection methods
- `src/lib.rs` - Add feature flag constants
- `src/tests.rs` - Feature detection tests

### Deliverables
- ✅ Complete feature detection API
- ✅ Conditional feature support
- ✅ Kernel compatibility layer
- ✅ Extended enter arguments

---

## Phase 9: Advanced Registration Features (Week 12)

**Priority: LOW** - Advanced resource management

### Dependencies
- Phase 1 (basic registration)

### Tasks

#### 9.1 Restriction Management
- [ ] Implement `IORING_REGISTER_RESTRICTIONS`
  - `Restrictions` struct with allowlist/denylist
  - Support opcode, register, and file restrictions
  - Methods: `register_restrictions()`, `unregister_restrictions()`
- [ ] Tests for restrictions

#### 9.2 Per-Buffer Rings
- [ ] Implement `IORING_REGISTER_PBUF_RING`
  - Register per-buffer ring for advanced buffer management
  - Methods: `register_pbuf_ring()`, `unregister_pbuf_ring()`
- [ ] Tests for buffer rings

#### 9.3 Worker Management
- [ ] Implement `IORING_REGISTER_IOWQ_MAX_WORKERS`
  - Configure worker thread limits
  - Methods: `register_iowq_max_workers()`
- [ ] Tests for worker management

#### 9.4 Sync Cancel
- [ ] Implement `IORING_REGISTER_SYNC_CANCEL`
  - Register for synchronous cancel notifications
- [ ] Tests for sync cancel

#### 9.5 Registered Ring FD
- [ ] Support `IORING_ENTER_REGISTERED_FD` flag
  - Use registered ring fd for lower overhead
- [ ] Tests for registered ring fd

### Files to Modify/Create
- `src/io_uring.rs` - Advanced registration methods
- `src/lib.rs` - Restriction and buffer ring constants
- `src/tests.rs` - Advanced registration tests

### Deliverables
- ✅ Operation restrictions
- ✅ Per-buffer ring support
- ✅ Worker thread management
- ✅ Advanced cancel mechanisms

---

## Phase 10: Testing, Documentation, and Optimization (Week 13-14)

**Priority: HIGH** - Quality assurance and polish

### Dependencies
- All previous phases

### Tasks

#### 10.1 Comprehensive Testing
- [ ] Add integration tests for all operations
- [ ] Add stress tests for queue operations
- [ ] Add benchmarks for critical paths
- [ ] Add tests for kernel compatibility
- [ ] Add tests for error handling

#### 10.2 Documentation
- [ ] Add rustdoc for all public APIs
- [ ] Add usage examples for each operation type
- [ ] Add tutorial/walkthrough for common patterns
- [ ] Document all unsafe blocks with safety invariants
- [ ] Document atomic ordering requirements

#### 10.3 Performance Optimization
- [ ] Profile critical paths
- [ ] Optimize memory access patterns
- [ ] Reduce allocations in hot paths
- [ ] Optimize atomic operations
- [ ] Add caching for frequently accessed data

#### 10.4 Examples
- [ ] Create `examples/file_io.rs` - File I/O example
- [ ] Create `examples/network.rs` - Networking example
- [ ] Create `examples/poll.rs` - Poll-based I/O example
- [ ] Create `examples/buffers.rs` - Registered buffers example
- [ ] Create `examples/echo_server.rs` - Echo server example

### Files to Modify/Create
- `src/*.rs` - Add documentation
- `examples/*.rs` - Create examples
- `tests/integration.rs` - Integration tests
- `tests/stress.rs` - Stress tests
- `benches/*.rs` - Benchmarks

### Deliverables
- ✅ Comprehensive test coverage
- ✅ Complete API documentation
- ✅ Performance optimizations
- ✅ Example programs

---

## Summary Timeline

| Phase | Duration | Priority | Dependencies |
|-------|----------|-----------|--------------|
| 1: Registration APIs | 2 weeks | HIGH | None |
| 2: File Operations | 2 weeks | HIGH | None |
| 3: Poll/Timeouts | 1 week | MED-HIGH | None |
| 4: Networking | 2 weeks | MEDIUM | 1, 3 |
| 5: Advanced I/O | 1 week | MEDIUM | 2 |
| 6: Queue Mgmt | 1 week | LOW-MED | All |
| 7: Advanced Setup | 1 week | LOW | All |
| 8: Feature Detection | 1 week | LOW-MED | 1 |
| 9: Advanced Reg | 1 week | LOW | 1 |
| 10: Testing/Docs | 2 weeks | HIGH | All |

**Total: 14 weeks (3.5 months)**

## MVP Definition

**Minimum Viable Product** (usable for real applications):
- ✅ Phases 1-5 complete
- ✅ Comprehensive testing
- ✅ Core documentation

This provides:
- Registration (buffers, files, eventfd)
- All common file operations
- Basic networking
- Polling and timeouts

## Success Criteria

For each phase, success is defined as:
1. All tasks in phase checklist completed
2. All tests passing
3. Documentation complete for new APIs
4. Code follows project style guidelines
5. `cargo clippy` and `cargo fmt` pass

## Risk Mitigation

1. **Missing rustix support**: Fall back to raw syscalls
2. **Kernel compatibility**: Add feature detection and graceful degradation
3. **Testing complexity**: Use loopback sockets, temp files for tests
4. **Documentation burden**: Document as you implement, not after
5. **Scope creep**: Stick to defined phases, defer extras

## Open Questions

1. Should we implement liburing-compatible API exactly?
2. Do we need async/await integration (future work)?
3. Should we expose more rustix features directly?
4. What's the target kernel version minimum?
5. Do we need Windows/macOS stubs (always error)?

## Next Steps

1. Review and approve this plan
2. Prioritize phases based on use cases
3. Start with Phase 1 (Registration APIs)
4. Create tracking issue for each phase
5. Set up CI for automated testing
