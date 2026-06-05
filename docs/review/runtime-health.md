# Runtime Health Review

> **Date**: 2026-06-05
> **Scope**: TelePi Rust binary — runtime resilience, fault tolerance, observability
> **Verdict**: Solid foundation for a single-user/personal tool. Key gaps around subprocess timeouts, HTTP client reuse, and graceful shutdown would matter at scale.

---

## Summary Table

| Criterion | Score | Verdict |
|-----------|-------|---------|
| 1. Timeout Handling | **C** | Critical subprocess timeout missing; HTTP clients have no explicit timeouts |
| 2. Retry Logic | **B** | Good for Telegram API; absent for subprocesses and voice transcription |
| 3. Logging | **B+** | Structured tracing with good field hygiene; no sensitive data in logs |
| 4. Concurrency Safety | **A-** | Correct patterns (RwLock + double-check, Mutex busy guard); no race conditions found |
| 5. Configuration Validation | **B+** | Fail-fast on required fields; some silent parse failures in optional fields |
| 6. Resource Cleanup | **C+** | Temp files cleaned per-request; no global cleanup on crash; no `Drop` impls |
| 7. Graceful Shutdown | **D+** | Relies solely on teloxide Ctrl+C; no drain logic; polling loop is unbounded |

---

## 1. Timeout Handling — **C**

### Evidence

| Location | Timeout | Assessment |
|----------|---------|------------|
| `src/bot/handler.rs:429-432` | 5s on edit task drain | ✅ Reasonable — prevents hung edit tasks |
| `src/bot/handler.rs:132` | `reqwest::Client::new()` — no timeout | ⚠️ Default reqwest timeout is **no timeout** for connect; file download from Telegram API can hang indefinitely |
| `src/bot/handler.rs:252` | Same `reqwest::Client::new()` for photo download | ⚠️ Same issue — HTTP download can block forever |
| `src/voice/mod.rs:90-113` | `reqwest::Client::new()` for OpenAI Whisper API call | ⚠️ Cloud API call with no timeout — can hang on network issues |
| `src/pi/cli_session.rs:281` | `cmd.spawn()` — no timeout on subprocess | 🔴 **Critical**: `pi` CLI subprocess has **zero timeout**. A stuck `pi` process blocks the chat permanently until `/abort` |
| `src/bot/mod.rs:30` | `bot.delete_webhook().send().await?` | ⚠️ No timeout — startup can hang on Telegram API issues |
| `src/bot/prompt_inbox.rs:30` | `interval()` ticker | ✅ Correct — tick interval is bounded by design |

### Recommendations

1. **Highest priority**: Add a timeout to `execute_streaming()` in `cli_session.rs`. A reasonable default is 5–10 minutes for AI inference. Use `tokio::time::timeout()` wrapping the entire `while let Some(line) = lines.next_line()` loop, or apply a per-read timeout.
2. Use `reqwest::Client::builder()` with explicit `.timeout(Duration::from_secs(60))` and `.connect_timeout(Duration::from_secs(10))` for all HTTP clients. Create a shared client (cloneable) rather than `Client::new()` per request.
3. Add a timeout to `bot.delete_webhook()` at startup — 10s is sufficient.

---

## 2. Retry Logic — **B**

### Evidence

| Location | Pattern | Assessment |
|----------|---------|------------|
| `src/bot/transport.rs:7-37` | `with_retry()`: 3 attempts, linear backoff (2s × attempt), network errors only | ✅ Good — distinguishes network vs API errors, capped retries |
| `src/bot/mod.rs:19-22,79-102` | 409 Conflict retry: 5 attempts, 3s fixed delay | ✅ Good — handles competing bot instances |
| `src/bot/transport.rs:66,89,101` | All Telegram send/edit/typing wrapped in `with_retry` | ✅ Consistent coverage |
| `src/pi/cli_session.rs:281-398` | `execute_streaming()` — no retry on subprocess failure | ⚠️ A crash in `pi` CLI is final — no recovery |
| `src/voice/mod.rs:108-113` | OpenAI API call — no retry | ⚠️ Cloud APIs are inherently flaky; a single failure kills the transcription |
| `src/bot/handler.rs:128-147` | File download from Telegram — no retry | ⚠️ Telegram file downloads can fail transiently |

### Recommendations

1. Add retry (2–3 attempts with backoff) to `transcribe_openai()` — cloud APIs are the most likely point of transient failure after Telegram.
2. Subprocess failures in `pi` CLI should not be retried automatically (the user should decide via `/retry`), but a stale process should be detected and cleaned up.
3. Consider wrapping Telegram file downloads in `with_retry`.

---

## 3. Logging — **B+**

### Evidence

| File | Pattern | Assessment |
|------|---------|------------|
| `src/bot/handler.rs` | `info!(chat_key, prompt_len)`, `error!(error)`, `warn!(user_id)` | ✅ Structured fields, appropriate levels |
| `src/pi/cli_session.rs` | `info!(session_id, args)`, `info!(text_len, tokens_in, tokens_out)` | ✅ Good operational visibility |
| `src/bot/prompt_inbox.rs` | `info!(file)`, `error!(error)` | ✅ Inbox lifecycle is observable |
| `src/bot/transport.rs:27` | `tracing::warn!(attempt, delay, error)` | ✅ Retry attempts are logged |
| `src/main.rs:21` | `eprintln!()` for kill messages | ⚠️ Inconsistent — uses eprintln instead of tracing |
| `src/main.rs:78-90` | `println!()` for setup/status output | ℹ️ Acceptable — CLI output, not runtime logging |
| `src/main.rs:64` | Logs proxy and log_level at startup | ✅ Config is observable at startup |
| All files | No bot token, API key, or user content in log output | ✅ No sensitive data leaks |

### Recommendations

1. Replace `eprintln!` in `kill_existing_processes()` with `tracing::warn!` for consistency.
2. Consider adding a request ID or correlation ID to trace a single prompt through the entire pipeline (download → transcribe → prompt → response).
3. The `chat_key` logged is a numeric ID, not a username — good practice.

---

## 4. Concurrency Safety — **A-**

### Evidence

| Pattern | Location | Assessment |
|---------|----------|------------|
| `Arc<Mutex<BotChatStateInner>>` | `src/bot/state.rs:46` | ✅ Correct — async Mutex for per-chat busy guard |
| `Arc<RwLock<SessionRegistryInner>>` | `src/pi/registry.rs:16` | ✅ Correct — read-heavy RwLock for session map |
| Double-check locking | `src/pi/registry.rs:46-51,54-59` | ✅ Classic pattern — read lock, then upgrade with write lock + re-check |
| `Arc<Mutex<Option<Child>>>` | `src/pi/cli_session.rs:28` | ✅ Correct — single writer for running process handle |
| `Arc<tokio::sync::Mutex<HashMap>>` for model_lists | `src/bot/handler.rs:22` | ✅ Tokio Mutex (not std) — correct for async context |
| `BotChatState` is `Clone` via `Arc` | `src/bot/state.rs:44-47` | ✅ Shared state is properly arc-wrapped |
| `HandlerState` is `Clone` | `src/bot/handler.rs:16-23` | ✅ All fields are `Arc`-wrapped |
| `unsafe { std::env::set_var }` | `src/main.rs:47-50` | ⚠️ Guarded by comment "single-threaded before tokio runtime starts" — technically correct but fragile |

### Recommendations

1. The `unsafe { std::env::set_var }` at `main.rs:47` is safe in practice (runs before `rt.block_on`), but consider using `reqwest::Proxy` via `Client::builder()` instead of setting env vars, which is cleaner and avoids the `unsafe` block entirely.
2. No deadlocks detected — lock ordering is consistent (always `BotChatState` → `SessionRegistry` → `CliSession.running_child`).

---

## 5. Configuration Validation — **B+**

### Evidence

| Location | Behavior | Assessment |
|----------|----------|------------|
| `src/config.rs:144-146` | `bot_token` — fails with `InvalidConfig("missing required field")` | ✅ Fail-fast with clear message |
| `src/config.rs:155-158` | `allowed_user_ids` — fails if empty | ✅ Fail-fast — prevents running without auth |
| `src/config.rs:161-173` | `workspace` — cascading fallback to cwd | ✅ Sensible default |
| `src/config.rs:185` | `prompt_inbox_interval_ms` — `parse::<u64>().ok()` swallows errors | ⚠️ Invalid value (e.g., "abc") silently falls back to 60_000 — should warn |
| `src/config.rs:197` | `sherpa_onnx_num_threads` — same `.ok()` pattern | ⚠️ Same silent fallback |
| `src/config.rs:33` | `ToolVerbosity::from_str_loose("garbage") → Summary` | ℹ️ Lenient — acceptable for non-critical field |
| `src/config.rs:238-263` | `load_toml_config()` — clear error for read/parse failures | ✅ Error includes file path |
| `src/config.rs:268-273` | `TELEPI_CONFIG` env var — silently ignored if file doesn't exist | ⚠️ Should log a warning if explicitly set path is missing |

### Recommendations

1. Log a `warn!` when `TELEPI_CONFIG` env var points to a non-existent file — a user who sets this explicitly expects it to be found.
2. Log a `warn!` when numeric config fields fail to parse rather than silently using defaults.
3. Consider validating that `workspace` is actually a directory that exists.

---

## 6. Resource Cleanup — **C+**

### Evidence

| Location | Pattern | Assessment |
|----------|---------|------------|
| `src/bot/handler.rs:197` | `tokio::fs::remove_file(&ogg_path).await.ok()` after voice processing | ✅ Cleanup on success and error paths (error early-returns also clean up) |
| `src/bot/handler.rs:297` | `tokio::fs::remove_file(&img_path).await.ok()` after photo processing | ✅ Cleanup present |
| `src/pi/cli_session.rs:413-416` | `running_child` set to `None` after child exits | ✅ Process handle released |
| `src/pi/registry.rs:79-81` | `session.dispose().await.ok()` on removal | ✅ Dispose called, errors suppressed (acceptable) |
| `src/pi/cli_session.rs:543-546` | `dispose()` is a no-op (just logs) | ⚠️ Doesn't kill the subprocess or clean session directory |
| No `Drop` impl | Entire codebase | ⚠️ If a `CliSession` is dropped without `dispose()`, the running child process leaks |
| `src/bot/handler.rs:125-126` | `tokio::fs::create_dir_all(&temp_dir).await.ok()` | ℹ️ Temp dir `/tmp/telepi` created but never cleaned globally |
| Crash/interrupt | Temp files in `/tmp/telepi` accumulate | ⚠️ No startup cleanup of stale temp files |

### Recommendations

1. Implement `Drop` for `CliSession` that kills any running child process (at minimum sends SIGTERM). Currently, if the bot crashes mid-prompt, orphan `pi` processes survive. The `kill_existing_processes()` in `main.rs` is a blunt workaround.
2. Add a startup sweep to clean `/tmp/telepi` (or use `tempfile` crate for auto-cleanup).
3. Make `dispose()` actually terminate the running subprocess and clean up the session directory.

---

## 7. Graceful Shutdown — **D+**

### Evidence

| Location | Behavior | Assessment |
|----------|----------|------------|
| `src/bot/mod.rs:85` | `.enable_ctrlc_handler()` on teloxide Dispatcher | ✅ Ctrl+C stops the dispatcher |
| No `tokio::signal` handler | Entire codebase — no SIGTERM handling | 🔴 Systemd/launchd sends SIGTERM, not Ctrl+C — service stop may not shut down cleanly |
| `src/bot/prompt_inbox.rs:33-38` | `loop { ticker.tick().await }` — infinite polling loop | ⚠️ No cancellation token or shutdown signal — this task outlives the dispatcher |
| No request draining | Entire codebase | ⚠️ In-flight prompts are abandoned on shutdown — no "waiting for N requests to finish" |
| `src/bot/mod.rs:73-76` | Inbox handle stored as `_inbox_handle` but never aborted | ⚠️ The handle is leaked (intentional — keeps the task alive) but cannot be cancelled |
| `src/pi/cli_session.rs:527` | `libc::SIGTERM` for subprocess abort | ✅ Per-request abort works — but there's no global "kill all sessions on shutdown" |

### Recommendations

1. **Add `tokio::signal::ctrl_c()` + SIGTERM handler** in `main.rs`. On signal, set a shutdown flag that the prompt inbox loop and dispatcher respect.
2. **Pass a `CancellationToken`** (from `tokio_util`) to the prompt inbox task. Cancel it during shutdown.
3. **Drain active sessions on shutdown**: iterate `SessionRegistry`, call `abort()` on all running sessions, then `dispose()` on all.
4. The current `enable_ctrlc_handler()` only works for interactive terminal use — it does **not** handle SIGTERM from service managers. Add:
   ```rust
   tokio::select! {
       _ = tokio::signal::ctrl_c() => {},
       _ = async {
           #[cfg(unix)]
           tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
               .unwrap().recv().await;
       } => {},
   }
   ```

---

## Additional Observations

### HTTP Client Churn

Three separate `reqwest::Client::new()` calls across the codebase (`handler.rs:132`, `handler.rs:252`, `voice/mod.rs:90`). Each creates a new connection pool. For a long-running bot, these should share a single `Client` instance (it's `Clone`-cheap via `Arc`).

### `unsafe` in `main.rs:47-50`

`std::env::set_var` is marked `unsafe` in Rust 2024 edition. The current usage is safe (single-threaded, before tokio runtime), but `reqwest::Client::builder().proxy(...)` would be cleaner and avoid the `unsafe` block.

### `kill_existing_processes()` fragility

Using `pgrep -f "telepi"` matches any process with "telepi" in its command line, including editors with the file open. The `pkill -f "pi --mode json"` could kill unrelated processes. Consider using PID files instead.

### No Health Check Endpoint

For Docker/service deployments, there's no HTTP health check endpoint. The Dockerfile would need an external probe (e.g., checking if the process is alive). Consider adding a simple health status that service managers can query.

---

## Priority Fixes

| Priority | Issue | Impact |
|----------|-------|--------|
| 🔴 P0 | Add subprocess timeout in `execute_streaming()` | Hung `pi` process permanently blocks a chat |
| 🔴 P0 | Add SIGTERM handler for service manager compatibility | Service stop may not shut down cleanly |
| ⚠️ P1 | Add timeout to all `reqwest::Client` calls | Network issues can block the bot indefinitely |
| ⚠️ P1 | Implement `Drop` or cleanup for `CliSession` | Orphan processes survive crashes |
| ⚠️ P2 | Pass `CancellationToken` to prompt inbox loop | Polling task can't be stopped gracefully |
| ℹ️ P2 | Share a single `reqwest::Client` across handlers | Connection pool efficiency |
| ℹ️ P3 | Add retry to OpenAI Whisper API calls | Cloud API reliability |
| ℹ️ P3 | Log warnings for silent config parse fallbacks | Debuggability |
