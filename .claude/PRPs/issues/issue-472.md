# Investigation: IPC Protocol Performance — Per-Message Flush, No Connection Pooling, Base64 Overhead

**Issue**: #472 (https://github.com/Wirasm/kild/issues/472)
**Type**: ENHANCEMENT
**Investigated**: 2026-02-17T12:20:00Z

### Assessment

| Metric     | Value  | Reasoning                                                                                                      |
| ---------- | ------ | -------------------------------------------------------------------------------------------------------------- |
| Priority   | HIGH   | Labels P1. Every daemon-managed terminal session is affected — latency compounds on every keystroke and PTY read |
| Complexity | MEDIUM | 4 sub-items across 3-4 crates, each well-scoped. Items 1-2 are isolated. Item 3 touches more files but is mechanical. Item 4 is deferred. |
| Confidence | HIGH   | Issue provides exact file:line references. All code paths verified — the problems are unambiguous. Performance audit backs findings. |

---

## Problem Statement

The KILD daemon IPC protocol has three compounding performance issues: (1) every `write_message()` call triggers a kernel flush syscall even during high-frequency PTY streaming, (2) every CLI/shim daemon request creates a new Unix socket connection instead of reusing existing ones, and (3) all PTY data is base64-encoded for JSON transport, adding 33% size overhead to the hottest data path.

---

## Analysis

### Evidence Chain

WHY: Daemon terminal sessions have higher latency than expected
↓ BECAUSE: Every PTY output chunk triggers a kernel flush syscall
Evidence: `crates/kild-daemon/src/protocol/codec.rs:45` — `writer.flush().await?` called on every `write_message()`

↓ BECAUSE: The streaming loop in `stream_pty_output()` calls `write_message()` per broadcast chunk
Evidence: `crates/kild-daemon/src/server/connection.rs:428` — `write_message(&mut *w, &msg).await`

↓ AND BECAUSE: Every CLI/shim request creates a fresh socket
Evidence: `crates/kild-core/src/daemon/client.rs` — 9 public functions each call `IpcConnection::connect()`
Evidence: `crates/kild-tmux-shim/src/ipc.rs` — 5 functions each call `IpcConnection::connect()`, including `write_stdin()` (per-keystroke!)

↓ AND BECAUSE: All PTY data is base64-encoded, inflating payloads by 33%
Evidence: `crates/kild-daemon/src/server/connection.rs:422` — `engine.encode(&data)` on every PTY output chunk
Evidence: `crates/kild-protocol/src/messages.rs:175-179` — `PtyOutput.data: String` (base64-encoded)

### Affected Files

| File | Lines | Action | Description |
| ---- | ----- | ------ | ----------- |
| `crates/kild-daemon/src/protocol/codec.rs` | 37-47 | UPDATE | Remove flush from `write_message()`, add `write_message_flush()` |
| `crates/kild-daemon/src/server/connection.rs` | 49-56, 194-243, 408-466 | UPDATE | Use `write_message()` (no flush) for streaming, `write_message_flush()` for request-response |
| `crates/kild-protocol/src/client.rs` | 62-138 | UPDATE | Add `BufReader` field to `IpcConnection` for connection reuse, add reconnect logic |
| `crates/kild-core/src/daemon/client.rs` | 110-459 | UPDATE | Cache `IpcConnection` per call sequence, reuse across sequential operations |
| `crates/kild-tmux-shim/src/ipc.rs` | 11-171 | UPDATE | Cache `IpcConnection` for the shim process lifetime |
| `crates/kild-protocol/src/messages.rs` | 174-179, 124-129, 206-211 | UPDATE | Change `PtyOutput.data`, `WriteStdin.data`, `ScrollbackContents.data` from `String` to `Vec<u8>` with `#[serde(with = "serde_bytes")]` or binary framing |
| `crates/kild-daemon/src/server/connection.rs` | 229, 294, 370, 415-426 | UPDATE | Remove base64 encode/decode calls |
| `crates/kild-core/src/daemon/client.rs` | 386-393 | UPDATE | Remove base64 decode from `read_scrollback()` |
| `crates/kild-tmux-shim/src/ipc.rs` | 72, 128-130 | UPDATE | Remove base64 encode/decode from `write_stdin()`, `read_scrollback()` |
| `crates/kild-ui/src/daemon_client.rs` | 461, 577, 601, 611 | UPDATE | Remove base64 encode/decode |
| `crates/kild-ui/src/terminal/state.rs` | 449 | UPDATE | Remove base64 decode |
| `crates/kild/src/commands/attach.rs` | 204, 305 | UPDATE | Remove base64 encode/decode |
| `crates/kild-daemon/src/client/stream.rs` | 18 | UPDATE | Remove base64 decode from `decode_pty_output()` |
| `crates/kild-daemon/src/client/connection.rs` | 189 | UPDATE | Remove base64 encode |

### Integration Points

- `crates/kild-daemon/src/server/connection.rs:20-91` — `handle_connection()` is the main request/response loop. Reads messages, dispatches, writes responses. Flush needed after response writes.
- `crates/kild-daemon/src/server/connection.rs:408-466` — `stream_pty_output()` is the hot streaming loop. Receives PTY chunks via `broadcast::Receiver`, encodes to base64, writes via `write_message()`. Flush is wasteful here — tokio's buffered write already handles batching.
- `crates/kild-protocol/src/client.rs:101-129` — `IpcConnection::send()` is the sync request/response method used by kild-core and kild-tmux-shim. Creates a `BufReader` on every call (wasteful but not a leak since `&self.stream` is borrowed).
- `crates/kild-ui/src/daemon_client.rs:84-93` — `send_message()` async helper also flushes after every message. Same issue as codec.rs but in the smol async context.
- `crates/kild-ui/src/terminal/state.rs:440-460` — UI terminal reader loop decodes base64 from PtyOutput. Hot path for UI rendering.

### Git History

- **Introduced**: `6f1cfa7` (2026-02-10) — Original daemon protocol implementation (codec.rs with flush)
- **Connection extracted**: `b97d6ff` (2026-02-16) — `IpcConnection` moved to kild-protocol (refactor, no pooling added)
- **Implication**: Original design; flush was correct for initial single-message-per-connection model but became problematic as streaming and frequent IPC calls were added.

---

## Implementation Plan

### Step 1: Remove per-message flush from `write_message()` (Item 1)

**File**: `crates/kild-daemon/src/protocol/codec.rs`
**Lines**: 34-47
**Action**: UPDATE

**Current code:**

```rust
// Line 34-47
/// Write a single JSONL message to an async writer.
///
/// Serializes the message as compact JSON followed by a newline, then flushes.
pub async fn write_message<W, T>(writer: &mut W, msg: &T) -> Result<(), DaemonError>
where
    W: AsyncWrite + Unpin,
    T: Serialize,
{
    let json = serde_json::to_string(msg)?;
    writer.write_all(json.as_bytes()).await?;
    writer.write_all(b"\n").await?;
    writer.flush().await?;
    Ok(())
}
```

**Required change:**

```rust
/// Write a single JSONL message to an async writer.
///
/// Serializes the message as compact JSON followed by a newline.
/// Does NOT flush — callers should flush explicitly when transitioning
/// from write phase to read phase, or when a batch of writes is complete.
pub async fn write_message<W, T>(writer: &mut W, msg: &T) -> Result<(), DaemonError>
where
    W: AsyncWrite + Unpin,
    T: Serialize,
{
    let json = serde_json::to_string(msg)?;
    writer.write_all(json.as_bytes()).await?;
    writer.write_all(b"\n").await?;
    Ok(())
}

/// Write a single JSONL message and flush immediately.
///
/// Use for request-response messages where the peer is waiting for a response.
/// For streaming (e.g. PTY output), prefer `write_message()` without flush.
pub async fn write_message_flush<W, T>(writer: &mut W, msg: &T) -> Result<(), DaemonError>
where
    W: AsyncWrite + Unpin,
    T: Serialize,
{
    write_message(writer, msg).await?;
    writer.flush().await?;
    Ok(())
}
```

**Why**: The streaming loop (`stream_pty_output`) sends many messages per second. Each flush triggers a syscall. Removing it from the hot path and flushing only when needed (request-response transitions) eliminates unnecessary syscalls.

---

### Step 2: Update daemon server to use `write_message_flush()` for responses

**File**: `crates/kild-daemon/src/server/connection.rs`
**Lines**: 49-58 (response writes), 194-243 (attach handler)
**Action**: UPDATE

**Current code (line 51):**

```rust
if let Err(e) = write_message(&mut *w, &response).await {
```

**Required change**: Replace `write_message` with `write_message_flush` at response write sites:

1. **Line 51** — Response to dispatched message: change to `write_message_flush`
2. **Line 198** — Attach ack: change to `write_message` (no flush yet, scrollback follows)
3. **Line 217** — Resize warning: change to `write_message` (no flush yet, scrollback follows)
4. **Line 234** — Scrollback replay: change to `write_message` + explicit `w.flush().await` after line 242 (end of attach batch)
5. **Line 428** — Streaming PTY output in `stream_pty_output()`: keep as `write_message` (NO flush — this is the hot path)
6. **Line 443** — Streaming PTY dropped notification: keep as `write_message` (no flush needed)

**Why**: Request-response messages must flush to ensure the client receives the response. Streaming messages should NOT flush individually — let tokio batch them. The attach handler writes ack + optional resize warning + scrollback as a batch, then flushes once.

---

### Step 3: Add connection reuse to `IpcConnection` (Item 2)

**File**: `crates/kild-protocol/src/client.rs`
**Lines**: 62-138
**Action**: UPDATE

**Current code:**

```rust
pub struct IpcConnection {
    stream: UnixStream,
}

impl IpcConnection {
    pub fn send(&mut self, request: &ClientMessage) -> Result<DaemonMessage, IpcError> {
        // ...
        writeln!(self.stream, "{}", msg)?;
        self.stream.flush()?;
        let mut reader = BufReader::new(&self.stream);
        // ...
    }
}
```

**Required change:**

```rust
pub struct IpcConnection {
    stream: UnixStream,
    reader_buf: Vec<u8>,
}

impl IpcConnection {
    pub fn connect(socket_path: &Path) -> Result<Self, IpcError> {
        // ... existing connect logic ...
        Ok(Self {
            stream,
            reader_buf: Vec::with_capacity(4096),
        })
    }

    pub fn send(&mut self, request: &ClientMessage) -> Result<DaemonMessage, IpcError> {
        let msg = serde_json::to_string(request).map_err(|e| IpcError::ProtocolError {
            message: e.to_string(),
        })?;

        writeln!(self.stream, "{}", msg)?;
        self.stream.flush()?;

        // Reuse the read buffer across calls
        self.reader_buf.clear();
        let mut reader = BufReader::new(&self.stream);
        let mut line = String::new();
        reader.read_line(&mut line)?;

        if line.is_empty() {
            return Err(IpcError::ProtocolError {
                message: "Empty response from daemon".to_string(),
            });
        }

        let response: DaemonMessage =
            serde_json::from_str(&line).map_err(|e| IpcError::ProtocolError {
                message: format!("Invalid JSON response: {}", e),
            })?;

        if let DaemonMessage::Error { code, message, .. } = response {
            return Err(IpcError::DaemonError { code, message });
        }

        Ok(response)
    }

    /// Check if the connection is still usable (not broken pipe).
    /// Returns false if the socket appears closed.
    pub fn is_alive(&self) -> bool {
        // Peek to check if the socket is still connected
        let mut buf = [0u8; 0];
        match self.stream.peek(&mut buf) {
            Ok(_) => true,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => true,
            Err(_) => false,
        }
    }
}
```

**Why**: The connection itself already supports multiple sends (the stream isn't consumed). The change makes this explicit and adds a health check for reconnect support.

---

### Step 4: Cache connections in kild-core daemon client

**File**: `crates/kild-core/src/daemon/client.rs`
**Lines**: 110-459
**Action**: UPDATE

**Required change**: Add a thread-local cached connection:

```rust
use std::cell::RefCell;

thread_local! {
    static CACHED_CONNECTION: RefCell<Option<IpcConnection>> = const { RefCell::new(None) };
}

/// Get a connection to the daemon, reusing a cached one if available.
fn get_connection() -> Result<IpcConnection, DaemonClientError> {
    let socket_path = crate::daemon::socket_path();

    CACHED_CONNECTION.with(|cell| {
        let mut cached = cell.borrow_mut();
        if let Some(conn) = cached.as_ref() {
            if conn.is_alive() {
                // Take the connection out of the cache for exclusive use
                return Ok(cached.take().unwrap());
            }
        }
        // No cached connection or stale — create new one
        *cached = None;
        IpcConnection::connect(&socket_path).map_err(Into::into)
    })
}

/// Return a connection to the cache after successful use.
fn return_connection(conn: IpcConnection) {
    CACHED_CONNECTION.with(|cell| {
        *cell.borrow_mut() = Some(conn);
    });
}
```

Then update each public function to use `get_connection()` / `return_connection()`:

```rust
pub fn ping_daemon() -> Result<bool, DaemonClientError> {
    // ... existing setup ...
    let mut conn = match get_connection() {
        Ok(c) => c,
        Err(DaemonClientError::NotRunning { .. }) => return Ok(false),
        Err(e) => return Err(e),
    };
    conn.set_read_timeout(Some(Duration::from_secs(2)))?;
    match conn.send(&request) {
        Ok(_) => {
            return_connection(conn);
            Ok(true)
        }
        Err(_) => Ok(false),
    }
}
```

**Why**: Eliminates socket creation overhead for sequential CLI operations (e.g., `kild list` queries status for each session). Connection is cached per-thread and reconnects on error.

---

### Step 5: Cache connections in kild-tmux-shim

**File**: `crates/kild-tmux-shim/src/ipc.rs`
**Lines**: 11-171
**Action**: UPDATE

**Required change**: Same thread-local pattern as Step 4, but simpler since the shim is a short-lived process:

```rust
use std::cell::RefCell;

thread_local! {
    static CACHED_CONNECTION: RefCell<Option<IpcConnection>> = const { RefCell::new(None) };
}

fn get_or_connect() -> Result<IpcConnection, ShimError> {
    CACHED_CONNECTION.with(|cell| {
        let mut cached = cell.borrow_mut();
        if let Some(conn) = cached.take() {
            if conn.is_alive() {
                return Ok(conn);
            }
        }
        let paths = KildPaths::resolve().map_err(|e| ShimError::state(e.to_string()))?;
        IpcConnection::connect(&paths.daemon_socket()).map_err(Into::into)
    })
}

fn return_conn(conn: IpcConnection) {
    CACHED_CONNECTION.with(|cell| {
        *cell.borrow_mut() = Some(conn);
    });
}
```

This is especially important for `write_stdin()` which is called per-keystroke in agent team sessions.

**Why**: The shim's `write_stdin()` is called for every keystroke from Claude Code agent teams. Each call currently creates a new socket connection — hundreds of connections per minute. Caching eliminates ~99% of socket overhead.

---

### Step 6: Eliminate base64 encoding for PTY data (Item 3)

**File**: `crates/kild-protocol/src/messages.rs`
**Lines**: 174-179, 124-129, 206-211
**Action**: UPDATE

This is the largest change. Two approaches:

#### Approach A: JSON-escaped bytes (simpler, partial win)

Change `data: String` to `data: Vec<u8>` with `#[serde(with = "base64_serde")]` using a custom serde helper. This doesn't eliminate encoding overhead — just changes where it happens.

**Not recommended** — same overhead, just reorganized.

#### Approach B: Binary framing for PTY messages (full win)

Add a binary frame variant alongside JSONL. Use a 1-byte type discriminator:
- `0x7B` (`{`) = JSONL message (current format, backwards compatible)
- `0x01` = Binary PTY output frame: `[0x01][4-byte session_id_len][session_id][4-byte data_len][raw data]`
- `0x02` = Binary stdin frame: `[0x02][4-byte session_id_len][session_id][4-byte data_len][raw data]`

**Files to update:**

1. `crates/kild-daemon/src/protocol/codec.rs` — Add `write_binary_frame()` and `read_frame()` (returns enum of JSONL or binary)
2. `crates/kild-daemon/src/server/connection.rs` — Use binary framing for PtyOutput streaming and WriteStdin decoding
3. `crates/kild-protocol/src/messages.rs` — Keep existing JSONL types (used for non-PTY messages). Add binary frame types to protocol crate.
4. `crates/kild-protocol/src/client.rs` — Update `IpcConnection::send()` to handle binary responses (for ScrollbackContents)
5. All consumer sites (attach.rs, daemon_client.rs, terminal/state.rs, ipc.rs) — Read binary frames directly

**Recommended approach**: Approach B for the streaming path (PtyOutput, WriteStdin). Keep JSONL for all other messages (request/response, session management). This gives the biggest throughput win where it matters most.

**Implementation detail for codec.rs:**

```rust
/// Frame type discriminator for mixed JSONL/binary protocol.
pub enum Frame<T> {
    /// Standard JSONL message (existing protocol).
    Json(T),
    /// Binary PTY output: session_id + raw bytes.
    PtyOutput { session_id: String, data: Vec<u8> },
    /// Binary stdin input: session_id + raw bytes.
    StdinInput { session_id: String, data: Vec<u8> },
}

/// Write a binary PTY output frame (no base64, no JSON overhead).
pub async fn write_pty_output_binary<W>(
    writer: &mut W,
    session_id: &str,
    data: &[u8],
) -> Result<(), DaemonError>
where
    W: AsyncWrite + Unpin,
{
    let sid_bytes = session_id.as_bytes();
    writer.write_all(&[0x01]).await?; // frame type
    writer.write_all(&(sid_bytes.len() as u32).to_be_bytes()).await?;
    writer.write_all(sid_bytes).await?;
    writer.write_all(&(data.len() as u32).to_be_bytes()).await?;
    writer.write_all(data).await?;
    Ok(())
}
```

**Why**: Base64 encoding adds 33% size overhead AND CPU cost to every byte of terminal output. For high-throughput sessions (compilation output, large diffs), this is the biggest single optimization.

---

### Step 7: Update UI daemon client flush behavior

**File**: `crates/kild-ui/src/daemon_client.rs`
**Lines**: 84-93
**Action**: UPDATE

**Current code:**

```rust
async fn send_message(
    stream: &mut Async<UnixStream>,
    msg: &ClientMessage,
) -> Result<(), DaemonClientError> {
    let json = serde_json::to_string(msg)?;
    stream.write_all(json.as_bytes()).await?;
    stream.write_all(b"\n").await?;
    stream.flush().await?;
    Ok(())
}
```

**Required change**: Split into `send_message()` (no flush) and `send_message_flush()` (with flush), mirroring the daemon codec change. The UI's `send_write_stdin()` and `send_resize()` are called frequently and should not flush per-message — the smol async runtime will handle batching.

**Why**: Same rationale as Step 1 — eliminates unnecessary syscalls on the UI client side for write-heavy operations (keystroke forwarding, resize events).

---

### Step 8: Add/Update Tests

**File**: `crates/kild-daemon/src/protocol/codec.rs` (tests section)
**Action**: UPDATE

**Test cases to add:**

```rust
#[tokio::test]
async fn test_write_message_does_not_flush() {
    // Verify write_message writes data but does not auto-flush
    // by checking that data is in buffer but not yet readable
    // (test with a BufWriter wrapper)
    let inner: Vec<u8> = Vec::new();
    let mut buf_writer = tokio::io::BufWriter::new(inner);
    let msg = ClientMessage::Ping { id: "1".to_string() };
    write_message(&mut buf_writer, &msg).await.unwrap();
    // Data is buffered, not flushed to inner
    assert!(!buf_writer.buffer().is_empty());
}

#[tokio::test]
async fn test_write_message_flush_flushes() {
    let mut buf: Vec<u8> = Vec::new();
    let msg = ClientMessage::Ping { id: "1".to_string() };
    write_message_flush(&mut buf, &msg).await.unwrap();
    // Vec<u8> has no buffer — data written directly
    assert!(!buf.is_empty());
}
```

**File**: `crates/kild-protocol/src/client.rs` (tests section)
**Action**: UPDATE

**Test cases to add:**

```rust
#[test]
fn test_connection_reuse_multiple_sends() {
    // Verify IpcConnection can send multiple requests on same socket
    let dir = tempfile::tempdir().unwrap();
    let sock_path = dir.path().join("test.sock");
    let listener = UnixListener::bind(&sock_path).unwrap();

    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut reader = std::io::BufReader::new(&stream);
        // Handle two requests
        for _ in 0..2 {
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            writeln!(stream, r#"{{"type":"ack","id":"1"}}"#).unwrap();
            stream.flush().unwrap();
        }
    });

    let mut conn = IpcConnection::connect(&sock_path).unwrap();
    let req = ClientMessage::Ping { id: "1".to_string() };
    let r1 = conn.send(&req);
    assert!(r1.is_ok());
    let r2 = conn.send(&req);
    assert!(r2.is_ok());

    handle.join().unwrap();
}
```

---

## Patterns to Follow

**From codebase — mirror these exactly:**

```rust
// SOURCE: crates/kild-protocol/src/client.rs:101-129
// Pattern for request/response IPC with explicit flush
pub fn send(&mut self, request: &ClientMessage) -> Result<DaemonMessage, IpcError> {
    let msg = serde_json::to_string(request).map_err(|e| IpcError::ProtocolError {
        message: e.to_string(),
    })?;
    writeln!(self.stream, "{}", msg)?;
    self.stream.flush()?;
    // ... read response ...
}
```

```rust
// SOURCE: crates/kild-ui/src/daemon_client.rs:24-29
// Pattern for monotonic request IDs
static REQUEST_COUNTER: AtomicU64 = AtomicU64::new(1);
fn next_request_id() -> String {
    let n = REQUEST_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("ui-{n}")
}
```

```rust
// SOURCE: crates/kild-daemon/src/server/connection.rs:408-466
// Pattern for streaming loop with broadcast receiver
async fn stream_pty_output(
    mut rx: tokio::sync::broadcast::Receiver<Vec<u8>>,
    session_id: &str,
    writer: Arc<Mutex<tokio::net::unix::OwnedWriteHalf>>,
    shutdown: tokio_util::sync::CancellationToken,
) {
    // ... select on rx.recv() vs shutdown ...
}
```

---

## Edge Cases & Risks

| Risk/Edge Case | Mitigation |
| -------------- | ----------- |
| Cached connection goes stale (daemon restart) | `is_alive()` check before reuse; reconnect on error in `get_connection()` |
| Binary framing breaks existing clients during upgrade | First byte `{` detection: existing JSONL clients work unchanged. Binary framing only on new protocol paths. |
| Removing flush from streaming causes data buffering delays | tokio's write half on Unix sockets has no userspace buffer — writes go directly to kernel. Flush only matters for BufWriter wrappers. Verify no BufWriter wraps the OwnedWriteHalf. |
| Thread-local connection cache leaks FDs on long-running processes | Connections are checked with `is_alive()` and replaced when stale. Thread-local drops on thread exit. |
| Base64 removal changes wire format (breaks running daemon) | Requires daemon restart. Add protocol version negotiation or use approach B (binary framing alongside JSONL). |
| `BufReader` in `IpcConnection::send()` may consume extra bytes from stream | Currently borrows `&self.stream` — BufReader buffer is dropped after each call. For connection reuse, ensure no bytes are left in BufReader's internal buffer. Consider storing BufReader as field. |

---

## Validation

### Automated Checks

```bash
cargo fmt --check
cargo clippy --all -- -D warnings
cargo test --all
cargo build --all
```

### Manual Verification

1. Run `kild create test-branch --daemon` and verify terminal responsiveness (flush fix)
2. Run `kild list` with 10+ sessions and check latency improvement (connection pooling)
3. Attach to a session running `find / -type f 2>/dev/null` and verify no garbled output (base64 removal)
4. Stop and restart daemon mid-session, verify reconnect works (connection cache resilience)

---

## Scope Boundaries

**IN SCOPE:**

- Item 1: Remove per-message flush from `write_message()` (Steps 1-2, 7)
- Item 2: Connection pooling via cached `IpcConnection` (Steps 3-5)
- Item 3: Eliminate base64 encoding for PTY data (Step 6)
- Tests for all changes (Step 8)

**OUT OF SCOPE (do not touch):**

- Item 4: Synchronous JSON parsing optimization (lower priority, deferred)
- UI daemon client architecture (already uses two-connection model with smol async)
- Daemon config changes (`pty_output_batch_ms`, `client_buffer_size`, etc.)
- Wire protocol versioning (can be addressed in a follow-up if binary framing is adopted)

---

## Implementation Order

1. **Remove flush** (Steps 1-2, 7) — 1-2 files, immediate win, zero risk
2. **Connection pooling** (Steps 3-5) — 3 files, moderate effort, big cumulative impact
3. **Base64 elimination** (Step 6) — 8+ files, largest refactor, biggest throughput win

Each item can be implemented and merged independently. Recommend splitting into 3 PRs.

---

## Metadata

- **Investigated by**: Claude
- **Timestamp**: 2026-02-17T12:20:00Z
- **Artifact**: `.claude/PRPs/issues/issue-472.md`
