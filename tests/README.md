# Test data and reference programs

This directory contains Rust tests, checked-in reference outputs and small
programs used to regenerate those outputs. Normal `cargo test --locked` runs
only the Rust tests and reads the checked-in files; it does not need Delphi,
Free Pascal or a checkout of the original project.

- [`fixtures/`](fixtures/README.md) explains the `.f32`, timeline, CSV and
  provenance files consumed by tests.
- [`pascal-reference/`](pascal-reference/README.md) explains the Pascal test
  drivers, their upstream source dependency and how to regenerate references.

Regenerating reference data changes test expectations. Review every changed
fixture and its provenance before accepting such a change.

Station and audio reference fixtures use the application's current
`assets/calls.txt`; regenerate them when that list changes. The QSO protocol
integration test instead uses a fixed test callsign so it remains independent
from the user's customizable dataset.
