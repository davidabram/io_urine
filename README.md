# io_urine

`io_urine` is a tiny, low-level Rust wrapper for Linux `io_uring`.

It is also a vibe-coded project: this repo exists to see whether a pack of
agentic coding tools can incrementally assemble a working `io_uring` library
without immediately setting the computer on fire.

If you’re looking for a polished, production-grade async runtime integration,
this is not that. If you’re looking for a small codebase where the unsafe bits
are at least trying their best, welcome.

## Goals

- Pure Rust (no libc dependency).
- `no_std`-friendly library code (tests use `std`).
- Minimal production dependencies (currently just `rustix`).
- Close-to-the-metal API (liburing-ish naming where it helps).

## Status

This repo has a working core ring + basic operations + Phase 1 (registration
APIs) + Phase 2 (file operations) + Phase 3 (poll and timeouts) + 
Phase 4 (networking operations) + Phase 5 (advanced I/O operations) +
Phase 6 (advanced queue management) + Phase 7 (advanced setup and features) +
Phase 8 (feature detection and probing).

Implemented (high level):
- Ring setup + memory mapping (`IoUring::new`, `IoUring::with_entries`, `SetupBuilder`).
- SQ/CQ management (`get_sqe`, `submit`, `submit_and_wait`, `enter`,
  `peek_cqe`, `copy_cqes`, `cqe_seen`).
- SQE prep helpers (NOP, read/write, fsync, close, fixed buffers, file ops, poll/timeout,
  networking, advanced I/O).
- `io_uring_register` wrappers for buffers/files/eventfd/probe.
- File operations (`openat`, `statx`, `fallocate`, `fadvise`, `madvise`,
  `unlinkat`, `renameat`, `mkdirat`, `symlinkat`, `linkat`, `close_direct`).
- Poll and timeout operations (`poll_add`, `poll_remove`, `timeout`, `timeout_relative`,
  `timeout_absolute`, `timeout_remove`, `link_timeout`).
- Networking operations (`send`, `recv`, `sendmsg`, `recvmsg`, `accept`, `connect`, `shutdown`).
- Advanced I/O operations (`splice`, `tee`, `provide_buffers`, `remove_buffers`, 
  `free_buffers`, `cancel`, `msg_ring`).
- Advanced queue management (SQE caching, linked operations, user data allocation, 
  multishot operations).
- Advanced setup features (SetupBuilder with all IORING_SETUP_* flags, 
  custom queue sizes, SQ polling, extended SQE/CQE formats).
- Feature detection and probing (kernel version, feature flags, conditional support).
- CQE result parsing helpers.
- Comprehensive test coverage for feature detection.

Not implemented (yet):
- Advanced registration features (restrictions, buffer rings, worker management) - Phase 9+.

For planning docs and a running implementation checklist, see `lode/`.

## What’s implemented (more detail)

### Core ring infrastructure
- `RwMmap` wrapper for `mmap`/`munmap` (`src/mmap.rs`).
- Submission queue / completion queue ring state (`src/sq.rs`, `src/cq.rs`).
- `IoUring` orchestrator (`src/io_uring.rs`).
- `SetupBuilder` for advanced ring configuration (`src/io_uring.rs`).

### SQE prep helpers
Located in `src/sqe.rs`:

**Basic I/O:**
- `Nop`
- `Read` / `Write`
- `Readv` / `Writev`
- `ReadFixed` / `WriteFixed` (for registered buffers)
- `Fsync`
- `Close`

**File operations (Phase 2):**
- `OpenAt` (open file relative to directory fd)
- `CloseDirect` (close registered/fixed file)
- `Statx` (extended file status)
- `Fallocate` (preallocate file space)
- `Fadvise` (file access pattern advice)
- `Madvise` (memory access pattern advice)
- `UnlinkAt` (unlink/delete file)
- `RenameAt` (rename file)
- `MkdirAt` (create directory)
- `SymlinkAt` (create symbolic link)
- `LinkAt` (create hard link)

**Poll and timeout operations (Phase 3):**
- `PollAdd` (add poll on file descriptor with events)
- `PollRemove` (remove poll operation by user_data)
- `Timeout` (timeout with count and flags, relative/absolute helpers)
- `TimeoutRemove` (remove timeout by user_data)
- `LinkTimeout` (linked timeout for operation chains)

### Registration APIs (Phase 1)
Located in `src/io_uring.rs`:
- Buffers: `register_buffers`, `unregister_buffers`
- Files: `register_files`, `unregister_files`, `register_files_update`
- Eventfd: `register_eventfd`, `unregister_eventfd`, `register_eventfd_async`
- Probing: `probe` (returns `Probe`), `opcode_supported`

### File operation APIs (Phase 2)
Located in `src/io_uring.rs`, convenience wrappers for file operations:
- `openat`, `close_direct`
- `statx` (extended file metadata)
- `fallocate`, `fadvise`, `madvise`
- `unlinkat`, `unlink`
- `renameat`, `rename`
- `mkdirat`, `mkdir`
- `symlinkat`, `symlink`
- `linkat`, `link`

### Poll and timeout APIs (Phase 3)
Located in `src/io_uring.rs`, convenience wrappers for event-driven operations:
- `poll_add(fd, events)` - add poll on file descriptor
- `poll_remove(user_data)` - remove poll operation
- `timeout(ts, count, flags)` - timeout with parameters
- `timeout_relative(ts)` / `timeout_absolute(ts)` - timeout helpers
- `timeout_remove(user_data)` - remove timeout operation
- `link_timeout(ts, flags)` - linked timeout for operation chains

### CQE helpers (Phase 2)
Located in `src/cqe.rs`:
- `cqe_res_to_result` (convert CQE result to `Result<i32, Errno>`)
- `cqe_result_to_result` (convenience wrapper for CQE)

### Advanced setup features (Phase 7)
Located in `src/io_uring.rs`:
- `SetupBuilder` - Fluent API for advanced io_uring configuration
- Queue size customization (`sq_entries`, `cq_entries`)
- SQ polling setup (`sqpoll`, `sq_affinity`, `sq_thread_idle`)
- Advanced flags (`clamp`, `attach_wq`, `disabled`, `submit_all`, 
  `coop_taskrun`, `taskrun_flag`)
- Extended formats (`sqe128`, `cqe32`) for 128-byte SQEs and 32-byte CQEs
- Backward compatible: existing constructors now use SetupBuilder internally

### Advanced queue management (Phase 6)
Located in `src/io_uring.rs`:
- SQE caching and reuse (`get_sqe_with_reclaim`, `reclaim_sqe`)
- Linked operations (`link_sqe`, `hardlink_sqe`, `drain_sqe`, `make_async`)
- User data allocation (`alloc_user_data`, `free_user_data`, `set_sqe_user_data`)
- Multi-shot operations (`poll_add_multishot`, `accept_multishot`, 
  `cancel_multishot`, `cqe_has_more`)

## Building, testing, linting

Run from repo root:

Build:
- `cargo build`
- `cargo build --release`

Tests:
- `cargo test`
- Single test (substring): `cargo test test_nop`
- Single test (fully qualified): `cargo test tests::tests::test_nop`
- With output: `cargo test -- --nocapture`

Formatting:
- `cargo fmt`
- `cargo fmt -- --check`

Lint:
- `cargo clippy`
- `cargo clippy -- -D warnings`

## Notes / footguns

- Registered buffers may require a sufficiently high `RLIMIT_MEMLOCK`.
- This is Linux-only and assumes a kernel new enough to support the features
  being used.
- `io_uring` is fundamentally a shared-memory + syscalls interface; the code
  uses `unsafe` for pointer/offset work. Keep unsafe blocks small and always
  explain invariants.
- Advanced setup flags (SQE128, CQE32, SQPOLL, etc.) require kernel 5.11+ 
  and may need additional privileges or specific kernel configurations.
- Use `SetupBuilder` for advanced configuration; the simple constructors (`IoUring::new`,
  `IoUring::with_entries`) remain available for basic use cases.

## Contributing

- Read `AGENTS.md` for repo conventions.
- Prefer small, test-backed changes.
- When adding new opcodes, add a `PrepSqe`/`PrepSqeMut` type and a targeted test
  (even if it's just "prep fields are correct").
- For poll operations, use event flag constants from `lib.rs` (POLLIN, POLLOUT, etc.).
- For timeouts, use the `Timespec` struct and helper methods for relative/absolute timeouts.
- For advanced setup features, extend `SetupBuilder` with new fluent methods and add 
  comprehensive tests that handle both success and expected failure modes.

## License

MIT. See `LICENSE`.

Yes, it’s for shit and giggles.
