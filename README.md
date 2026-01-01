# io_urine

`io_urine` is a tiny, low-level Rust wrapper for Linux `io_uring`.

It aims to stay close to the metal (and far away from libc), using `rustix` for the syscall layer.

## Current status (no promises, just vibes)

| Thing | Status | Notes |
|---|---|---|
| Platform | Linux-only | Kernel 5.4+ recommended |
| Build | `cargo check` (passes) | Currently emits a warning in the `enter_ext_arg` path |
| Tests | `cargo test` (passes; 72 tests) | Mostly validates SQE field preparation |
| API stability | early / unstable | Expect breaking changes; bring snacks |
| `no_std` | not yet | Library uses `Vec`/`RefCell` today |
| Safety | `unsafe` involved | Shared memory + syscalls = you must respect invariants |

## What it does today

| Area | What you get |
|---|---|
| Ring setup | `IoUring::new`, `IoUring::with_entries`, plus a `SetupBuilder` (best-effort flags) |
| Submission | `get_sqe`, `submit`, `submit_and_wait`, `enter` |
| Completion | `peek_cqe`, `copy_cqes`, `cqe_seen` |
| Registration | buffers/files/eventfd/probe wrappers via `io_uring_register` |
| SQE preparation | Convenience methods on `IoUring` + opcode structs in `io_urine::sqe` implementing `PrepSqe` / `PrepSqeMut` |

This is not an async runtime integration. It’s the plumbing. You bring the callbacks, polling loop, and emotional support beverage.

## Try it

- Build: `cargo build`
- Tests: `cargo test`
- Example: `cargo run --example readme`

## More docs

- Repo conventions: `AGENTS.md`
- Codebase report: `lode/codebase-report.md`

## License

MIT. See `LICENSE`.

Yes, the name is a joke.
