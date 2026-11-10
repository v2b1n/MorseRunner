# Changelog

## [Unreleased]

### Added

- Separate macOS DMG builds for Intel and Apple Silicon.
- Local build-workflow for debian
- UI Grooming
- Release attestations

### Changed

- Stage successful main builds as unpublished draft prereleases until checksum signature and release verification are complete.
- Include both macOS DMGs in the shared checksum manifest and artifact-attestation release checks.

## [1.1.0] - 2026-11-06

### Added

- Windows 10/11 x86_64 portable ZIP and per-user setup installer.
- YAML settings in the user's configuration directory; session logs remain JSON in the user's data directory.
- A standalone Linux amd64 tarball alongside the Debian 13 package.

### Changed

- Build and test release artifacts for Debian 13 amd64 and Windows x86_64.
- Verify release artifact checksums and build provenance before maintainer publication.
