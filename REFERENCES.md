# Reference projects studied

During the audit cycle for this repository the following established secret-scanning projects were reviewed. One concrete, actionable pattern from each is noted below.

| Project | Stars | License | Pattern adopted |
|---|---|---|---|
| [gitleaks](https://github.com/zricethezav/gitleaks) | 18k+ | MIT | Per-rule `allowlist` regex field — mirrors our `exempt_patterns` config key |
| [trufflehog](https://github.com/trufflesecurity/trufflehog) | 17k+ | AGPL-3.0 | Verified-secret HTTP callbacks on findings — out of scope for v0.1 but noted for v0.2 |
| [detect-secrets](https://github.com/Yelp/detect-secrets) | 3k+ | Apache-2.0 | Baseline file (`.secrets.baseline`) that snapshots known findings so only net-new ones fail the gate |
| [kingfisher](https://github.com/mongodb/kingfisher) | 1k+ | Apache-2.0 | Rule definitions live in a separate data file (TOML/YAML), not inlined in source — target for v0.3 user-defined rules |
| [secretlint](https://github.com/secretlint/secretlint) | 1k+ | MIT | Plugin-per-rule architecture with a shareable config ecosystem — maps well to our planned `extras` system |

## Audit findings (2026-05-02)

### `src/lib.rs`

| # | Category | Finding | Fix applied |
|---|---|---|---|
| 1 | Correctness | `github-fine-grained` regex used exact `{82}` quantifier; PAT lengths may increase in future GitHub format changes | Changed to `{82,}` |
| 2 | Correctness | No rule for OpenAI `sk-proj-*` API keys (format change since 2023) or classic `sk-` format | Added `openai-api-key` rule covering both `sk-proj-…` / `sk-svcacct-…` and classic `sk-` |

### `src/main.rs`

| # | Category | Finding | Fix applied |
|---|---|---|---|
| 3 | Correctness | `Severity` was imported at crate level but only used inside `#[cfg(test)]`, causing a compiler warning | Moved import into test module |

### Repository root

| # | Category | Finding | Fix applied |
|---|---|---|---|
| 4 | Integration | No `.pre-commit-hooks.yaml` in repo root; pre-commit.com framework **requires** this file to auto-install hooks | Created `.pre-commit-hooks.yaml` |
| 5 | Doc↔code | README claimed "9+ built-in rules" after only 9; table was missing `openai-api-key` | Updated count and table |
| 6 | Doc↔code | README mentioned pre-commit framework but gave no `.pre-commit-config.yaml` snippet | Added example snippet |
| 7 | Cross-platform | No Termux / Alpine install docs | Added cross-platform table to README |

### Tests

| # | Category | Finding | Fix applied |
|---|---|---|---|
| 8 | Test coverage | No test for `github-fine-grained` rule | Added `detects_github_fine_grained_pat` test |
| 9 | Test coverage | No test for OpenAI key rules | Added `detects_openai_classic_key` and `detects_openai_project_key` tests |

## Known follow-ups

- **v0.2**: Add musl cross-compilation target to CI matrix (`x86_64-unknown-linux-musl`) and publish pre-built binaries for Alpine/Termux.
- **v0.2**: SARIF output (`--format sarif`) for GitHub code-scanning UI integration.
- **v0.3**: User-defined rules via TOML (inspired by kingfisher's data-driven rule files).
- **v0.4**: `.gatecheckignore` (gitignore-style whole-file allowlist).
- **detect-secrets-style baseline**: snapshot known findings so only net-new secrets fail the gate.
