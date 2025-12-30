# Agent Guide — io_urine

## Project overview

`io_urine` is a small, low-level Rust wrapper for Linux `io_uring`.

Core constraints / goals:
- Linux-only (requires `io_uring` support; kernel 5.4+ recommended).
- Keep library code `no_std`-friendly (prefer `core`; tests use `std`).
- No libc dependency.
- Keep production deps minimal (currently: `rustix`).

Repository layout (most-touched files):
- `src/lib.rs`: public exports + kernel constants + core structs.
- `src/io_uring.rs`: `IoUring` type, submission/enter, registration APIs.
- `src/sqe.rs`: operation “prep” types implementing `PrepSqe`/`PrepSqeMut`.
- `src/sq.rs`, `src/cq.rs`: SQ/CQ ring management and atomics.
- `src/mmap.rs`: `RwMmap` wrapper around `mmap`/`munmap`.
- `src/err.rs`: `InitError` / `EnterError`.
- `src/tests.rs`: unit tests (compiled via `#[cfg(test)]`).

## Build / test / lint commands

Run these from the repo root (or use `--manifest-path Cargo.toml`).

Build:
- `cargo build`
- `cargo build --release`

Tests (all):
- `cargo test`

Tests (single test):
- `cargo test test_nop` (substring match)
- `cargo test tests::tests::test_nop` (fully qualified)
- `cargo test test_register_unregister_buffers`
- `cargo test -- --nocapture` (show test stdout)
- `RUST_BACKTRACE=1 cargo test <name>` (better panic traces)

Formatting:
- `cargo fmt`
- `cargo fmt -- --check`

Lint:
- `cargo clippy`
- `cargo clippy -- -D warnings`

Docs:
- `cargo doc --no-deps`
- `cargo doc --open` (may require a local browser)

## Code style guidelines

### Formatting
- Run `cargo fmt` for all Rust changes.
- There is no repo-local `rustfmt.toml`; keep lines reasonably short
  (~80 cols) for readability, especially doc comments.

### Clippy
- The crate enables `#![warn(clippy::all, clippy::pedantic)]` in `src/lib.rs`.
  Prefer fixes over new `#[allow(...)]` attributes.
- If an `allow` is unavoidable, keep it narrowly scoped and documented.

### Imports
Prefer this import order (top to bottom):
1. `core::*`
2. external crates (usually `rustix::*`)
3. internal modules (`crate::*`)

Example:
```rust
use core::ffi::c_void;
use core::sync::atomic::Ordering;

use rustix::fd::AsFd;
use rustix::io_uring;

use crate::err::InitError;
```

### Naming
- Types/traits/enums: `PascalCase`.
- Functions/methods/vars: `snake_case`.
- Constants: `SCREAMING_SNAKE_CASE`.

### Types & ABI correctness
- Anything passed to the kernel must be `#[repr(C)]` and field-for-field
  compatible with the kernel ABI.
- Keep kernel constants in `src/lib.rs` and ensure values match upstream
  Linux headers / `rustix`.
- Use `u32` for ring indices/counters/flags that are kernel `u32`.
- Use `usize` for Rust indexing/lengths.
- Prefer explicit `try_into()` when converting `usize -> u32`; treat
  conversion failure as `InitError::InvalidParameters`.
- Prefer `OwnedFd` for ownership and `AsFd`/`BorrowedFd` for borrowing.
- When a kernel interface uses an `int fd`, prefer `i32` in the public API.

### Error handling
- Setup/registration/mmap errors: `InitError`.
- Enter/submit errors: `EnterError`.
- Prefer mapping `rustix` errors directly:
  - `map_err(InitError::SyscallFailed)` for setup syscalls
  - `map_err(InitError::RegisterFailed)` for `io_uring_register`
- Avoid allocations and `String` in error paths.
- Avoid panics in library code; tests may `panic!`.

### Unsafe code
- Every `unsafe` block must have a `// SAFETY:` comment explaining:
  - what invariants are assumed
  - why the pointer/offset math is valid
  - why aliasing rules are not violated
- Keep `unsafe` localized to low-level modules (ring/mmap/syscall glue).

### Atomics & ring semantics
- Respect the ordering already used in `src/sq.rs` and `src/cq.rs`.
  (If you change these, you must understand and preserve the kernel
  synchronization contract.)
- Use wrapping arithmetic for ring counters (`u32::wrapping_add/sub`).

### SQE preparation conventions
- SQEs are reused. `IoUring::get_sqe` resets the SQE to `Default`.
  If you ever bypass `get_sqe`, you must ensure no stale fields leak.
- A `PrepSqe`/`PrepSqeMut` implementation should set:
  - `opcode`
  - all required fields for that opcode
  - any flags (`sqe.flags`, `sqe.rw_flags`, `sqe.buf_index`, etc.)
- Use `PrepSqeMut` when you must take a `&mut [u8]` and store a mutable
  buffer pointer in the SQE.
- For fixed buffers (`READ_FIXED`/`WRITE_FIXED`), always set `sqe.buf_index`.
- For registered files, set `IOSQE_FIXED_FILE` and interpret `sqe.fd` as
  an index into the registered file table.

### Dependencies
- Production deps should stay minimal (currently only `rustix`).
  Do not add new production dependencies without explicit approval.
- Dev-deps are OK when they improve test quality (current: `tempfile`,
  `pretty_assertions`).

### Testing conventions
- Tests live in `src/tests.rs` and are nested as `tests::tests::*`.
- Use `tempfile::NamedTempFile` to obtain valid fds for op prep tests.
- Use `rustix::event::eventfd` for eventfd-related tests.
- For kernel-dependent features, it’s OK to treat `Errno::NOSYS` or
  `Errno::INVAL` as “feature not available” and return early.
- Avoid flaky timing- or network-dependent tests.

## Other agent rule files

- No Cursor rules found (`.cursor/rules/` or `.cursorrules`).
- No Copilot instructions found (`.github/copilot-instructions.md`).
