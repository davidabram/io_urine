# io_urine Codebase Report

This is a snapshot-style walkthrough of what the repository *actually contains today*, where the sharp edges are, and where to look when you’re trying to understand why your SQE is doing interpretive dance.

## At a glance

| Topic | Current state | Notes (aka “why is it like this”) |
|---|---|---|
| What it is | Low-level Rust wrapper around Linux `io_uring` | Closer to “here’s a ring, good luck” than “here’s an async runtime” |
| Platform | Linux-only | If your OS doesn’t have `io_uring`, this crate also doesn’t have `io_uring` |
| Syscall layer | `rustix` (no libc) | See `Cargo.toml` |
| Build | `cargo check` (passes) | One warning in `src/io_uring.rs` about unused `sigmask_ptr` |
| Tests | `cargo test` (passes; 72 tests) | Mostly SQE field-prep tests; very little end-to-end kernel execution |
| `no_std` | Not currently | Crate uses `Vec`/`RefCell` in library code; no `#![no_std]` gate yet |
| Safety story | Unsafe-heavy | The whole point of `io_uring` is shared memory and syscalls. Whee. |

## Repository map

| Path | What lives here | Why you care |
|---|---|---|
| `Cargo.toml` | Crate metadata + deps | Declares `rustix` (prod) and `tempfile`/`pretty_assertions` (dev) |
| `src/lib.rs` | Public exports + ABI structs + constants | Exposes `IoUring`, SQE/CQE structs, many Linux constants |
| `src/io_uring.rs` | `IoUring` type and most methods | Ring setup, submit/enter, registration wrappers, convenience methods |
| `src/sq.rs` | Submission queue ring logic | Head/tail tracking, SQE allocation, kernel tail update |
| `src/cq.rs` | Completion queue ring logic | CQE peeking, head advance, overflow counter |
| `src/sqe.rs` | SQE “prep” helpers + small helper types | Implements `PrepSqe`/`PrepSqeMut` for many opcodes |
| `src/cqe.rs` | CQE helpers | Turns `cqe.res` into `Result<i32, Errno>` and exposes flags |
| `src/mmap.rs` | `RwMmap` wrapper | `mmap`/`munmap` wrapper used for ring mappings |
| `src/err.rs` | Error types | `InitError` / `EnterError` with errno mapping |
| `src/tests.rs` | Unit tests | Prep correctness checks and a couple tiny “submit” sanity tests |
| `examples/readme.rs` | Minimal example | Shows ring creation, `nop`, `submit` |
| `lode/` | Project knowledge docs | Planning/status docs + this report |

## Core API surface (the parts you’ll actually call)

| Type / module | Role | Where |
|---|---|---|
| `IoUring` | Owns the ring FD + mmaps; provides submission/completion API | `src/io_uring.rs` |
| `SetupBuilder` | Builder for configuring setup flags/queue sizes | `src/io_uring.rs` |
| `SubmissionQueue` / `CompletionQueue` | SQ/CQ ring memory management | `src/sq.rs`, `src/cq.rs` |
| `PrepSqe` / `PrepSqeMut` | Trait-based SQE preparation | `src/lib.rs`, implementations in `src/sqe.rs` |
| `sqe::*` | Per-opcode prep structs | `src/sqe.rs` |
| `cqe::*` | Result + flag helpers | `src/cqe.rs` |

### Typical flow

1. Create a ring: `IoUring::new(entries)` or `IoUring::with_entries(sq, cq)`.
2. Get an SQE: `ring.get_sqe()` (or use a convenience method like `ring.nop()`).
3. Prep the SQE: call a `PrepSqe`/`PrepSqeMut` op (or use the built-in convenience wrappers).
4. Submit: `ring.submit()` or `ring.submit_and_wait(n)`.
5. Consume CQEs: `ring.peek_cqe()` / `ring.copy_cqes(n)` and then `ring.cqe_seen(cqe)`.

## What’s implemented (practical view)

This is based on the code in `src/io_uring.rs` and `src/sqe.rs`, not the aspirational planning docs.

| Area | Present in code | What that means in practice |
|---|---|---|
| Ring setup + mmaps | yes | `io_uring_setup` + separate mmaps for SQ, CQ, SQEs |
| Submit/enter | yes | `submit`, `submit_and_wait`, `enter` |
| CQE helpers | yes | Convert `cqe.res` to `Result<i32, Errno>` |
| Registration wrappers | yes | buffers/files/eventfd/probe via `io_uring_register` |
| SQE prep helpers | yes (many) | Lots of opcode structs exist and set fields |
| Examples | yes (tiny) | `examples/readme.rs` runs `nop` and submits |

## Reality checks / sharp edges

This section is intentionally blunt, because your future self deserves honesty.

| Item | Risk level | Why | Where |
|---|---:|---|---|
| “`no_std`-friendly” claim | Medium | Library code uses `Vec`/`RefCell`; there’s no `#![no_std]` gate | `src/io_uring.rs`, `src/lib.rs` |
| Advanced/extended formats | High | SQE/CQE structs are fixed-size; “SQE128/CQE32” would need ABI-aware layouts | `src/lib.rs`, `src/io_uring.rs` |
| Extended enter arguments | High | There’s an unused `sigmask_ptr` warning, and the struct layout is likely incomplete | `src/io_uring.rs`, `src/lib.rs` |
| Test coverage shape | Medium | Tests validate field prep more than kernel behavior | `src/tests.rs` |

If you want one actionable takeaway: treat *most* of the “fancy” APIs as “compiles and has tests for field assignment”, not “production verified against every kernel config.”

## Suggested next cleanups (if you feel like adulting)

| Task | Why it matters | Candidate starting point |
|---|---|---|
| Add `no_std` gating (and/or `alloc`) | Makes the crate match its stated goal | `src/lib.rs` crate attributes, replace `Vec` usage or gate it |
| Validate SQE field mappings vs kernel ABI | Prevents subtle “works on my machine” failures | `src/sqe.rs` prep structs, especially networking + polling |
| Decide on extended-format support strategy | Avoid accidental footguns in `SetupBuilder` | `src/io_uring.rs` setup flags + `io_uring_sqe`/`io_uring_cqe` layouts |
| Expand end-to-end tests | Prep tests are great; execution tests are better | Add small real syscall tests per opcode (skip on `NOSYS`/`INVAL`) |

## Related lodes

- Planning: `lode/implementation-plan.md`
- Status tracking: `lode/implementation-status.md`
- Lode concept overview: `lode/lode-overview.md`
