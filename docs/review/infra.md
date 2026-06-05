# Infrastructure Review — TelePi

**Date:** 2026-06-05
**Overall Grade:** D+

A self-hosted Rust binary with no CI/CD, a security issue in `.gitignore`, several unused dependencies, and no user-facing documentation. The Docker multi-stage build is well-structured, but the absence of automation makes this project fragile for anything beyond local development.

---

## 1. Dependency Hygiene — C

| Aspect | Score | Evidence | Recommendation |
|--------|-------|----------|----------------|
| Unused deps | ⚠️ | `glob`, `sha2`, `base64`, `tokio-stream`, `tracing-appender` — zero imports in `src/` | Remove all five from `Cargo.toml`. Run `cargo +nightly udeps` or `cargo machete` to confirm. |
| Outdated deps | ⚠️ | `reqwest` locked at 0.11.27 (`Cargo.lock:1350`); current stable is 0.12.x. Blocked by `teloxide 0.13` pinning. | Monitor teloxide 0.14 release for reqwest 0.12 migration. No action until upstream moves. |
| Dual thiserror | ⚠️ | `thiserror 2.0.18` (direct, `Cargo.toml:36`) and `thiserror 1.0.69` (transitive via teloxide, `Cargo.lock:1780`). Both compiled. | Harmless but increases binary size. Will resolve when teloxide upgrades. |
| Vulnerability scan | ❌ | No `cargo audit`, `cargo deny`, or equivalent in any pipeline. | Add `cargo audit` as a pre-commit hook or CI step. |
| License audit | ✅ | `Cargo.toml:6` declares MIT. | Add `cargo deny` to verify transitive license compliance. |

---

## 2. Build Reproducibility — B-

| Aspect | Score | Evidence | Recommendation |
|--------|-------|----------|----------------|
| Lock file | ✅ | `Cargo.lock` committed (v4, `Cargo.lock:3`). Correct for a binary crate. | No change needed. |
| Docker base image | ✅ | `Dockerfile:2` pins `rust:1.85-slim`. Matches MSRV in `Cargo.toml:7`. | Consider pinning full SHA for hermetic builds (`rust:1.85-slim@sha256:...`). |
| Build flag | ⚠️ | `Dockerfile:8` uses `cargo build --release` without `--locked`. | Add `--locked` to ensure Cargo.lock is respected exactly: `cargo build --release --locked`. |
| Multi-stage build | ✅ | `Dockerfile:1-28` — builder stage compiles, runtime stage is `debian:bookworm-slim`. Minimal attack surface. | Good as-is. |
| Non-root user | ✅ | `Dockerfile:20-21` creates and switches to `telepi` user. | No change needed. |
| .cargo/config.toml | ❌ | No `.cargo/config.toml` exists. No build flags, no target overrides. | Optional: add for consistent build flags (`[build] target`, incremental settings). |

---

## 3. CI/CD — F

| Aspect | Score | Evidence | Recommendation |
|--------|-------|----------|----------------|
| Pipeline existence | ❌ | `.github/workflows/` is an empty directory. | Create at minimum: build, test, lint, audit on every push/PR. |
| Test automation | ❌ | 14 unit tests exist (`config.rs:319-387`, `format.rs:108-147`, `tree.rs:332-350`) but run nowhere automatically. | Add `cargo test` step to CI. |
| Lint / format | ❌ | No `cargo clippy` or `cargo fmt --check` in any pipeline. | Add as CI gates. Consider `rustfmt.toml` / `clippy.toml` for team consistency. |
| Caching | ❌ | No CI to cache. | When CI is added: cache `~/.cargo/registry`, `~/.cargo/git`, and `target/` keyed on `Cargo.lock` hash. |
| Release automation | ❌ | No release workflow, no GitHub Releases, no artifact publishing. | Add a release workflow triggered by version tags (`v*`). Build binaries for linux/amd64 and optionally macOS. |
| Docker image publish | ❌ | Dockerfile exists but no CI pushes to a registry. | Add GHCR or Docker Hub publishing step on release tags. |

---

## 4. Environment Config — D

| Aspect | Score | Evidence | Recommendation |
|--------|-------|----------|----------------|
| .gitignore coverage | 🔴 | `.gitignore` contains only `/target`. `telepi.toml` (with plaintext bot token) and `.env` are **not ignored**. | Add `telepi.toml`, `.env`, `*.toml` (local config) to `.gitignore` immediately. |
| Secrets in repo | 🔴 | `telepi.toml:10` contains a live Telegram bot token in plaintext. This is committed to git history. | **Rotate the token immediately.** Add `telepi.toml` to `.gitignore`. Use `git filter-branch` or BFG to purge from history. |
| Config resolution | ✅ | `TELEPI_CONFIG` env → `./telepi.toml` → `~/.pi/telepi/config.toml` (documented in `docs/context/tech-stack.md:100`). | Good layered approach. |
| .env.example | ✅ | `.env.example` exists with all variables documented and safe placeholder values. | Good. |
| Docker secrets | ✅ | `docker-compose.yml:8` mounts `.env` as `:ro` (read-only). | Good practice. |
| Docker networking | ⚠️ | `docker-compose.yml:16` uses `network_mode: host`. Exposes all host ports to the container. | Consider explicit port mapping or a dedicated bridge network if isolation matters. |

---

## 5. Documentation — D

| Aspect | Score | Evidence | Recommendation |
|--------|-------|----------|----------------|
| README.md | ❌ | No `README.md` at project root. `fffind` returned zero results. | Create a root README with: project description, quick start, configuration reference, build instructions. |
| CONTRIBUTING.md | ❌ | Does not exist. | Add contributing guidelines: setup, PR process, code style. |
| CHANGELOG.md | ❌ | Does not exist. | Start a changelog (keepachangelog.com format). Even at 0.1.0 it establishes the habit. |
| Context docs | ✅ | `docs/context/` contains 5 well-structured analysis docs (architecture, modules, tech-stack, conventions, api). | These are developer-internal. Not a substitute for user-facing README. |
| API docs | ✅ | `docs/context/api.md` documents all CLI and Telegram bot commands. | Reference from README once it exists. |

---

## 6. Release Process — F

| Aspect | Score | Evidence | Recommendation |
|--------|-------|----------|----------------|
| Versioning | ⚠️ | `Cargo.toml:3` — version `0.1.0`. No git tags found in log (`git log --oneline -10`). | Tag releases (`git tag v0.1.0`). Adopt semver. |
| Changelog | ❌ | No CHANGELOG.md. | Start tracking changes from v0.2.0 onward. |
| Rollback | ❌ | No CI artifacts, no container registry images, no release binaries. Nothing to roll back to. | CI-built artifacts and tagged images enable rollback. |
| Distribution | ❌ | Binary must be built locally. No pre-built releases, no `cargo install` support (binary crate, no lib). | Publish pre-built binaries via GitHub Releases on version tags. |

---

## Security Summary

| Issue | Severity | Location | Action |
|-------|----------|----------|--------|
| Plaintext bot token in tracked file | 🔴 Critical | `telepi.toml:10` | Rotate token. Add to `.gitignore`. Purge from git history. |
| `.gitignore` ignores only `/target` | 🔴 High | `.gitignore:1` | Add `.env`, `telepi.toml`, and other local config files. |
| No vulnerability scanning | 🟡 Medium | — | Add `cargo audit` to pre-commit or CI. |
| Docker `network_mode: host` | 🟡 Low | `docker-compose.yml:16` | Consider explicit port mapping for better isolation. |

---

## Quick Wins (ordered by impact)

1. **Rotate the Telegram bot token** — it's in git history.
2. **Fix `.gitignore`** — add `.env`, `telepi.toml`, `*.local.toml`.
3. **Remove unused deps** — `glob`, `sha2`, `base64`, `tokio-stream`, `tracing-appender`.
4. **Add `--locked` to Dockerfile** — `cargo build --release --locked`.
5. **Create basic CI** — even a minimal GitHub Actions workflow (build + test + clippy + fmt).
6. **Write a README.md** — project description, quick start, configuration.
7. **Tag v0.1.0** — establish the version baseline.
