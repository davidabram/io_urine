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

This repo has a working core ring + basic operations + Phase 1 “registration”
APIs.

Implemented (high level):
- Ring setup + memory mapping (`IoUring::new`, `IoUring::with_entries`).
- SQ/CQ management (`get_sqe`, `submit`, `submit_and_wait`, `enter`,
  `peek_cqe`, `copy_cqes`, `cqe_seen`).
- A handful of SQE prep helpers (NOP, read/write, etc.).
- `io_uring_register` wrappers for buffers/files/eventfd/probe.
- Unit tests for the above (`cargo test` is currently green).

Not implemented (yet):
- Most io_uring opcodes (poll, timeouts, networking, filesystem ops like
  `OPENAT`/`STATX`, splice/tee/cancel, buffer rings, etc.).
- Higher-level “ergonomic” helpers (user_data allocators, multishot helpers,
  robust feature gating, etc.).
- Deep kernel compatibility work beyond basic probing.

For planning docs and a running implementation checklist, see `lode/`.

## What’s implemented (more detail)

### Core ring infrastructure
- `RwMmap` wrapper for `mmap`/`munmap` (`src/mmap.rs`).
- Submission queue / completion queue ring state (`src/sq.rs`, `src/cq.rs`).
- `IoUring` orchestrator (`src/io_uring.rs`).

### SQE prep helpers
Located in `src/sqe.rs`:
- `Nop`
- `Read` / `Write`
- `Readv` / `Writev`
- `Fsync`
- `Close`
- `ReadFixed` / `WriteFixed` (for registered buffers)

### Registration APIs (Phase 1)
Located in `src/io_uring.rs`:
- Buffers: `register_buffers`, `unregister_buffers`
- Files: `register_files`, `unregister_files`, `register_files_update`
- Eventfd: `register_eventfd`, `unregister_eventfd`, `register_eventfd_async`
- Probing: `probe` (returns `Probe`), `opcode_supported`

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

## Contributing

- Read `AGENTS.md` for repo conventions.
- Prefer small, test-backed changes.
- When adding new opcodes, add a `PrepSqe`/`PrepSqeMut` type and a targeted test
  (even if it’s just “prep fields are correct”).

## License

MIT. See `LICENSE`.

Yes, it’s for shit and giggles.
