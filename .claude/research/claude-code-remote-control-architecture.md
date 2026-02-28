# Claude Code Remote Control — Architecture Research

**Date:** 2026-02-27
**Package:** @anthropic-ai/claude-code v2.1.62
**Internal codename:** Tengu
**Purpose:** Understand how Claude Code's remote-control injects messages into running sessions, and how KILD can leverage this.

## 1. NPM Package Structure

- **Size:** 30MB packed, 80MB unpacked
- **Entry:** Single 11.9MB minified `cli.js` (12,439 lines of bundled ESM)
- **Zero runtime dependencies** — everything bundled
- **Optional deps:** `@img/sharp-*` for image processing

Contents:

| File | Size | Purpose |
|---|---|---|
| `cli.js` | 11.9MB | Entire application — bundled, minified ESM |
| `resvg.wasm` | 2.5MB | SVG rasterization (QR code rendering for remote-control) |
| `tree-sitter.wasm` | 205KB | Tree-sitter runtime |
| `tree-sitter-bash.wasm` | 1.4MB | Bash grammar for shell command parsing |
| `sdk-tools.d.ts` | 87KB | TypeScript types for SDK tool I/O schemas |
| `vendor/ripgrep/` | ~50MB | Bundled rg + ripgrep.node for all platforms |

## 2. High-Level Architecture

The CLI has these layers:

1. **CLI parser** — Commander.js
2. **REPL/TUI** — Ink (React for CLI)
3. **API client** — Axios + custom streaming parsers
4. **Analytics** — "Tengu" event system (`tengu_*` events) to Segment + Datadog
5. **Auth** — OAuth against `claude.ai/oauth/authorize`, keychain on macOS
6. **Feature flags** — GrowthBook SDK (`tengu_tst_*` experiments)
7. **Bridge** — WebSocket + SSE transport layer for remote-control

## 3. Remote Control — Three-Layer Architecture

```
┌─────────────────┐     HTTPS/WSS      ┌─────────────────────────────────┐
│  Remote Device   │◄──────────────────►│  Anthropic API                  │
│  (claude.ai/code │     SSE + POST     │  api.anthropic.com              │
│   or mobile app) │                    │  bridge.claudeusercontent.com   │
└─────────────────┘                    └──────────┬──────────────────────┘
                                                  │
                                         HTTP Poll + WSS
                                                  │
                                       ┌──────────▼──────────┐
                                       │  Local CLI           │
                                       │  (claude remote-     │
                                       │   control)           │
                                       │  - Full filesystem   │
                                       │  - MCP servers       │
                                       │  - Git, tools, etc   │
                                       └──────────────────────┘
```

### Bridge URLs

- **Production:** `wss://bridge.claudeusercontent.com`
- **Staging:** `wss://bridge-staging.claudeusercontent.com`
- **Local dev:** `ws://localhost:8765` (env `LOCAL_BRIDGE=1`)

### API Endpoints (reverse-engineered)

| Method | Endpoint | Purpose |
|---|---|---|
| `POST` | `/v1/environments/bridge` | Register local environment |
| `GET` | `/v1/environments/{id}/work/poll?block_ms=900&ack=true` | Long-poll for work |
| `POST` | `/v1/environments/{id}/work/{id}/ack` | Acknowledge received work |
| `POST` | `/v1/environments/{id}/work/{id}/stop` | Stop active work |
| `DELETE` | `/v1/environments/bridge/{id}` | Deregister environment |
| `POST` | `/v1/sessions` | Create a new session |
| `GET` | `/v1/sessions/{id}` | Get session details |
| `POST` | `/v1/sessions/{id}/events` | Send events to session |
| `POST` | `/v1/sessions/{id}/archive` | Archive session |
| `WSS` | `/v1/session_ingress/ws/{sessionId}` | WebSocket ingress (real-time) |
| `WSS` | `/v1/sessions/ws/{sessionId}/subscribe` | WebSocket subscribe (remote device) |

### Session Ingress URL Construction

```javascript
function ky1(baseUrl, sessionId) {
  const isLocal = baseUrl.includes("localhost") || baseUrl.includes("127.0.0.1");
  const protocol = isLocal ? "ws" : "wss";
  const version = isLocal ? "v2" : "v1";
  const host = baseUrl.replace(/^https?:\/\//, "").replace(/\/+$/, "");
  return `${protocol}://${host}/${version}/session_ingress/ws/${sessionId}`;
}
```

## 4. Two Modes of Remote Control

### Mode 1: Standalone Bridge (`claude remote-control`)

The bridge server is the **parent process**. It spawns Claude Code as a **child**.

```
Remote UI → Anthropic API → [poll] → Bridge Parent
                                          │
                                    spawn child:
                                    claude --print \
                                      --sdk-url wss://.../v1/session_ingress/ws/{sessionId} \
                                      --input-format stream-json \
                                      --output-format stream-json \
                                      --replay-user-messages
                                          │
                                    Child connects WebSocket directly
                                    (parent only manages lifecycle)
```

The child's `sy1` class (extends `cn6`) creates a `HybridTransport` WebSocket connection to the ingress URL. Messages flow bidirectionally through the WebSocket. The parent doesn't forward messages through stdin — the child has direct WebSocket access.

**Environment variables set on child:**

- `CLAUDE_CODE_ENVIRONMENT_KIND=bridge`
- `CLAUDE_CODE_SESSION_ACCESS_TOKEN=<token>`
- `CLAUDE_CODE_POST_FOR_SESSION_INGRESS_V2=1`

### Mode 2: REPL Bridge (`/remote-control` inside interactive TUI)

This is the one relevant to KILD — injecting into an **already running interactive session**.

```
Remote UI → Anthropic API → [poll] → initReplBridge() hook
                                          │
                                    WebSocket to session ingress
                                          │
                                    Tvq() message handler
                                          │
                                    type:"user" → Sf({value, mode:"prompt", uuid})
                                          │
                                    o_ prompt queue (in-memory array)
                                          │
                                    CM1() dequeue → REPL main loop processes
```

`initReplBridge()` is a React (Ink) hook called `iWz` internally. It:

1. Checks feature flag `allow_remote_sessions` and OAuth tokens
2. Calls `registerBridgeEnvironment()` with `{dir, machineName, branch, gitRepoUrl}`
3. Creates a bridge session via `createBridgeSession()`
4. Starts a poll loop (`Nvq`) calling `GET /v1/environments/{id}/work/poll`
5. On work received, decodes the JWT `work_secret` to extract `session_ingress_token`
6. Creates a `HybridTransport` WebSocket to the ingress URL
7. Sets `onData` handler → `Tvq()` → parse JSON → `onInboundMessage` callback
8. The callback calls `Sf()` to inject into the prompt queue

### The Critical Function: `Sf()` — Universal Prompt Injection

```javascript
// Sf pushes to o_ (global prompt queue) with priority "next"
function Sf(A) {
  o_.push({...A, priority: A.priority ?? "next"});
  c56(); // notify listeners
}

// CM1 dequeues by highest priority (now=0, next=1, later=2)
function CM1() {
  if (o_.length === 0) return;
  let bestIdx = 0, bestPri = EM1[o_[0].priority ?? "next"];
  for (let i = 1; i < o_.length; i++) {
    let pri = EM1[o_[i].priority ?? "next"];
    if (pri < bestPri) { bestIdx = i; bestPri = pri; }
  }
  return o_.splice(bestIdx, 1)[0];
}
```

Both the headless SDK loop and the REPL bridge call `Sf()` to inject user messages. The format:

```javascript
Sf({
  value: content,        // string | ContentBlock[]
  mode: "prompt",        // treat as user prompt
  uuid: uuid,            // for deduplication
  skipSlashCommands: true, // don't process /commands from remote
  priority: "next"       // process before queued items
})
```

## 5. Transport Layer Details

### HybridTransport (`nn6 extends ln6`)

- Extends `WebSocketTransport` (class `ln6`)
- Uses WebSocket for **receiving** data
- Falls back to HTTP POST for **sending** events (more reliable than WebSocket writes)
- POST URL: converts `/ws/` → `/session/` and appends `/events`
- 10 retry attempts with exponential backoff (500ms initial, 8s cap)
- Auto-reconnect with exponential backoff (1s initial, 30s cap)
- Ping/pong keepalive every 10s
- Message buffering for retry on reconnect
- Periodic keep_alive data frames every 5 minutes

### SSETransport (`an6`) — v2 alternative

- Server-Sent Events transport (behind `CLAUDE_CODE_USE_CCR_V2` flag)
- Sequence number deduplication
- Reconnects with `from_sequence_num` parameter
- POST for sending events (same as HybridTransport)

### Wire Protocol (JSONL over WebSocket)

**Inbound messages (remote → CLI):**

```jsonl
{"type":"user","uuid":"id","session_id":"sid","message":{"role":"user","content":"text"}}
{"type":"control_request","request_id":"rid","request":{"subtype":"interrupt"}}
{"type":"control_request","request_id":"rid","request":{"subtype":"set_model","model":"..."}}
{"type":"control_request","request_id":"rid","request":{"subtype":"set_max_thinking_tokens","max_thinking_tokens":N}}
{"type":"control_request","request_id":"rid","request":{"subtype":"initialize",...}}
{"type":"control_request","request_id":"rid","request":{"subtype":"can_use_tool","tool_name":"...","input":{...},"tool_use_id":"..."}}
```

**Outbound messages (CLI → remote):**

All assistant responses, tool uses, system events — the full `stream-json` event stream.

**Control messages:**

```jsonl
{"type":"control_response","response":{"subtype":"success","request_id":"rid","response":{...}}}
{"type":"update_environment_variables","variables":{"KEY":"value"}}
{"type":"keep_alive"}
```

### Message Deduplication

- UUID-based deduplication via a `Set` (capacity 2000)
- Messages already seen by UUID are logged and dropped
- The headless loop also checks session persistence for cross-session dedup

## 6. Headless SDK stdin Protocol

When Claude Code runs with `--print --input-format stream-json`, `cn6.read()` reads JSONL from stdin. Each line is processed by `processLine()`:

**Accepted stdin message types:**

1. `{"type":"user","message":{"role":"user","content":"text"},"session_id":"","parent_tool_use_id":null}` — user prompt
2. `{"type":"control_request","request_id":"id","request":{"subtype":"interrupt"}}` — abort current turn
3. `{"type":"control_request","request_id":"id","request":{"subtype":"initialize",...}}` — SDK initialization
4. `{"type":"control_request","request_id":"id","request":{"subtype":"set_model","model":"..."}}` — change model
5. `{"type":"control_request","request_id":"id","request":{"subtype":"set_max_thinking_tokens","max_thinking_tokens":N}}` — thinking config
6. `{"type":"control_request","request_id":"id","request":{"subtype":"can_use_tool",...}}` — permission response
7. `{"type":"control_request","request_id":"id","request":{"subtype":"mcp_status"}}` — query MCP state
8. `{"type":"control_request","request_id":"id","request":{"subtype":"rewind_files",...}}` — file rewind
9. `{"type":"control_response","response":{"subtype":"success","request_id":"id",...}}` — permission decision
10. `{"type":"update_environment_variables","variables":{"KEY":"value"}}` — update env vars
11. `{"type":"keep_alive"}` — no-op keepalive

**Constraints:**

- `--sdk-url` requires `--print`, `--input-format stream-json`, `--output-format stream-json`
- `--replay-user-messages` requires both stream-json formats
- These are all headless-only (no Ink TUI)

## 7. Connection Lifecycle

### Standalone Bridge Flow

```
CLI start → registerBridgeEnvironment() → poll loop
                                              ↓
                           work received (session assigned)
                                              ↓
                      decode work_secret JWT → extract session_ingress_token
                                              ↓
                    spawn child with --sdk-url → child connects WebSocket
                                              ↓
                    bidirectional streaming via WebSocket
                                              ↓
                    on done: SIGTERM child → stopWork() → archiveSession() → deregister
```

### REPL Bridge Flow

```
/remote-control → initReplBridge() hook
                                              ↓
                    registerBridgeEnvironment() + createBridgeSession()
                                              ↓
                    start poll loop (Nvq)
                                              ↓
                    work received → decode JWT → connect HybridTransport
                                              ↓
                    onData → Tvq() → Sf() (inject into prompt queue)
                                              ↓
                    REPL dequeues → processes as if user typed it
                                              ↓
                    on teardown: stopWork() → archiveSession() → deregister
```

### Resilience

- **Sleep detection:** If poll gap > 2x max backoff, assume system sleep, reset error budget
- **Environment re-registration:** Up to 3 re-creation attempts on `poll_work_environment_not_found`
- **Session reconnection:** 3 reconnect attempts with exponential backoff
- **Token refresh:** Auto-refresh OAuth token on 401, retry request
- **Work secret decoding:** JWT with `session_ingress_token` extracted from base64url payload

## 8. Teleport (`--remote` / `--teleport`)

Separate from remote-control. Creates sessions on Anthropic's cloud infrastructure:

- `claude --remote "task"` → `POST /v1/sessions` with cloud `environment_id`
- `claude --teleport [session-id]` → fetches message log from cloud session, replays locally
- Cloud environments listed via `rc6()` function, filtered by `kind !== "bridge"`
- Git context with `source: "git_repository"`, outcome branches `claude/{branch}`

## 9. Relevance to KILD

### Current KILD Injection Architecture

```
kild inject <branch> "text"
    │
    ├─ Claude sessions (InjectMethod::ClaudeInbox):
    │   Write to ~/.claude/teams/honryu/inboxes/<fleet_safe_name>.json
    │   ~1s polling delay, requires fleet team setup
    │   Delivery: Dropbox + ClaudeInbox
    │
    └─ Other agents (InjectMethod::Pty):
        Write text + \r to PTY stdin via daemon WriteStdin IPC
        50ms pause between text and Enter
        Fragile: relies on TUI raw mode parsing
        Delivery: Dropbox + Pty
```

### Key Insights

1. **`Sf()` is the universal injection point** — both headless SDK and REPL bridge use it. It's an in-process function call pushing to an in-memory queue.

2. **The headless SDK mode (`--print --input-format stream-json`) is clean and reliable** but loses the TUI. It accepts JSONL on stdin with user messages, control requests, and environment updates.

3. **The REPL bridge works inside the TUI** but requires Anthropic's relay infrastructure (registration, polling, WebSocket ingress). There's no local-only REPL bridge mode.

4. **PTY stdin injection (current KILD approach) works** because Ink's raw mode reads terminal input character-by-character. Text + \r is treated as paste + Enter. But it's fragile if the agent is mid-output or at a permission prompt.

5. **`--sdk-url` requires headless mode** — you cannot combine it with the interactive TUI.

### Design Options for KILD

**Option A: Enhanced PTY Injection (short-term, pragmatic)**

Keep PTY stdin injection but make it state-aware:

- Query daemon for agent idle state before injecting
- Use Claude Code status hook (`agent-status`) to detect safe injection windows
- Add retry with backoff when agent is busy
- New daemon IPC: `InjectMessage` that bundles text + state check + retry logic server-side

**Option B: Headless + kild-ui Rendering (long-term, powerful)**

Spawn Claude Code with `--sdk-url ws://localhost:{PORT}`:

- Daemon runs local WebSocket server per session
- `kild inject` writes JSONL user messages to the WebSocket
- `kild-ui` renders stream-json events natively (better than Ink TUI)
- Control messages (interrupt, model switch) become first-class daemon ops
- Terminal `kild attach` would show raw JSONL (acceptable if kild-ui is primary)

**Option C: Keep inbox for teams, enhance PTY for direct (recommended)**

- Teams/fleet: Keep inbox protocol — right semantics for async task delivery with queue, dedup, ack
- Direct injection: Enhanced PTY with state-aware retry (Option A)
- Future: Option B when kild-ui is mature enough to replace terminal attach

### PTY Injection vs SDK JSONL Comparison

| Aspect | PTY stdin (current) | SDK JSONL (headless) |
|---|---|---|
| Delivery | write bytes + \r to PTY | write JSON line to stdin pipe |
| Reliability | Fragile (TUI state dependent) | Reliable (structured protocol) |
| Deduplication | None | UUID-based |
| Rich content | Text only | ContentBlock[] (images, etc.) |
| Control messages | None | interrupt, set_model, permissions |
| Bidirectional | No (fire-and-forget) | Yes (stream-json stdout) |
| TUI visible | Yes | No (headless only) |
| Agent-agnostic | Yes (all agents) | Claude Code only |

## 10. Key Source Locations in cli.js

All references are to the minified v2.1.62 bundle. Class/function names are mangled.

| Concept | Internal name | Description |
|---|---|---|
| Base stdin reader | `cn6` | JSONL line reader, `processLine()` handles all message types |
| SDK WebSocket client | `sy1 extends cn6` | Connects to `--sdk-url`, wraps WebSocket as readable stream |
| WebSocket transport | `ln6` | Base WebSocket with auto-reconnect, ping/pong, keepalive |
| Hybrid transport | `nn6 extends ln6` | WebSocket receive + HTTP POST send |
| SSE transport | `an6` | Server-Sent Events alternative (v2) |
| Transport factory | `$Vq` | Creates `nn6`, `ln6`, or `an6` based on URL and env |
| Bridge API client | `Gy1` | REST client for `/v1/environments/` and `/v1/sessions/` |
| Bridge REPL init | `iWz` (initReplBridge) | React hook, sets up polling + WebSocket for TUI integration |
| Message handler | `Tvq` | Parses ingress WebSocket data, routes user/control messages |
| Prompt queue push | `Sf` | Universal injection: push to `o_` queue with priority |
| Prompt queue pop | `CM1` | Dequeue by priority (now=0, next=1, later=2) |
| Queue signal | `c56` | Notifies listeners when queue changes |
| Session runner | `uGq` | Spawns headless child process for standalone bridge |
| Bridge main | `LXz` | Entry point for `claude remote-control` command |
| Create session | `xqz` | `POST /v1/sessions` with environment, git context |
| Poll loop | `Nvq` | Long-poll for work with error budget, sleep detection |
| Ingress URL builder | `ky1` | Constructs `wss://.../v1/session_ingress/ws/{id}` |
