# Process Standard Library (C-1.11)

The `process` standard library provides subprocess management capabilities, allowing Lime programs to spawn, run, and manage external processes.

## Package

- **Name**: `process`
- **Version**: `v0.1.0`
- **Location**: `packages/process/v0.1.0/`

## Functions

### `process_spawn(command, args)` -> `Int`

Spawns a subprocess and returns its PID. Returns `-1` on failure.

```lime
let pid = process_spawn("echo", ["hello", "world"])
```

### `process_run(command, args)` -> `String`

Runs a subprocess and returns its stdout as a string.

```lime
let output = process_run("echo", ["hello"])
```

### `process_output(command, args)` -> `String`

Alias for `process_run`. Returns the stdout of the subprocess.

```lime
let result = process_output("echo", ["test"])
```

### `process_wait(pid)` -> `Int`

Waits for a subprocess to finish and returns its exit code. Returns `0` in the interpreter (since Child handles are not stored).

```lime
let exit_code = process_wait(pid)
```

### `process_kill(pid)` -> `Bool`

Sends a termination signal to a subprocess. Returns `false` in the interpreter (since Child handles are not stored).

```lime
let killed = process_kill(pid)
```

### `process_status(pid)` -> `String`

Returns the status of a subprocess as a string. Returns `"unknown"` in the interpreter.

```lime
let status = process_status(pid)
```

### `process_args()` -> `List(String)`

Returns the command-line arguments passed to the Lime process (excluding the program name).

```lime
let args = process_args()
```

## Implementation Notes

- **Interpreter**: Uses Rust's `std::process::Command` for `spawn`, `run`, and `output`. `wait`, `kill`, and `status` return placeholder values since the interpreter does not store `Child` handles.
- **Native codegen**: Uses platform-specific C runtime functions:
  - **Windows**: `CreateProcess`, `GetExitCodeProcess`, `TerminateProcess`, `_popen`
  - **POSIX**: `fork`, `exec`, `waitpid`, `kill`, `pipe`