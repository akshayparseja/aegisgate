# Contributing to AegisGate

Thank you for wanting to contribute — your help improves AegisGate for everyone. This document explains how to report issues, propose changes, and submit patches in a way that allows us to review and merge them quickly. I (the maintainers) value clear, test-backed contributions and a respectful community.

Table of contents
- [Quick start](#quick-start)
- [Reporting bugs & requesting features](#reporting-bugs--requesting-features)
- [Working on a change (PR workflow)](#working-on-a-change-pr-workflow)
- [Commit message & sign-off (DCO)](#commit-message--sign-off-dco)
- [Code style & linters](#code-style--linters)
- [Testing locally](#testing-locally)
- [Continuous integration / checks](#continuous-integration--checks)
- [What we review](#what-we-review)
- [License & copyright](#license--copyright)
- [Code of conduct](#code-of-conduct)

---

## Quick start

1. Fork the repository on GitHub and clone your fork:
   - `git clone git@github.com:<your-username>/aegisgate.git`
2. Create a named branch for your work:
   - `git checkout -b feat/short-description` or `bugfix/short-description`
3. Make changes, run tests and linters locally.
4. Commit with a clear message and a DCO sign-off (see below).
5. Push and open a Pull Request (PR) against the `main` branch of the upstream repository.

---

## Reporting bugs & requesting features

- Search existing issues first to avoid duplicates.
- For bugs, include:
  - Steps to reproduce
  - Expected vs actual behavior
  - Version / commit / Docker image used
  - Minimal reproduction (config, traffic example, or script)
  - Relevant logs and metrics (redact secrets)
- For feature requests, include:
  - Use case and value proposition
  - Example configuration or CLI
  - Backwards compatibility concerns

If you're unsure, open an issue describing the idea — maintainers will triage and guide next steps.

---

## Working on a change (PR workflow)

Preferred workflow:
- Fork → Branch → Commit → PR

Guidelines:
- Keep PRs focused (one logical change per PR).
- Rebase on `main` rather than merge `main` into feature branches to keep history simple, unless the PR is large and needs frequent merges.
- Include tests for bug fixes and features whenever practical.
- Add or update documentation (`README.md`, `config/*.yaml.example`, or docs folder) for any user-visible change.
- Mark work-in-progress PRs as draft until ready for review.

PR checklist (what the submitter should ensure)
- [ ] Code compiles and tests pass locally
- [ ] `cargo fmt` applied and `cargo clippy` addressed (see code style)
- [ ] Added/updated tests where applicable
- [ ] Updated README/config docs if user-visible
- [ ] Signed-off (DCO) on commits

---

## Commit message & sign-off (DCO)

We use the Developer Certificate of Origin (DCO) to track contributor agreement that you have the right to submit the code.

- Sign your commits with a `Signed-off-by` trailer:
  - Example commit command:
    - `git commit -s -m "feat: validate CONNECT packet length\n\nAdd a parser guard for maximum CONNECT remaining length."`
  - Or add manually to the commit message body:
    - `Signed-off-by: Jane Developer <jane@example.com>`

By signing-off you assert that:
- You created the patch or otherwise have the right to contribute it under the project license, or
- You have the right to submit work you are contributing from a third party.

If you must contribute via GitHub web editor and cannot sign with `-s`, add this line to the PR description:
```
Signed-off-by: Your Name <your-email@example.com>
```
But CLI `git commit -s` is preferred.

PRs missing signed-off commits may be blocked until signed-off or explicit contributor permission is provided.

---

## Code style & linters

AegisGate is written in Rust. Keep the repository consistent:

- Formatting: `cargo fmt --all`
- Lints: `cargo clippy --all -- -D warnings`
- Rust edition: 2021 (check `Cargo.toml` in each crate)
- Keep changes small and readable; prefer expressive naming and short functions.

For shell / scripts:
- Follow POSIX / project conventions
- Add `#! /usr/bin/env bash` or appropriate shebang and make scripts executable.

---

## Testing locally

Unit and integration tests:
- Run crate tests:
  - `cargo test --manifest-path crates/aegis-proxy/Cargo.toml`
- Run the whole workspace tests (from repo root):
  - `cargo test --workspace`

Docker-based quick integration:
- `docker-compose up -d` (refer to `docker-compose.yaml`)
- Use the included `test_http_detect.py` or equivalent scripts to reproduce cases.

If you add a failing test, include a short note in the PR describing the failure case.

---

## Continuous integration / checks

We aim to run CI on each PR (tests, formatting, clippy). If CI is not yet configured for a branch, run the checks locally before opening a PR.

Suggested local commands:
- `cargo fmt --all -- --check`
- `cargo clippy --all -- -D warnings`
- `cargo test --manifest-path crates/aegis-proxy/Cargo.toml`

---

## What we review

During review we look for:
- Correctness and safety (security-sensitive code gets more scrutiny)
- Tests that cover the change
- No leaking secrets or credentials
- Documentation for user-visible changes
- Performance regressions for critical paths (proxying, parsing)
- Proper error handling and logging

Reviewers may ask for changes; be responsive and keep PRs up-to-date (rebase if requested).

---

## License & copyright

- The repository currently contains a `LICENSE` file at the root. Please ensure your contributions are compatible with that license.
- If you plan to change the project license (for example, to Apache-2.0), mention it in an issue and get consensus from maintainers before submitting large contributions that depend on the license change.

By contributing (signing off on commits) you agree to license your contribution under the project's license.

---

## Code of conduct

Be respectful and collaborative. See `CODE_OF_CONDUCT.md` (if present) for details. If you encounter harassment or have any community concerns, contact the maintainers privately.

---

If you prefer, I can:
- Add a `CONTRIBUTING.md` file to the repo with this text,
- Add a `CODE_OF_CONDUCT.md` template,
- Prepare a GitHub Actions CI workflow that runs fmt, clippy and tests on PRs.

Tell me which of those you want me to create next and I’ll add them. Thanks again — your contributions help make AegisGate more robust and easier to use.