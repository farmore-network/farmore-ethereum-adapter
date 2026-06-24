# Contributing to Farmore

Farmore is permissionless: contributions, new chain adapters, and integrations are
welcome from anyone, no approval required.

## Ground rules

- **Never commit secrets.** Private keys, mnemonics, `.env`, RPC/API keys must never enter
  git history. A `gitleaks` pre-commit hook and CI gate enforce this — install the hook:
  ```bash
  pipx install pre-commit && pre-commit install
  ```
- **Keep CI green.** Every PR must pass: `cargo test`, `cargo clippy -- -D warnings`,
  `cargo fmt --check`, `cargo audit`, `cargo deny check`, and the secret scan.
- **No regressions in value-bearing logic or test depth.**

## Workflow

1. Fork and branch from `main`.
2. Make focused changes with tests.
3. Run the full local checks above.
4. Open a PR with a clear description. Sign your commits (`git commit -S`) — `main`
   requires signed commits.

## Dependency hygiene

Pin dependencies; commit lockfiles where applicable; justify any new dependency in the PR.
