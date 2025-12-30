# LODE Plan: Zig IoUring to Rust io_urine Port

## Overview

This LODE document provides a comprehensive plan for porting Zig's `std.os.linux.IoUring` implementation to Rust, creating the `io_urine` library. The port aims to be idiomatic Rust while maintaining the efficiency and design principles of the original Zig implementation.

## Project Goals

1. **Core Objective**: Create a pure Rust, `no_std`, no libc io_uring library
2. **Inspiration**: Zig's `std.os.linux.IoUring.zig` (v0.15.x)
3. **Key Characteristics**:
   - No standard library dependency (except in tests)
   - No libc dependency
   - Single dependency: `rustix` (Rust equivalent of Zig's `std.os`)
   - Kernel 5.4+ support (uses `SINGLE_MMAP` feature)

## Architecture Overview

### Core Components

1. **IoUring**: Main entry point; owns file descriptor and memory mappings
2. **SubmissionQueue (SQ)**: Manages submission ring and Submission Queue Entries (SQEs)
3. **CompletionQueue (CQ)**: Manages completion ring and Completion Queue Entries (CQEs)
4. **RwMmap**: Safe wrapper around mmap/munmap with bounds checking
5. **PrepSqe Trait**: Interface for preparing SQE operations
6. **Error Types**: Custom error enums for initialization and operations

### Memory Layout

- Single mmap for SQ and CQ rings (Kernel 5.4+ `SINGLE_MMAP` feature)
- Separate mmap for SQE array
- Kernel provides offsets; validate and use them

## Implementation Phases

### Phase 1: Project Setup and Foundation

#### 1.1 Initialize Project Structure
```
io_urine/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── io_uring.rs
│   ├── sq.rs
│   ├── cq.rs
│   ├── sqe.rs
│   ├── cqe.rs
│   ├── mmap.rs
│   └── err.rs
├── examples/
│   └── readme.rs
├── tests/
│   └── nop.rs
└── lode/
    └── io_urine_plan.md
```

#### 1.2 Cargo.toml Configuration
```toml
[package]
name = "io_urine"
version = "0.1.0"
edition = "2021"
description = "Pure Rust, no_std, no libc io_uring library"

[dependencies]
rustix = { version = "0.38", features = ["io_uring", "mm"] }

[dev-dependencies]
pretty_assertions = "1.4"
tempfile = "3.24"

[features]
default = []
std = ["rustix/std"]

[profile.dev]
opt-level = 1

[profile.release]
opt-level = 3
lto = true
```

### Phase 2: Core Types and Constants

#### 2.1 io_uring Constants (from Linux headers)
- `IORING_SETUP_IOPOLL`: Poll for I/O
- `IORING_SETUP_SQPOLL`: Use kernel thread for submission
- `IORING_SETUP_SQ_AFF`: Affinity for SQ thread
- `IORING_ENTER_GETEVENTS`: Wait for completion events
- `IORING_ENTER_SQ_WAKEUP`: Wakeup SQ poll thread
- `IORING_OFF_SQ_RING`: Offset for SQ ring
- `IORING_OFF_CQ_RING`: Offset for CQ ring  
- `IORING_OFF_SQES`: Offset for SQE array

#### 2.2 Ring Offset Structures
```rust
#[repr(C)]
pub struct io_sqring_offsets {
    pub head: u32,
    pub tail: u32,
    pub ring_mask: u32,
    pub ring_entries: u32,
    pub flags: u32,
    pub dropped: u32,
    pub array: u32,
}

#[repr(C)]
pub struct io_cqring_offsets {
    pub head: u32,
    pub tail: u32,
    pub ring_mask: u32,
    pub ring_entries: u32,
    pub flags: u32,
    pub overflow: u32,
    pub cqes: u32,
}
```

#### 2.3 io_uring_params Structure
```rust
#[repr(C)]
pub struct io_uring_params {
    pub sq_entries: u32,
    pub cq_entries: u32,
    pub flags: u32,
    pub sq_thread_cpu: u32,
    pub sq_thread_idle: u32,
    pub features: u32,
    pub resv: [u32; 4],
    pub sq_off: io_sqring_offsets,
    pub cq_off: io_cqring_offsets,
}
```

### Phase 3: Memory Mapping (RwMmap)

#### 3.1 RwMmap Wrapper
- Create safe wrapper around mmap/munmap
- Bounds checking for all offset-based memory accesses
- Drop implementation for automatic cleanup
- Support read-write and read-only mappings

#### 3.2 Key Methods
```rust
impl RwMmap {
    pub fn new(fd: RawFd, offset: usize, size: usize, writable: bool) -> Result<Self, InitError>;
    
    pub fn as_ptr(&self) -> *mut c_void;
    
    pub fn as_slice(&self, offset: usize, size: usize) -> &[u8];
    
    pub fn as_slice_mut(&mut self, offset: usize, size: usize) -> &mut [u8];
}
```

### Phase 4: Submission Queue (SQ) Implementation

#### 4.1 Submission Queue Entry (SQE)
```rust
#[repr(C)]
#[derive(Debug)]
pub struct io_uring_sqe {
    pub opcode: u8,
    pub flags: u8,
    pub ioprio: u16,
    pub fd: i32,
    pub off: u64,
    pub addr: u64,
    pub len: u32,
    pub rw_flags: i32,
    pub user_data: u64,
    pub buf_index: u16,
    pub personality: u16,
    pub splice_fd_in: i32,
    pub __pad2: [u64; 3],
}
```

#### 4.2 SQE Operations
- `opcode`: IORING_OP_* constants
- `flags`: IOSQE_* flags
- `user_data`: User-provided data returned in CQE

#### 4.3 Submission Queue Ring
```rust
pub struct SubmissionQueue {
    mmap: RwMmap,
    ptr: *mut c_void,
    sqe_ptr: *mut io_uring_sqe,
    sqe_mask: u32,
    sqe_entries: u32,
    head: AtomicU32,
    tail: AtomicU32,
    array: *mut u32,
    flags: AtomicU32,
    dropped: AtomicU32,
}
```

### Phase 5: Completion Queue (CQ) Implementation

#### 5.1 Completion Queue Entry (CQE)
```rust
#[repr(C)]
#[derive(Debug)]
pub struct io_uring_cqe {
    pub user_data: u64,
    pub res: i32,
    pub flags: u32,
}
```

#### 5.2 Completion Queue Ring
```rust
pub struct CompletionQueue {
    mmap: RwMmap,
    ptr: *mut c_void,
    cqe_ptr: *mut io_uring_cqe,
    cqe_mask: u32,
    cqe_entries: u32,
    head: AtomicU32,
    tail: AtomicU32,
    overflow: AtomicU32,
}
```

### Phase 6: SQE Preparation Methods (PrepSqe Trait)

#### 6.1 PrepSqe Trait Definition
```rust
pub trait PrepSqe {
    fn prep(&self, sqe: &mut io_uring_sqe);
}
```

#### 6.2 Operation Constants
- `IORING_OP_NOP`: No operation
- `IORING_OP_READV`: Vector read
- `IORING_OP_WRITEV`: Vector write
- `IORING_OP_READ`: Single buffer read
- `IORING_OP_WRITE`: Single buffer write
- `IORING_OP_FSYNC`: File sync
- `IORING_OP_POLL_ADD`: Add poll request
- `IORING_OP_POLL_REMOVE`: Remove poll request
- `IORING_OP_SYNC_FILE_RANGE`: Sync file range
- `IORING_OP_SENDMSG`: Send message
- `IORING_OP_RECVMSG`: Receive message
- `IORING_OP_TIMEOUT`: Timeout
- `IORING_OP_TIMEOUT_REMOVE`: Remove timeout
- `IORING_OP_ACCEPT`: Accept connection
- `IORING_OP_CONNECT`: Connect socket
- `IORING_OP_CLOSE`: Close file descriptor

### Phase 7: Main IoUring Structure

#### 7.1 IoUring Core Structure
```rust
pub struct IoUring {
    fd: RawFd,
    params: io_uring_params,
    sq_mmap: RwMmap,
    cq_mmap: RwMmap,
    sqe_mmap: RwMmap,
    sq: SubmissionQueue,
    cq: CompletionQueue,
}
```

#### 7.2 Core Methods
```rust
impl IoUring {
    pub fn new(entries: u32) -> Result<Self, InitError>;
    
    pub fn get_sqe(&mut self) -> Option<&mut io_uring_sqe>;
    
    pub fn submit(&mut self) -> Result<usize, EnterError>;
    
    pub fn submit_and_wait(&mut self, wait_count: usize) -> Result<usize, EnterError>;
    
    pub fn peek_cqe(&mut self) -> Option<&io_uring_cqe>;
    
    pub fn copy_cqes(&mut self, count: usize) -> &[io_uring_cqe];
    
    pub fn cqe_seen(&mut self, cqe: &io_uring_cqe);
    
    pub fn enter(&mut self, to_submit: u32, wait_count: u32, flags: u32, 
                 sig: Option<&sigset_t>) -> Result<usize, EnterError>;
}
```

### Phase 8: Error Handling

#### 8.1 Error Types
```rust
pub mod err {
    #[derive(Debug)]
    pub enum InitError {
        UnsupportedKernel,
        MmapFailed(Errno),
        FcntlFailed(Errno),
        SyscallFailed(Errno),
        InvalidParameters,
    }
    
    #[derive(Debug)]
    pub enum EnterError {
        SyscallFailed(Errno),
        BadOffset,
    }
}
```

### Phase 9: Atomic Operations and Memory Ordering

#### 9.1 Memory Ordering Requirements
- `sq.head`: read with `Acquire`
- `sq.tail`: write with `Release`
- `cq.head`: write with `Release`
- `cq.tail`: read with `Acquire`
- `sq.flags`: read with `Relaxed`

#### 9.2 Wrapping Arithmetic
Ring indices wrap around every 2^32 operations. Always use:
```rust
let next = self.sqe_tail.wrapping_add(1);
let pending = tail.wrapping_sub(head);
```

### Phase 10: Testing Strategy

#### 10.1 Unit Tests
- Test atomic operations
- Test ring index wrapping
- Test SQE preparation methods
- Test CQE handling

#### 10.2 Integration Tests
- Test with real files using `tempfile`
- Test with sockets
- Test edge cases (queue full, wraparound)

#### 10.3 Example Tests
- NOP operation test
- Read/write file test
- Vector I/O test

## Code Style Guidelines

### Formatting
- Max width: 80 characters
- Comment width: 80 characters with wrapping
- Use `use_small_heuristics = "Max"`
- Run `cargo fmt` before committing

### Import Order
1. `core::*` (standard library core)
2. External crates (`rustix::*`)
3. Internal modules (`use crate::*`, `mod mmap;`)

### Naming Conventions
- Types/Structs/Enums/Traits: `PascalCase`
- Functions/Methods: `snake_case`
- Variables: `snake_case`
- Constants: `SCREAMING_SNAKE_CASE`
- Module names: `snake_case`

### Types & Memory
- Use `u32` for io_uring ring indices/offsets
- Use `usize` for Rust slice/array indexing
- Prefer explicit types over type inference for public APIs
- Use `#[derive(Debug)]` for all public types

## Safety Requirements

### Unsafe Blocks
- ALWAYS document with `// SAFETY:` comment
- Explain invariants required for safety

### Unsafe Functions
- Add `# Safety` section in doc comments
- List caller requirements

### Documentation
- Document shared memory access patterns
- Document atomic ordering requirements
- Trust kernel-provided offsets but validate before use

## Build and Test Commands

### Building
```bash
cargo build              # Debug build
cargo build --release    # Release build
```

### Testing
```bash
cargo test               # Run all tests
cargo test <test_name>   # Run a single test
cargo test -- --nocapture  # Show stdout during tests
```

### Linting & Formatting
```bash
cargo clippy             # Run linter
cargo clippy -- -D warnings  # Fail on warnings
cargo fmt                # Format code
cargo fmt -- --check     # Check formatting without modifying
```

### Documentation
```bash
cargo doc --open         # Generate and open docs
```

## References

1. **Inspiration**: Zig's IoUring.zig (v0.15.x)
2. **Kernel docs**: `man 7 io_uring`, `man 2 io_uring_setup`
3. **Reference**: liburing (C library)
4. **Syscall interface**: rustix crate

## Implementation Order

1. Project setup and Cargo.toml configuration
2. Core constants and structures (io_uring_params, offsets)
3. RwMmap wrapper implementation
4. SQE and CQE structures
5. SubmissionQueue implementation
6. CompletionQueue implementation
7. PrepSqe trait and operation methods
8. Main IoUring structure and initialization
9. Submission and completion methods
10. Error handling types
11. Tests and examples
12. Clippy fixes and final polish

## Key Design Decisions

1. **Zero-copy approach**: Return references where lifetime allows
2. **Batch operations**: Prefer batched APIs over single-item APIs
3. **Clear ownership**: IoUring owns the ring; methods take `&mut self`
4. **Trait-based prep methods**: Use PrepSqe trait for SQE preparation
5. **Liburing compatibility**: Match liburing method names where possible

## Risk Mitigation

1. **Kernel version compatibility**: Use SINGLE_MMAP feature for 5.4+ kernels
2. **Memory safety**: Comprehensive bounds checking in RwMmap
3. **Error handling**: Descriptive error variants with errno information
4. **Atomic ordering**: Correct memory ordering per kernel requirements

## Success Criteria

1. All tests pass
2. No clippy warnings (even with pedantic settings)
3. Code formatted according to guidelines
4. Documentation with examples
5. Performance comparable to liburing
