# Third-party notices

The CW simulation adapts and ports selected behavior and algorithms from
the published Morse Runner 1.68 Delphi source. The upstream source repository at
commit `218a1ab53eafc509abc447131229b93f0839b74e` includes an MPL-2.0 license and
MPL notices in its Pascal units. The Rust source remains under this repository's
MPL-2.0 license; upstream attribution is retained in affected files. See
`SOURCE-MAP.md` for the source-level map.

Direct Rust dependencies are listed in `Cargo.toml` and pinned in `Cargo.lock`.
The Debian package includes `THIRD_PARTY-LICENSES.md`, generated from the complete
resolved dependency graph, with each crate's license text and attribution.
