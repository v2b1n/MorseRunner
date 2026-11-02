# Agent instructions — MorseRunner

## Purpose

This is the official source repository for MorseRunner. Prioritize reviewable changes, reproducible builds, security, clarity and accurate attribution.

## Contribution workflow

- Work on feature branches and submit pull requests; do not push directly to `main` unless explicitly authorized by the maintainer.
- Keep a PR focused on one approved deliverable. Prefer one well-structured signed commit for maintainer-directed feature imports; ordinary external contributions can follow normal review conventions.
- Reference relevant issues **in this repository**, and keep PR descriptions concise: Summary, Testing, user-visible changes, limitations and issue linkage.
- Do not copy internal planning notes, lengthy ticket histories, credentials or unrelated discussions into public PRs.
- Never merge your own PR or create/publish releases without explicit maintainer authorization.

## Quality and compatibility

- Run the applicable formatting, lint, tests and build checks; describe what was actually tested.
- For latency-sensitive behavior, include performance or manual validation where available; never claim platform testing that was not performed.
- Keep release/platform documentation accurate. Do not claim macOS or Windows support before validation and release.
- Preserve copyright headers, third-party notices, license requirements and relevant `SOURCE-MAP.md` / `THIRD_PARTY.md` attribution. Follow MPL-2.0 obligations.

## CI and release security

- Pin all GitHub Actions and reusable workflows used with `uses:` to immutable full-length commit SHAs; annotate the intended upstream version for maintainability.
- Keep pull-request workflows least-privileged. Never expose release credentials to untrusted PR code or use unsafe `pull_request_target` checkout patterns.
- Official artifacts must be produced by the public release workflow, accompanied by a complete `SHA256SUMS`, maintainer-signed `SHA256SUMS.asc` and GitHub Artifact Attestations for applicable build artifacts.
- Publishing and cryptographic release authorization are maintainer-controlled; do not bypass the review and signing steps.

If a request conflicts with these rules, ask the maintainer before making a public change.
