# Why AegisGate uses the Apache License 2.0

Short summary
- We chose the Apache License 2.0 (SPDX: `Apache-2.0`) because it is permissive and widely accepted while also providing an explicit contributor patent grant. That combination encourages corporate adoption (important for security/network tooling) without forcing derivative works to be open-source.

Key reasons (concise)
- Permissive and familiar: like MIT/BSD in practical terms for reuse, distribution, and inclusion in proprietary products.
- Explicit patent grant: contributors grant a patent license for their contributions, which reduces legal uncertainty for companies and downstream users.
- Patent termination/retaliation clause: discourages patent litigation against the project and downstream users.
- Corporate-friendly attribution: Apache's NOTICE mechanism provides a clear way to surface required attributions if and when third‑party components demand it.
- Tooling and ecosystem fit: many cloud, security, and infra projects use Apache‑2.0; it removes a common blocker for enterprise adoption.

What Apache‑2.0 means for you and downstream users
- You are free to use, modify, distribute, and sublicense the software (including in proprietary products), provided you:
  - Include the Apache‑2.0 license text in distributions,
  - Preserve copyright and license notices,
  - If a `NOTICE` file is present, propagate its content per the license rules.
- Contributors grant patent rights for their contributions (subject to the license's terms).
- If you initiate patent litigation over the project, your patent license from contributors terminates.

Practical next steps (recommended actions)
1. Replace the repository `LICENSE` with the full Apache‑2.0 text (we will add this file at the repo root).  
2. Update `Cargo.toml` and other packaging metadata:
   - Example: `license = "Apache-2.0"` (or `license = "Apache-2.0"` in the workspace/package sections).
3. Add a small `NOTICE` file only if you need to surface third‑party required attribution or notices. Most projects do not need a `NOTICE` unless a dependency requires it.
4. Add repository badges (README): license badge from shields.io, CI, and crates/published links to improve trust.
   - Example badge: `https://img.shields.io/badge/license-Apache%202.0-blue.svg`
5. Add governance & contribution docs to encourage safe contributions:
   - `CONTRIBUTING.md` — process for issues, PRs, tests, and a contributor sign-off (DCO) or CLA if you plan to accept corporate contributions.
   - `CODE_OF_CONDUCT.md` — community standards for respectful collaboration.
6. Consider adding a short `LICENSE-CHOICE.md` (this file) so maintainers and legal reviewers understand the rationale.
7. If you plan to accept contributions from others (especially companies), consider:
   - Using a Developer Certificate of Origin (DCO) or a simple Contributor License Agreement (CLA).
   - Adding CI that runs tests on PRs and a checklist in `CONTRIBUTING.md` so maintainers can accept contributions with confidence.

Notes & caveats
- Dual‑licensing (e.g., `Apache-2.0 OR MIT`) is common in Rust ecosystems; it can increase compatibility. If you later want to allow `MIT` users, you can relicense new contributions or adopt dual licensing with care.
- Apache‑2.0 is not copyleft; if you want every derived distribution or hosted service to publish source changes, choose AGPLv3 instead — but that reduces adoption by companies and many users.
- Apache‑2.0 does not remove trademark or patent ownership; it only grants specified freedoms. You should be explicit about trademarks if you care about branding.

Suggested short README snippet to display the choice
- License: Apache‑2.0 — permissive license with patent protection. See `LICENSE` for details.

If you'd like, I can:
- Create/replace the repo `LICENSE` file with the official Apache‑2.0 text,
- Add a `CONTRIBUTING.md` with a recommended DCO sign-off template and contributor workflow,
- Add `CODE_OF_CONDUCT.md` using a standard template,
- Update `Cargo.toml` snippets to reflect the new `license = "Apache-2.0"` setting,
- Add README badge examples and a small `NOTICE` template if required.

Tell me which of those you want me to add next and I will prepare the files.