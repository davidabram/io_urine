# io_urine Implementation Status

This document tracks io_urine's implementation status against the Linux io_uring standard.

## Executive Summary

**Overall Implementation Status: ~20% Complete**

io_urine provides a solid foundation with core ring management and basic I/O operations, but is missing most advanced features including networking, file operations, registration APIs, and advanced queue management.

## What's Implemented

### Core Infrastructure ✅

All fundamental ring buffer infrastructure is in place:

- **Ring Setup**
  - `IoUring::new(entries)` - Basic ring creation
  - `IoUring::with_entries(sq_entries, cq_entries)` - Custom queue sizes
  - Proper memory mapping via `RwMmap`
  - Support for SINGLE_MMAP (Kernel 5.4+)

- **Submission Queue (SQ) Management**
  - `get_sqe()` - Acquire SQE
  - `submit()` - Submit operations
  - `submit_and_wait()` - Submit and wait for completions
  - `enter()` - Low-level io_uring_enter access
  - Ring state tracking (head, tail, available space)
  - Wakeup flag support (`IORING_SQ_NEED_WAKEUP`)

- **Completion Queue (CQ) Management**
  - `peek_cqe()` - Peek at next completion
  - `copy_cqes()` - Batch copy completions
  - `cqe_seen()` - Mark completion as processed
  - Ring state tracking (events available, overflow)

- **Error Handling**
  - `InitError` - Setup/initialization errors
  - `EnterError` - Submit/enter errors
  - Proper errno conversion

- **Type System**
  - `io_uring_sqe` struct with all kernel fields
  - `io_uring_cqe` struct
  - `iovec` struct
  - `PrepSqe` and `PrepSqeMut` traits for operation preparation

### Constants ✅

All basic constants are defined (lib.rs:28-108):

- **Opcodes** (0-34): `IORING_OP_NOP`, `IORING_OP_READ`, `IORING_OP_WRITE`, etc.
- **SQE Flags**: `IOSQE_ASYNC`, `IOSQE_IO_DRAIN`, `IOSQE_IO_LINK`, etc.
- **Setup Flags**: `IORING_SETUP_IOPOLL`, `IORING_SETUP_SQPOLL`, etc.
- **Enter Flags**: `IORING_ENTER_GETEVENTS`, `IORING_ENTER_SQ_WAKEUP`, etc.
- **CQE Flags**: `IORING_CQE_F_BUFFER`, `IORING_CQE_F_MORE`, etc.
- **Feature Flags**: `IORING_F_SINGLE_MMAP`, `IORING_F_SQE128`, etc.
- **Ring Offsets**: `IORING_OFF_SQ_RING`, `IORING_OFF_CQ_RING`, etc.

### I/O Operations Implemented (7/35) ⚠️

Basic file I/O operations are supported:

| Opcode | Operation | Implemented | Location |
|---------|-----------|-------------|-----------|
| 0 | `IORING_OP_NOP` | ✅ | sqe.rs:61 |
| 1 | `IORING_OP_READV` | ✅ | sqe.rs:69 |
| 2 | `IORING_OP_WRITEV` | ✅ | sqe.rs:103 |
| 3 | `IORING_OP_FSYNC` | ✅ | sqe.rs:183 |
| 15 | `IORING_OP_CLOSE` | ✅ | sqe.rs:207 |
| 33 | `IORING_OP_WRITE` | ✅ | sqe.rs:160 |
| 34 | `IORING_OP_READ` | ✅ | sqe.rs:137 |

### Convenience Methods ✅

High-level methods on `IoUring`:

- `nop()` - Prepare NOP operation
- `read(fd, buf, offset)` - Prepare read operation
- `write(fd, buf, offset)` - Prepare write operation
- `close(fd)` - Prepare close operation
- `prepare()` / `prepare_mut()` - Generic operation preparation
- `sq_space_left()`, `is_sq_full()` - SQ state queries
- `cq_space_left()`, `is_cq_empty()` - CQ state queries

### Testing ✅

Basic test coverage (tests.rs):

- Ring creation tests
- NOP operation test
- CLOSE operation test
- Queue state tests

## What's NOT Implemented

### Missing I/O Operations (28/35) ❌

Major categories of missing operations:

#### File Operations
- `IORING_OP_READ_FIXED` (4) - Read from registered buffers
- `IORING_OP_WRITE_FIXED` (5) - Write to registered buffers
- `IORING_OP_SYNC_FILE_RANGE` (8) - Sync file range
- `IORING_OP_OPENAT` (20) - Open file
- `IORING_OP_CLOSE_DIRECT` (21) - Fast close
- `IORING_OP_FALLOCATE` - File allocation
- `IORING_OP_STATX` - File stat extended
- `IORING_OP_FADVISE` - File advice
- `IORING_OP_MADVISE` - Memory advice
- Filesystem operations: `IORING_OP_UNLINKAT`, `IORING_OP_RENAMEAT`, `IORING_OP_MKDIRAT`, `IORING_OP_SYMLINKAT`, `IORING_OP_LINKAT`

#### Networking Operations
- `IORING_OP_SENDMSG` (9) - Send message with headers
- `IORING_OP_RECVMSG` (10) - Receive message with headers
- `IORING_OP_ACCEPT` (13) - Accept connection
- `IORING_OP_CONNECT` (14) - Connect socket
- `IORING_OP_SEND` (18) - Simple send
- `IORING_OP_RECV` (19) - Simple receive
- `IORING_OP_SHUTDOWN` (26) - Shutdown socket

#### Polling and Event Operations
- `IORING_OP_POLL_ADD` (6) - Add poll event
- `IORING_OP_POLL_REMOVE` (7) - Remove poll event

#### Timeout Operations
- `IORING_OP_TIMEOUT` (11) - Set timeout
- `IORING_OP_TIMEOUT_REMOVE` (12) - Remove timeout
- `IORING_OP_LINK_TIMEOUT` - Link timeout to operation

#### Buffer Management
- `IORING_OP_ALLOC_BUFFERS` (16) - Allocate buffers
- `IORING_OP_FREE_BUFFERS` (17) - Free buffers
- `IORING_OP_PROVIDE_BUFFERS` (23) - Provide buffer ring
- `IORING_OP_REMOVE_BUFFERS` (24) - Remove buffers

#### Advanced I/O
- `IORING_OP_ASYNC_CANCEL` (14) - Cancel async operation
- `IORING_OP_SPLICE` (22) - Splice between files
- `IORING_OP_TEE` (25) - Tee to pipe
- `IORING_OP_MSG_RING` (32) - Ring-to-ring messaging

### Missing Registration APIs ❌

No support for `io_uring_register()` system call (io_uring_register(2)):

- `IORING_REGISTER_BUFFERS` - Register user buffers
- `IORING_UNREGISTER_BUFFERS` - Unregister buffers
- `IORING_REGISTER_FILES` - Register file set
- `IORING_UNREGISTER_FILES` - Unregister files
- `IORING_REGISTER_EVENTFD` - Register eventfd for notifications
- `IORING_UNREGISTER_EVENTFD` - Unregister eventfd
- `IORING_REGISTER_PROBE` - Probe kernel for supported operations
- `IORING_REGISTER_PERSONALITY` - Register personality
- `IORING_REGISTER_RESTRICTIONS` - Register operation restrictions
- `IORING_REGISTER_IOWQ_MAX_WORKERS` - Set worker thread limits
- `IORING_REGISTER_PBUF_RING` - Register per-buffer ring
- `IORING_REGISTER_SYNC_CANCEL` - Register cancel sync

**Impact**: Cannot use registered buffers/files for zero-copy I/O, cannot probe kernel capabilities.

### Missing Advanced Features ❌

#### SQE Flags
- `IOSQE_FIXED_FILE` (0) - Use registered file set
- `IOSQE_CQE_SKIP_SUCCESS` - Skip CQE on success
- `IOSQE_BUFFER_SELECT` - Auto-select buffer

#### Setup Flags (Unsupported)
- `IORING_SETUP_CQSIZE` - Custom CQ size (defined but not used)
- `IORING_SETUP_CLAMP` - Clamp parameters (defined but not used)
- `IORING_SETUP_ATTACH_WQ` - Attach to existing work queue (defined but not used)
- `IORING_SETUP_R_DISABLED` - Start disabled (defined but not used)
- `IORING_SETUP_SUBMIT_ALL` - Submit all SQEs
- `IORING_SETUP_COOP_TASKRUN` - Cooperative taskrun
- `IORING_SETUP_TASKRUN_FLAG` - Set taskrun flag
- `IORING_SETUP_SQE128` - Use 128-byte SQEs
- `IORING_SETUP_CQE32` - Use 32-byte CQEs

#### Enter Flags (Unsupported)
- `IORING_ENTER_SQ_WAIT` - Wait for SQ space
- `IORING_ENTER_EXT_ARG` - Extended arguments
- `IORING_ENTER_REGISTERED_FD` - Use registered ring fd
- `IORING_ENTER_GETEVENTS` is defined but not exposed for custom use

#### Queue Management
- No SQE reclamation/caching
- No CQE overflow handling beyond reporting
- No per-operation user_data tracking helpers
- No multi-shot operation support
- No linked/hard-linked operation helpers

#### Feature Detection
- No kernel feature probing
- No operation capability checking
- Hardcoded dependency on Kernel 5.4+ (SINGLE_MMAP)

### Missing Convenience APIs ❌

No higher-level helpers like liburing provides:

- Batch operations helpers
- Linked operation helpers
- Timeout helpers
- Buffer selection helpers
- Eventfd helpers
- Probing helpers

## Comparison with liburing

liburing provides a much more complete API:

### liburing Categories Implemented
- ✅ Queue initialization
- ✅ Basic SQE submission
- ✅ Basic CQE handling
- ✅ Basic I/O ops (read, write, fsync, close, nop)

### liburing Categories NOT Implemented
- ❌ Buffer registration
- ❌ File registration
- ❌ Eventfd integration
- ❌ Operation probing
- ❌ All networking operations
- ❌ Timeout management
- ❌ Advanced I/O (splice, tee, fallocate, etc.)
- ❌ Filesystem operations (openat, unlinkat, etc.)
- ❌ Poll operations
- ❌ Buffer ring management
- ❌ Restriction management

## Recommendations

### Immediate Priorities (To reach MVP)

1. **Registration APIs** - Essential for performance
   - `io_uring_register()` wrapper
   - Buffer registration/unregistration
   - File registration/unregistration
   - Eventfd registration

2. **Common File Operations**
   - `IORING_OP_OPENAT` - File opening
   - `IORING_OP_FALLOCATE` - File allocation
   - `IORING_OP_STATX` - File stats

3. **Basic Polling**
   - `IORING_OP_POLL_ADD` - Event polling

### Medium-Term Priorities (To be feature-complete)

4. **Networking Operations**
   - Send/recv (MSG and simple variants)
   - Accept/Connect

5. **Timeout Management**
   - `IORING_OP_TIMEOUT`
   - `IORING_OP_TIMEOUT_REMOVE`

6. **Advanced Queue Features**
   - Linked operations helpers
   - SQE caching/reuse
   - User_data helpers

### Long-Term Priorities (Full io_uring support)

7. **All remaining opcodes**
8. **Advanced setup flags** (SQE128, CQE32, etc.)
9. **Feature probing**
10. **Restriction management**
11. **Per-buffer rings**

## References

- [io_uring(7) manual page](https://man7.org/linux/man-pages/man7/io_uring.7.html)
- [io_uring_enter(2) manual page](https://man7.org/linux/man-pages/man2/io_uring_enter.2.html)
- [io_uring_register(2) manual page](https://man7.org/linux/man-pages/man2/io_uring_register.2.html)
- [liburing source](https://git.kernel.dk/cgit/liburing/tree/src/include/liburing/io_uring.h)
- [Tokio io-uring crate](https://docs.rs/io-uring/latest/io_uring/opcode/index.html)

## Notes

- All constants up to kernel 6.x are defined
- Memory ordering is correctly implemented (Acquire/Release barriers)
- The design follows liburing conventions where applicable
- Pure no_std, no libc approach is maintained
- Uses rustix for system calls (consistent with design goals)
