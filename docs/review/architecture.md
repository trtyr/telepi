# Architecture Review — TelePi

> Reviewed: 2026-06-05 | Scope: src/ only, excluding docs/remote/

---

## Summary

TelePi has a clean, well-structured three-layer architecture for its size (~2,500 LOC across 28 Rust files). The primary strength is the `PiSession` trait abstraction, which decouples the Telegram bot from the Pi CLI subprocess. The primary weakness is that `HandlerState` is a monolithic god-object threading through every handler and command function, and `CliSession` leaks implementation details to the bot layer.

**Overall: B+** — solid for a project of this maturity, with clear paths to improve.

---

## 1. Separation of Concerns

**Score: B+**

| Evidence | Location |
|---|---|
| Three-layer model: CLI → Bot → Session is clearly defined | `docs/context/architecture.md` L30-57 |
| `transport.rs` and `keyboard.rs` depend only on teloxide — zero internal coupling | `src/bot/transport.rs`, `src/bot/keyboard.rs` |
| `format.rs` and `paths.rs` are standalone utilities with no reverse dependencies | `src/format.rs`, `src/paths.rs` |
| `PiSession` trait in `session.rs` has zero internal dependencies (pure types + trait) | `src/pi/session.rs` L1-118 |
| Bot layer (`bot/`) and session layer (`pi/`) communicate only through `PiSession` trait and `SessionRegistry` | verified via codegraph callers/callees |

| Concern | Detail |
|---|---|
| **Leak 1: `CliSession` type exposed to bot layer** | `bot/handler.rs:22` imports `crate::pi::cli_session::ModelInfo` directly. `bot/commands/model.rs` calls `CliSession::list_models()` — a concrete impl method, not part of the `PiSession` trait. This means the bot layer knows about the CLI implementation, breaking the abstraction. |
| **Leak 2: `HandlerState` mixes concerns** | `HandlerState` (`src/bot/handler.rs:17-23`) bundles config, session registry, chat state, AND model list caching into one struct. Model list caching is presentation-layer state that doesn't belong alongside session management. |

**Recommendation:**
- Move `ModelInfo` and `list_models()` into `PiSession` trait (or a separate `ModelProvider` trait).
- Extract `model_lists` from `HandlerState` into a dedicated `ModelCache` struct.

---

## 2. Dependency Direction

**Score: A-**

| Evidence | Location |
|---|---|
| `main.rs` → `cli.rs` → `config.rs` → `bot::run()` — clean top-down entry flow | `src/main.rs` |
| Bot layer depends on session layer (not vice versa) | codegraph: `bot/handler.rs` → `pi/registry`, `pi/session` |
| Session layer has zero reverse dependencies on bot | `src/pi/session.rs` imports nothing internal; `src/pi/registry.rs` imports only `config` + `error` + `pi/*` |
| Support modules (`config`, `error`, `paths`, `format`) are leaf dependencies | no circular imports detected |
| `install/` module is fully isolated — only consumed by `cli.rs` Status command | `src/install/mod.rs` |

| Concern | Detail |
|---|---|
| **Config coupling** | `TelePiConfig` is threaded from `main.rs` through `bot::run()` → `HandlerState` → `SessionRegistry` → `CliSession::create()`. This is dependency injection via parameter passing — correct direction, but verbose. A shared `Arc<TelePiConfig>` is the right choice here given Rust's ownership model. |
| **No coupling violations found** | All dependency arrows point inward-to-outward: CLI → Bot → Session → Support. No circular deps. |

**Recommendation:**
- Dependency direction is excellent. No changes needed. The `Arc<TelePiConfig>` propagation is idiomatic Rust and avoids global state.

---

## 3. Coupling

**Score: B**

| Evidence | Location |
|---|---|
| `HandlerState` is used by 18 functions across 7 files | codegraph callers of `HandlerState` |
| `PiSession` trait change impacts 59 symbols across 12 files | codegraph impact of `PiSession` |
| `SessionRegistry` uses `Arc<dyn PiSession>` — dynamic dispatch decouples concrete impl | `src/pi/registry.rs:27` |
| `transport.rs` and `keyboard.rs` are fully decoupled from internal types | only teloxide imports |

| Concern | Detail |
|---|---|
| **`HandlerState` is a coupling magnet** | Every handler and every command function takes `HandlerState` as a parameter. Adding a field to `HandlerState` doesn't break compilation but silently widens the coupling surface. 18 dependent functions is high for a single struct. |
| **`process_prompt` is the coupling nexus** | `bot/handler.rs:317` — this single function touches `SessionRegistry`, `BotChatState`, `TelePiConfig`, `PiEvent`, `transport`, `format`, and teloxide `Bot`. It's the tightest coupling point in the codebase (~130 lines of orchestration). |
| **`CliSession` concrete type leaks** | `ModelInfo` struct and `list_models()` are accessed directly from `bot/commands/model.rs`, bypassing the trait abstraction. |

**Recommendation:**
- Split `process_prompt` into smaller functions: one for session acquisition, one for streaming orchestration, one for final formatting.
- Consider splitting `HandlerState` into `AppState` (config + sessions + chat_state) and per-handler context (model_lists, last_prompt for retry).

---

## 4. Cohesion

**Score: B+**

| Evidence | Location |
|---|---|
| `bot/state.rs` — single responsibility: per-chat busy/idle tracking | `src/bot/state.rs` (107 lines) |
| `bot/transport.rs` — single responsibility: Telegram message send/edit/split | `src/bot/transport.rs` (131 lines) |
| `pi/session.rs` — single responsibility: trait + data types definition | `src/pi/session.rs` (118 lines) |
| `pi/tree.rs` — single responsibility: JSONL parsing and tree rendering | `src/pi/tree.rs` (350 lines) |
| `config.rs` — single responsibility: config loading and validation | `src/config.rs` (388 lines) |

| Concern | Detail |
|---|---|
| **`bot/handler.rs` does too many things** | At 491 lines, it handles text messages, voice messages, photo messages, abort, retry, AND the `process_prompt` orchestration. This is the lowest-cohesion module. |
| **`pi/cli_session.rs` does too many things** | At 548 lines, it handles subprocess management, JSON event parsing, streaming protocol translation, model listing, AND the `PiSession` trait impl. The JSON parsing logic (lines ~260-450) is the bulk and could be extracted. |
| **Dead code in `commands/tree.rs`** | `cmd_branch` and `cmd_label` are stubs not wired into the `Command` enum — code exists but is unreachable. |

**Recommendation:**
- Extract JSON event parsing from `cli_session.rs` into a dedicated `json_protocol.rs` module.
- Consider splitting `handler.rs` by message type: `handler_text.rs`, `handler_voice.rs`, `handler_media.rs`, with `handler.rs` as the orchestrator.
- Remove or gate the dead `cmd_branch`/`cmd_label` stubs.

---

## 5. Extensibility

**Score: A-**

| Evidence | Location |
|---|---|
| `PiSession` trait with 8 methods — designed for alternative implementations | `src/pi/session.rs:82-117` |
| `PiEvent` enum with 9 variants — extensible event stream | `src/pi/session.rs:58-75` |
| `SessionRegistry` uses `Arc<dyn PiSession>` — runtime-polymorphic sessions | `src/pi/registry.rs:27` |
| `VoiceBackend` enum with priority-based fallback pattern | `src/voice/mod.rs` |
| `TelePiError` enum covers all module boundaries with string-wrapped variants | `src/error.rs:5-38` |
| teloxide `dptree` filter chain — adding new message types is a new branch | `src/bot/mod.rs:47-64` |

| Concern | Detail |
|---|---|
| **Adding a new `PiSession` impl requires touching `registry.rs`** | `get_or_create` hardcodes `CliSession::create()` at `src/pi/registry.rs:64`. A factory pattern or constructor injection would allow new impls without modifying the registry. |
| **`PiEvent` lacks a generic extension variant** | If the Pi CLI adds new event types, `PiEvent` needs a new variant + match arm in `cli_session.rs`. An `Unknown { kind: String, data: Value }` variant would future-proof it. |
| **Command registration is centralized** | Adding a new command requires: (1) add to `Command` enum, (2) add dispatch arm, (3) write handler function. This is standard teloxide but not automatable. |

**Recommendation:**
- Inject session factory as `Arc<dyn Fn(config, ctx) -> PiSession>` into `SessionRegistry` instead of hardcoding `CliSession`.
- Add `PiEvent::Unknown` or `PiEvent::Raw(serde_json::Value)` variant for forward compatibility.
- These are nice-to-haves — the current design handles the known extension points well.

---

## 6. Design Patterns

**Score: B+**

| Pattern | Usage | Assessment |
|---|---|---|
| **Trait abstraction** | `PiSession` trait with `CliSession` impl | ✅ Clean, well-documented, async_trait |
| **Registry pattern** | `SessionRegistry` with `HashMap + RwLock` | ✅ Double-checked locking in `get_or_create` |
| **Strategy pattern** | `VoiceBackend` priority-based fallback | ✅ Simple, effective |
| **Event streaming** | `PiEvent` + mpsc channel + debounced edits | ✅ Good for real-time Telegram updates |
| **Dependency injection via params** | `TelePiConfig` threaded as `Arc<>` | ✅ Idiomatic Rust, avoids globals |
| **dptree filter chain** | teloxide message routing | ✅ Declarative, easy to add branches |
| **Busy guard** | `BotChatState` with `Arc<Mutex<>>` | ✅ Prevents concurrent prompts per chat |
| **Builder pattern** | teloxide `Dispatcher::builder()` | ✅ Standard framework pattern |

| Concern | Detail |
|---|---|
| **No over-engineering detected** | The project uses patterns where they solve real problems. No unnecessary abstraction layers, no premature generalization. |
| **`HandlerState` is a service locator anti-pattern** | It bundles everything a handler might need. In a larger project this would be a problem; at current scale it's acceptable but should not grow further. |
| **Retry logic is split** | 409 conflict retry in `bot/mod.rs` L78-102, network retry in `transport::with_retry`. Two retry mechanisms in different places with different strategies — not unified. |

**Recommendation:**
- Consolidate retry logic into a shared `retry::with_backoff` utility.
- Freeze `HandlerState` growth — any new shared state should go into a dedicated struct.

---

## Architecture Debt Summary

| Priority | Item | Impact | Effort |
|---|---|---|---|
| Medium | Extract `ModelInfo`/`list_models` to trait | Breaks CliSession leak to bot layer | Small |
| Medium | Split `process_prompt` into sub-functions | Reduces coupling nexus | Medium |
| Medium | Extract JSON parsing from `cli_session.rs` | Improves cli_session cohesion | Medium |
| Low | Inject session factory into `SessionRegistry` | Enables new PiSession impls without registry changes | Small |
| Low | Add `PiEvent::Unknown` variant | Forward compatibility | Trivial |
| Low | Remove dead `cmd_branch`/`cmd_label` stubs | Code hygiene | Trivial |
| Low | Consolidate retry logic | Single retry strategy | Small |
| Info | 4 TODO stubs in voice, install, tree, stats | Known incomplete features | Varies |

---

## What's Done Well

1. **`PiSession` trait** — the most important architectural decision. Enables swapping the Pi CLI for a direct protocol without touching the bot layer.
2. **Layer isolation** — `transport.rs`, `keyboard.rs`, `format.rs`, `paths.rs` are clean leaf modules with no reverse coupling.
3. **`SessionRegistry`** — proper concurrent session management with `RwLock` + double-checked locking.
4. **Event streaming architecture** — `PiEvent` + mpsc + debounced Telegram edits is a solid pattern for real-time bot responses.
5. **Error hierarchy** — `TelePiError` with `to_friendly_error()` cleanly separates internal errors from user-facing messages.
6. **No over-engineering** — patterns are applied where they solve real problems, not for theoretical extensibility.

---

*This review covers architectural structure only. Code quality, test coverage, and performance are out of scope.*
