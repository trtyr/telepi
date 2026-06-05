# Project Review Summary

**Project:** TelePi
**Date:** 2026-06-05
**Stack:** Rust / teloxide 0.13 / tokio
**Size:** ~2,500 LOC across 28 Rust source files

## Scores

| Angle | Score | Key Finding |
|-------|-------|-------------|
| Architecture | **B+** | Clean 3-layer design; `PiSession` trait is the right abstraction; `HandlerState` is a coupling magnet |
| Code Quality | **B** | Good naming and organization; copy-pasted download/auth boilerplate across 3+ handlers; 3 functions at 150+ lines |
| Error Handling | **B** | Solid `thiserror` enum; `to_friendly_error()` exists but is never called; UTF-8 slicing panic risk |
| Testing | **D+** | 14 unit tests across 3/28 files (10.7%); zero integration tests; no CI; critical modules untested |
| API Design | **B** | Excellent trait design and doc coverage; no `#[non_exhaustive]` on key types; `TelePiConfig` fields over-exposed |
| Infrastructure | **D+** | No CI/CD, no README, plaintext bot token in git history, 5 unused deps, no release process |
| Runtime Health | **C+** | Good concurrency patterns; critical subprocess timeout missing; no SIGTERM handler; no graceful shutdown |

## Overall Grade: C+

Strong architectural foundations and clean code conventions, but severely lacking in testing, infrastructure, and runtime hardening. The project is well-designed for its current single-user scope but would need significant investment in automation, testing, and resilience before scaling.

## Top 3 Strengths

1. **`PiSession` trait abstraction** — cleanly decouples the Telegram bot from the Pi CLI subprocess. Enables future protocol changes without touching the bot layer. (evidence: `src/pi/session.rs:82-117`)
2. **Concurrency patterns** — correct use of `Arc<RwLock>` with double-check locking for session registry, `Arc<Mutex>` for busy guard, proper async mutex for model lists. Zero race conditions found. (evidence: `src/pi/registry.rs:46-59`, `src/bot/state.rs:46`)
3. **Error type hierarchy** — `TelePiError` with 11 domain-specific variants and `to_friendly_error()` cleanly separates internal errors from user-facing messages. Consistent `?` propagation throughout. (evidence: `src/error.rs:5-65`)

## Top 3 Areas to Improve

1. **Testing & CI** — 14 tests across 3 files, zero integration tests, no CI pipeline. Critical modules (`handler.rs`, `state.rs`, `registry.rs`, `cli_session.rs`) have zero test coverage. Add GitHub Actions with `cargo test`, `cargo clippy`, `cargo fmt --check`. Create `tests/` directory with integration tests and fixtures.
2. **Runtime hardening** — No timeout on Pi CLI subprocess (`cli_session.rs:281`), no SIGTERM handler for service managers (`main.rs`), no graceful shutdown for prompt inbox polling loop (`prompt_inbox.rs:33`). Add `tokio::time::timeout` to `execute_streaming()`, add SIGTERM handler, pass `CancellationToken` to background tasks.
3. **Infrastructure & security** — Plaintext bot token committed in `telepi.toml` (CRITICAL), `.gitignore` only ignores `/target`, no README, no release process. Rotate token immediately, fix `.gitignore`, create README, add basic CI.

## Quick Wins (high impact, low effort)

1. **Wire up `to_friendly_error()`** in handler error paths — it exists but is never called. 15 min, prevents Rust internals leaking to Telegram users.
2. **Fix UTF-8 panic** in `handler.rs:163` (`&transcript.text[..100]`) — use `.chars().take(100).collect()`. 5 min, prevents production crash.
3. **Fix `.gitignore`** — add `telepi.toml`, `.env`, `*.local.toml`. 2 min, prevents future credential leaks.
4. **Add `--locked` to Dockerfile** — `cargo build --release --locked`. 1 min, ensures deterministic builds.
5. **Remove unused deps** — `glob`, `sha2`, `base64`, `tokio-stream`, `tracing-appender`. 5 min, reduces compile time and binary size.
6. **Add subprocess timeout** — wrap `execute_streaming()` in `tokio::time::timeout(Duration::from_secs(600))`. 30 min, prevents indefinite chat hangs.

## Systemic Issues (patterns, not one-offs)

1. **Copy-paste over abstraction** — Telegram file download logic is duplicated across `voice_handler` and `photo_handler` (identical ~30-line blocks). Auth + busy guard boilerplate is repeated in every handler. Error send-return patterns are copy-pasted across 5 command files. This is the #1 source of technical debt.
2. **Silent fallbacks** — Config parsing uses `.ok()` which swallows errors (`config.rs:185,197`), `data_dir()` falls back to `/tmp` silently (`cli_session.rs:212`), `ToolVerbosity::from_str_loose` silently defaults on garbage input. These make debugging production issues very difficult.
3. **No observability at boundaries** — Channel send failures are silently `.ok()`'d (`cli_session.rs:328,332,340`), cleanup operations swallow errors, no request correlation IDs. A single prompt flows through download → transcribe → process → respond with no way to trace it end-to-end.
