# Challenger Async Runtime v0.1.0

Challenger is a verified single-thread native async runtime for Lime.

## Highlights

- **Real Pending/Wake/Resume** — futures return Pending, are woken by events, and resumed by the executor
- **OS-backed TCP async I/O** — connect, accept, read, write via Windows IOCP / select reactor
- **Timer-based wakeups** — `sleep()` with millisecond precision via timer wheel
- **Executor** — single-thread cooperative scheduler with task interleave
- **Channels** — async channel send/receive with Pending semantics
- **Bounded Channels** — capacity-enforced channels (capacity 1..65536)
- **Mutex** — async mutex with contention-based Pending
- **RWLock** — async read-write lock with multiple-reader / single-writer semantics
- **Semaphore** — async semaphore with configurable capacity
- **Notify** — async notification primitive (notify_one / notify_all)
- **Cancellation** — task cancellation with safe timer cleanup

## Supported Async Primitives

| Primitive | Native E2E | Pending/Wake |
|-----------|:----------:|:------------:|
| Executor | Verified | Verified |
| Future | Verified | Verified |
| TCP | Verified | Verified |
| Timer/Sleep | Verified | Verified |
| Channel | Verified | Verified |
| Bounded Channel | Verified | Verified |
| Mutex | Verified | Verified |
| RWLock | Verified | Verified |
| Semaphore | Verified | Verified |
| Notify | Verified | Verified |
| Cancellation | Verified | Verified |

## Scope

**Single-thread native async runtime.**

Challenger provides a verified single-thread native async runtime for Lime. It is not a multi-thread executor and does not provide work-stealing or thread-pool scheduling.

## Usage

```lime
lime worker(ch):
    await sleep(100)
    await channel_send(ch, 42)

lime main():
    let ch = channel_new(0)
    let _w = spawn(worker(ch))
    let v = await channel_receive(ch)
    println(v)
    return
```

Build and run:

```sh
cargo build --release
target/release/lime build program.lime --emit-ll
```

## Build & Test

```sh
# Build
cargo build --release

# Run all tests
cargo test

# C runtime tests
cd src/codegen/runtime
clang.exe --target=x86_64-pc-windows-msvc -fms-extensions -fms-compatibility \
  -o challenger_test.exe challenger_test.c runtime.c -lws2_32 -lwinhttp -lshell32
./challenger_test.exe
```

## Known Limitations

1. **Single-thread only** — multi-thread executor is not supported
2. **Fixed waiter arrays** — sync primitives use 256-slot waiter arrays
3. **Channel capacity limit** — maximum 65536 messages per channel
4. **Windows-only** — native builds require MSVC/Windows SDK environment
5. **Interpreter** — async Pending/Wake/Resume semantics are not implemented in the interpreter; verification is Native-only
6. **Reactor cleanup** — cancelled tasks may leave stale reactor registrations (resource leak, not crash)

## Verification

- C runtime: 86/86 tests pass
- Cargo tests: 213/213 pass
- Async Native E2E: 36/36 tests pass
- Integrated async test: PASS (all primitives combined)
- New regressions: 0

## License

Licensed under either of

- Apache License, Version 2.0
- MIT License

at your option.
