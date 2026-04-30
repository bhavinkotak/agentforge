# Contributing to AgentForge

Thank you for your interest in contributing! AgentForge is actively developed and welcomes well-scoped contributions.

---

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Before You Start](#before-you-start)
- [Dev Environment Setup](#dev-environment-setup)
- [Project Structure](#project-structure)
- [Making Changes](#making-changes)
- [Commit Messages](#commit-messages)
- [Pull Request Process](#pull-request-process)
- [Review Checklist](#review-checklist)

---

## Code of Conduct

Be respectful. Harassment, dismissive comments, or personal attacks will not be tolerated.

---

## Before You Start

- **Bug fixes and small improvements**: open a PR directly.
- **New features or significant refactors**: open an issue first to discuss the approach. This avoids duplicate work and ensures the change aligns with the project direction.
- **Security vulnerabilities**: do **not** open a public issue. See [SECURITY.md](SECURITY.md) (or email the maintainer directly).

---

## Dev Environment Setup

### Prerequisites

| Tool | Version | Install |
|------|---------|---------|
| Rust | 1.78+ | [rustup.rs](https://rustup.rs) |
| Docker + Compose | any recent | [docker.com](https://docker.com) |
| sqlx-cli | latest | `cargo install sqlx-cli --no-default-features --features postgres` |
| cargo-audit | latest | `cargo install cargo-audit` |
| cargo-deny | latest | `cargo install cargo-deny` |

### Steps

```bash
# 1. Fork and clone
git clone https://github.com/<your-username>/agentforge.git
cd agentforge

# 2. Start PostgreSQL
docker-compose up -d postgres

# 3. Configure environment
cp .env.example .env
# Fill in at minimum: DATABASE_URL, OPENAI_API_KEY (or ANTHROPIC_API_KEY)

# 4. Run migrations
export DATABASE_URL="postgres://agentforge:agentforge@localhost:5432/agentforge"
sqlx migrate run

# 5. Build and test
source ~/.cargo/env
SQLX_OFFLINE=true cargo build --workspace
DATABASE_URL="postgres://agentforge:agentforge@localhost:5432/agentforge" cargo test --workspace
```

---

## Project Structure

This is a Cargo workspace. Each crate has a focused responsibility:

| Crate | Responsibility |
|-------|---------------|
| `agentforge-core` | Shared types, errors, traits (`AgentFile`, `EvalRun`, `Trace`, `Scenario`, …) |
| `agentforge-parser` | Agent file parsing: YAML, JSON, Markdown frontmatter, Copilot `.agent.md` |
| `agentforge-scenarios` | Scenario generation (schema-derived, adversarial, domain-seeded) |
| `agentforge-runner` | Parallel agent execution + full trace capture |
| `agentforge-scorer` | Deterministic assertions + LLM-as-judge scoring |
| `agentforge-optimizer` | Variant generation via prompt/tool mutation strategies |
| `agentforge-gatekeeper` | Three-gate promotion logic (score, regression, stability) |
| `agentforge-db` | PostgreSQL repository layer (SQLx 0.8) |
| `agentforge-api` | REST API (Axum 0.7) |
| `agentforge-cli` | CLI binary (Clap 4) |

Changes to `agentforge-core` types ripple across every downstream crate — be careful and include tests.

---

## Making Changes

### Branch naming

```
feat/<short-description>       # new capability
fix/<short-description>        # bug fix
chore/<short-description>      # maintenance, deps, CI
docs/<short-description>       # documentation only
```

### Code style

```bash
# Format (required — CI will fail otherwise)
cargo fmt --all

# Lint (required — all warnings must be resolved)
cargo clippy --all-targets --all-features -- -D warnings

# Check for security advisories
cargo audit

# Run tests
DATABASE_URL="postgres://agentforge:agentforge@localhost:5432/agentforge" \
  cargo test --workspace
```

### Working with SQLx

All database queries use `sqlx` with offline query checking. After adding or modifying a SQL query:

```bash
# Regenerate the .sqlx/ offline cache (requires a live DB)
DATABASE_URL="postgres://agentforge:agentforge@localhost:5432/agentforge" \
  cargo sqlx prepare --workspace

# Commit the updated .sqlx/ files alongside your code changes
git add .sqlx/
```

CI runs with `SQLX_OFFLINE=true`, so the `.sqlx/` cache must be kept in sync.

---

## Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <short summary>

[optional body]

[optional footer]
```

Types: `feat`, `fix`, `docs`, `chore`, `refactor`, `test`, `ci`, `perf`

Examples:

```
feat(scorer): add path-efficiency dimension to deterministic scorer
fix(runner): handle timeout on LLM calls with exponential backoff
docs: add GitHub Actions marketplace usage examples to README
chore(deps): update sqlx to 0.8.3
```

Commits with `feat:` trigger a minor version bump; `fix:` triggers a patch bump. This feeds into the automated [release-please](https://github.com/googleapis/release-please) release workflow.

---

## Pull Request Process

1. **Target `main`** — all PRs go against the `main` branch.
2. **Keep PRs focused** — one logical change per PR makes review easier and history cleaner.
3. **Fill in the PR template** — describe what changed and why; link related issues.
4. **All CI checks must pass** before requesting review:
   - `fmt` — `cargo fmt --check`
   - `clippy` — zero warnings
   - `test` — full test suite
   - `audit` — no unignored advisories
   - `action-test` — validates `action.yml` structure
5. **Request review from [@bhavinkotak](https://github.com/bhavinkotak)** — CODEOWNERS auto-assigns this; every PR requires maintainer approval before merging.
6. **Maintainer merges** — do not merge your own PR even if you have the permissions.

### Stale PRs

PRs with no activity for 30 days will be labelled `stale` and may be closed. Leave a comment if you need more time.

---

## Review Checklist

Before marking your PR ready for review, confirm:

- [ ] `cargo fmt --all` passes
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] `cargo test --workspace` passes (with a live DB or `SQLX_OFFLINE=true`)
- [ ] New behaviour is covered by tests
- [ ] `.sqlx/` cache updated if SQL queries were added or changed (`cargo sqlx prepare --workspace`)
- [ ] `README.md` or docs updated if user-facing behaviour changed
- [ ] Commit messages follow Conventional Commits

---

## License

By contributing, you agree that your contributions will be licensed under the [MIT License](LICENSE).
