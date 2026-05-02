> ⚠️ **DEPRECATED — v0.1 archived 2026-04-30.**  This repo was a portfolio exercise. After a 12-iteration / 27-domain competitive analysis, the recommended production path is:
>
> **kingfisher** (https://github.com/mongodb/kingfisher) — 942 rules + live secret validation · **trufflehog** (https://github.com/trufflesecurity/trufflehog) — established secret scanner · **prek** (https://prek.j178.dev) — Rust pre-commit framework
>
> The code below remains available for reference but is **no longer maintained**. See the linked alternatives for production use.

# gatecheck

> **Ultra-fast pre-commit secret + license scanner.** Single Rust binary with **~4 ms cold start** (vs Python's ~150 ms minimum), so every developer can run it on every commit without complaining. Library + CLI; embeds via `crates.io` into other Rust dev tools.

[![CI](https://github.com/M00C1FER/gatecheck/actions/workflows/ci.yml/badge.svg)](https://github.com/M00C1FER/gatecheck/actions)
![Rust](https://img.shields.io/badge/rust-1.74+-orange)
![License](https://img.shields.io/badge/license-MIT-green)

## What it does

- Scans staged diffs / files / stdin for secrets across **10+ built-in rules**: AWS keys/secrets, GitHub tokens (classic + fine-grained), OpenAI API keys, Slack tokens, JWTs, private-key blocks, generic Bearer tokens, hardcoded passwords, and license-conflict text (e.g., GPL boilerplate in an MIT project).
- Configurable `gatecheck.toml`: per-rule disable, exemption regexes, fail threshold.
- Three input modes: file/stdin (default), `--staged` (`git diff --cached`), `--diff` (working tree).
- Library API for embedding in other Rust dev tools.

## Why Rust here

The load characteristic is **cold-start latency**. A pre-commit hook fires on every `git commit`. Numbers:

| Lang | Typical cold start | Verdict |
|---|---|---|
| Python | ~150-200 ms | Devs disable hooks that take this long |
| Go | ~10-30 ms | Acceptable |
| **Rust** | **~4 ms (measured)** | Imperceptible |

Plus `crates.io` distribution lets other Rust tools embed `gatecheck::scan(diff, &cfg)` as a library — Go binaries don't compose at the library level the same way for the dev-tool ecosystem.

## Quick start

```bash
cargo install gatecheck                   # ~30s build, then it's a single 2 MB binary
gatecheck README.md                       # scan a file
echo "AKIAIOSFODNN7EXAMPLE" | gatecheck   # scan stdin
gatecheck --staged                        # scan `git diff --cached` (use as pre-commit hook)
gatecheck --list-rules                    # print built-in rule names
```

Pre-commit hook (`.git/hooks/pre-commit`):
```bash
#!/usr/bin/env bash
set -e
gatecheck --staged --threshold high
```

Or with the [pre-commit](https://pre-commit.com) framework — add to `.pre-commit-config.yaml`:

```yaml
repos:
  - repo: https://github.com/M00C1FER/gatecheck
    rev: v0.1.0
    hooks:
      - id: gatecheck
```

See also [`examples/`](examples/).

## Configuration (`gatecheck.toml`)

```toml
fail_at = "high"               # critical | high | medium | low

disable = [                    # turn off specific rules
    # "bearer-token",
]

exempt_patterns = [            # any line matching any regex is skipped
    "AKIAIOSFODNN7EXAMPLE",    # the literal AWS docs example
    "EXAMPLE_KEY",
]
```

A copy lives at [`examples/gatecheck.example.toml`](examples/gatecheck.example.toml).

## Built-in rules

| Rule | Severity | Detects |
|---|---|---|
| `aws-access-key`      | critical | `AKIA…` access-key IDs |
| `aws-secret`          | critical | secret-access-key shapes |
| `github-token`        | critical | `gh{p,o,u,s,r}_…` classic PATs |
| `github-fine-grained` | critical | `github_pat_…` fine-grained PATs |
| `openai-api-key`      | critical | `sk-proj-…` / `sk-…` OpenAI API keys |
| `slack-token`         | high     | `xox[baprs]-…` |
| `private-key`         | critical | `-----BEGIN … PRIVATE KEY-----` |
| `jwt`                 | medium   | three-segment JWT shape |
| `bearer-token`        | medium   | `Authorization: Bearer …` |
| `generic-password`    | medium   | hardcoded password literals |
| `gpl-in-mit`          | high     | GPL boilerplate (license-conflict heuristic) |

## Library use

```rust
use gatecheck::{scan, Config, should_fail, Severity};

let cfg = Config::default();
let findings = scan("...your content...", &cfg);
if should_fail(&findings, Severity::High) {
    // abort the commit / fail the CI job
}
```

## Comparison vs alternatives

| Tool | Lang | Cold start | Library API | Pre-commit-friendly |
|---|---|---|:-:|:-:|
| `gitleaks` | Go | ~30 ms | partial | ✅ |
| `trufflehog` | Python | ~600 ms | ❌ | ❌ |
| `detect-secrets` | Python | ~250 ms | ✅ | ⚠️ slow |
| **`gatecheck`** | **Rust** | **~4 ms** | **✅ via `crates.io`** | **✅** |

## Testing

```bash
cargo test
```

14 tests cover: every built-in rule (including OpenAI API keys), exemption patterns, rule-disable config, fail-threshold logic, line numbering, and the doc-test.

## Cross-platform notes

`gatecheck` is a single statically-linked binary (via `cargo build --release` with `lto = true`) and runs on any platform supported by the Rust toolchain.

| Platform | Install method | Notes |
|---|---|---|
| **Debian/Ubuntu** | `cargo install gatecheck` or download release binary | Tier 1 |
| **Arch / Manjaro** | `cargo install gatecheck` | Tier 2 |
| **Fedora / RHEL / Rocky** | `cargo install gatecheck` | Tier 2 |
| **Alpine** | `cargo install gatecheck` (requires musl toolchain: `cargo install --target x86_64-unknown-linux-musl`) | Best effort |
| **WSL2** | Same as Ubuntu — no `/sys/firmware/efi` assumptions made | ✅ |
| **Termux (Android arm64)** | `pkg install rust && cargo install gatecheck` | ✅ |

For Alpine or minimal environments, a pre-built `x86_64-unknown-linux-musl` binary can be used directly without any dynamic library dependencies.

## Roadmap

- v0.2: GitHub Action wrapper; SARIF output for code-scanning UI
- v0.3: Custom user-defined rules via TOML
- v0.4: Whole-file allowlist + `.gatecheckignore` (gitignore-style)

## License

MIT.
