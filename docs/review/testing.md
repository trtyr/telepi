# Testing Quality Review

> Last updated: 2026-06-05

## Executive Summary

TelePi has **minimal test coverage** — 14 unit tests across 3 of 28 source files (10.7%). Critical paths (bot handlers, session management, concurrency) are completely untested. No CI integration exists.

| Criterion | Grade | Summary |
|-----------|-------|---------|
| Coverage | D | 3/28 files tested; critical modules have zero tests |
| Test Quality | C+ | Tested files have meaningful tests; untested modules have none |
| Test Organization | D | No integration tests, no test directory, no fixtures |
| Edge Case Testing | C | Config tests cover edges; format/tree tests are thin |
| Test Maintainability | B- | Tests are readable but lack shared fixtures |
| CI Integration | F | No CI pipeline; tests run manually only |

---

## 1. Coverage

**Grade: D**

| Metric | Value |
|--------|-------|
| Source files | 28 |
| Files with tests | 3 (10.7%) |
| Total test functions | 14 |
| Integration/E2E tests | 0 |

**Tested files:** `src/config.rs` (7), `src/format.rs` (5), `src/pi/tree.rs` (2)

**Critical untested modules (HIGH risk):**
- `src/bot/handler.rs` — Core message handling (text, voice, photo, abort, retry)
- `src/bot/state.rs` — Busy guard (concurrency control; race conditions possible)
- `src/bot/transport.rs` — Telegram message sending, long-message splitting
- `src/pi/cli_session.rs` — Pi CLI subprocess management, JSON streaming protocol
- `src/pi/registry.rs` — Session registry with RwLock concurrent access

**Recommendation:** Prioritize testing handler.rs, state.rs, registry.rs. Use teloxide test utilities for mock Telegram contexts.

---

## 2. Test Quality

**Grade: C+**

**Config tests (7) — Good.** Cover happy path, empty input, invalid input, case insensitivity, and fallback behavior:
```rust
// src/config.rs:341-347
fn test_tool_verbosity_from_str() {
    assert_eq!(ToolVerbosity::from_str_loose("all"), ToolVerbosity::All);
    assert_eq!(ToolVerbosity::from_str_loose("SUMMARY"), ToolVerbosity::Summary);
    assert_eq!(ToolVerbosity::from_str_loose("garbage"), ToolVerbosity::Summary);
}
```

**Format tests (5) — Adequate.** Verify core markdown-to-HTML conversion but lack edge cases (empty string, malformed markdown, nested formatting).

**Tree tests (2) — Minimal.** Only test trivial utilities (`truncate`, `encode_workspace_path`). Complex functions (`find_session_dirs`, `format_conversation_tree`) are untested.

**Recommendation:** Add edge case tests; consider property-based testing (proptest) for parsers.

---

## 3. Test Organization

**Grade: D**

| Aspect | Status |
|--------|--------|
| Unit tests | Partial (3/28 files) |
| Integration tests | **Missing** — no `tests/` directory |
| E2E tests | **Missing** — no bot-level test harness |
| Test utilities | **Missing** — no shared fixtures or mocks |

**Expected structure:**
```
tests/
├── config_integration.rs    ← TOML file loading, env var overrides
├── bot_handler.rs           ← handler with mock Telegram context
├── session_registry.rs      ← concurrent access patterns
└── fixtures/
    ├── sample.toml
    └── sample_session.jsonl
```

**Recommendation:** Create `tests/` directory; add mock Telegram context and test fixtures.

---

## 4. Edge Case Testing

**Grade: C**

| File | Covered | Missing |
|------|---------|---------|
| config.rs | Empty/invalid input, case insensitivity | Whitespace-only, unicode, malformed TOML |
| format.rs | Basic escaping | Empty string, unclosed code blocks, nested markdown |
| tree.rs | Basic truncation | Empty string, zero length, missing directories |

**Recommendation:** Add boundary condition tests; test empty, zero-length, and malformed inputs.

---

## 5. Test Maintainability

**Grade: B-**

**Strengths:** Descriptive names, Rust convention (`#[cfg(test)]`), short focused tests, no external dependencies.

**Weaknesses:** No test helpers, inline test data not shared, no mocks for async handlers.

**Recommendation:** Extract shared fixtures; add helper functions for common setup.

---

## 6. CI Integration

**Grade: F**

No CI configuration exists (no `.github/workflows/`, no Makefile).

**Expected pipeline:**
```yaml
# .github/workflows/test.yml
name: Test
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo test
      - run: cargo clippy -- -D warnings
      - run: cargo fmt --check
```

**Recommendation:** Add CI pipeline; consider coverage reporting (cargo-tarpaulin).

---

## Priority Actions

1. **Add CI pipeline** — `.github/workflows/test.yml`
2. **Test bot/handler.rs** — Mock Telegram context for message handling
3. **Test bot/state.rs** — Concurrent access patterns for busy guard
4. **Test pi/registry.rs** — RwLock session registry under concurrent load
5. **Create tests/ directory** — Integration tests with fixtures
6. **Add edge case tests** — Empty inputs, boundary conditions, malformed data

**Overall Grade: D+**
