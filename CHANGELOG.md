# Changelog

## [1.0.0] - 2026-11-02

### Added

- Publish the Debian 13 (Trixie) amd64 MorseRunner release from the official repository.
- Attach a maintainer-signed SHA-256 manifest and GitHub Artifact Attestation for each release artifact.

### Changed

- Stage tag-triggered releases as drafts only after validating the owner's signed annotated tag.
- Require explicit owner review and publication of the completed release.
