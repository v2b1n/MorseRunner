# MorseRunner

A CW contest practice application built with Rust.

## Install

Releases are available for Linux amd64 as a Debian package (tested on Debian 13) or a standalone `MorseRunner` binary, and for Windows 10/11 x86_64 (tested on Windows 10).

**Pre-releases** are automatically published on every successful build. They are available for testing but have not yet been tested by the maintainer. Once a build has been tested and approved, it is promoted to a release without rebuilding.

### Release verification

Official release artifacts are accompanied by maintainer-signed SHA-256 checksums and GitHub Artifact Attestations.

To verify a downloaded release:

1. Verify the maintainer's GPG signature on `SHA256SUMS`.
2. Check the downloaded artifact against its SHA-256 checksum.
3. Verify its GitHub build provenance using `gh attestation verify`. GitHub cli is available at

**Example (Debian package):**

```bash
gpg --verify SHA256SUMS.asc SHA256SUMS
sha256sum -c SHA256SUMS
gh attestation verify morse-runner_1.0.25_amd64.deb -R v2b1n/MorseRunner
```

The maintainer's [public GPG key](https://keys.openpgp.org/vks/v1/by-fingerprint/C73C58F2A4662E48537C77CD17E233FD523708C0) is available for signature verification.

GPG signatures authenticate the release checksums, while GitHub Artifact Attestations provide verifiable information about where and how the artifacts were built. All release packages are listed in one `SHA256SUMS` manifest. Pre-releases have an unsigned manifest until the maintainer attaches `SHA256SUMS.asc` for promotion. On Linux or macOS, verify the manifest with `sha256sum --check SHA256SUMS`.

### Debian 13 (amd64)

Install the `.deb` package with:

```sh
sudo apt install ./morse-runner_*_amd64.deb
```

### Windows 10/11 (x86_64)

Download either the portable `.zip` or the setup `.exe` and compare its hash with the corresponding `SHA256SUMS` entry using `Get-FileHash -Algorithm SHA256 <file>` in Windows PowerShell. Extract the ZIP and run `morse-runner.exe` to use the portable version. The setup installer installs for the current Windows user under `%LOCALAPPDATA%\Programs\MorseRunner`, so administrator permission is not required. Uninstall from **Settings → Apps → Installed apps** or the MorseRunner Start menu shortcut.

The Windows executable and installer are not code-signed. Microsoft Defender SmartScreen may show an unknown-app warning, and Windows 11 Smart App Control or an organization policy may block unsigned programs. See [Microsoft's SmartScreen reputation guidance](https://learn.microsoft.com/windows/apps/package-and-deploy/smartscreen-reputation).

## Operation

1. Set **MY CALL**, speed, duration and conditions; start session.
2. **Realistic band** enables QRM, QRN, QSB, flutter and LIDS together. Conditions can also be selected independently. Flutter requires QSB; the bandwidth is selected for each new caller, as in the original.
3. **F1 / Enter** sends CQ. Caller counts follow the original Poisson distribution: occasionally nobody answers. Single-call mode starts callers automatically.
4. Copy a callsign, then Enter to send their call and your report/serial.
5. Listen to the actual reply. A caller may request `NR?` / `AGN`, repeat its call, correct a sent number with `EEEEE`, call again or give up. Fix a partial/wrong callsign and send it again; stations are independent, not scripted UI replies.
6. Copy report/serial and press Enter to send TU + your call and log the QSO. A log entry becomes verified only once the remote operator completes the QSO.

Key bindings:

* **F1** CQ
* **F2** report/serial
* **F3** TU + your call + log
* **F4** your call
* **F5** their call
* **F6** QSO B4
* **F7** ?
* **F8** NIL
* **Escape** cancels TX
* **F11** clears entry
* **PageUp/PageDown:** change speed
* **Tab** switches fields
* **Ctrl+Enter** invokes TU/log in this version

"Learning aid" checkbox reveals a station's true call/report for practice.

QSK now suppresses RX during keyed marks and restores it between them with the original gain recovery. With QSK off, RX is muted for the entire transmission. Sidetone adjusts self-monitoring independently of the master Volume.

## State and records

Settings are saved as YAML in the XDG configuration folder:

- `${XDG_CONFIG_HOME:-~/.config}/morse-runner/settings.yaml`

The settings include the station callsign, WPM, pitch, bandwidth, RIT, volume, sidetone, noise, session duration and mode, activity, and band conditions. They are saved automatically after changes and restored at the next launch. If the YAML file does not exist, the application starts with default settings.

Session logs are JSON in the XDG local data folder, normally `${XDG_DATA_HOME:-~/.local/share}/MorseRunner/`. Every session gets its own log, saved after each change; CSV exports next to it. NIL, DUP, RST and NR follow the original verification priority. A temporary NIL while TU is still being sent can be replaced by the verified result when the remote station acknowledges completion. A successful local transmission alone never guarantees a successful contact.

## Engine and signal model

- Original DX operator state machine, skills, patience, repeats, response delays, timeouts and partial/wildcard callsign matching, including LIDS probabilities.
- DX stations hear transmissions only when they are listening; collisions and missed/partial messages influence their subsequent behavior.
- Original station speed, serial, amplitude and frequency-offset distributions.
- Original cut-number rules and one-shot digit errors with EEEEE correction.
- Persistent QRM operators (QRL/QSY/CQ, patience and repeat timers), stochastic QRN impulses and bursts, filtered Rayleigh QSB and fast flutter channels.
- Integrated Blackman–Harris keying edges, 11025 Hz / 512-sample simulation clock, complex I/Q mixing, RIT, paired/warmed three-pass moving-average receiver filters, carrier modulation and symmetric look-ahead AGC.
- Original keyed QSK attenuation/recovery, queued TX pieces and safe editing of not-yet-transmitted callsign suffixes.

## Build

```sh
cargo run --locked
cargo test --locked
cargo clippy --locked -- -D warnings
cargo build --release --locked
./target/release/morse-runner --audio-check
./packaging/build-deb.sh
sudo apt install ./dist/morse-runner_*_amd64.deb
```

The Debian package is built and checked on a clean Debian 13 amd64 environment. Build prerequisites are Rust stable, build-essential, pkg-config, libasound2-dev, libx11-dev, libxi-dev, libxrandr-dev, libxcursor-dev, libxinerama-dev, libwayland-dev, libxkbcommon-dev, libgl1-mesa-dev, dpkg-dev, lintian and `cargo-about` (install with `cargo install --locked --features cli cargo-about`). Audio uses the system's default output device. The microphone is not used. Audio failures are displayed in the status area.

## Verification against the original Pascal

`tests/pascal-reference/` runs the actual upstream algorithm units with Free Pascal and small GUI/file-driver stubs. The same explicit random stream is injected into both implementations; this replaces Delphi's platform PRNG, not its distributions. The captured fixtures are checked in, so ordinary `cargo test` needs no Pascal tools.

The suite covers 24,576 operator cases, a 1,500-block station timeline, a full noisy QSK session, a second QSK-off / 200 Hz BW / negative-RIT session, numerical keyer / filter / QSB / modulator / AGC outputs, and successful/failed two-way contacts. Whole-path waveform comparisons enforce relative RMS and peak absolute errors below 0.0001 (normalized PCM). This establishes those tested scenarios, not every possible setting or bit-identical output on every CPU/driver.

See [`tests/README.md`](tests/README.md), [`tests/fixtures/README.md`](tests/fixtures/README.md) and [`tests/pascal-reference/README.md`](tests/pascal-reference/README.md) for the fixture formats, their purpose, and the complete regeneration procedure.

To regenerate fixtures, install `fp-compiler`, set `MORSE_RUNNER_UPSTREAM` to a local checkout of the published Delphi sources, then run:

```sh
export MORSE_RUNNER_UPSTREAM=/path/to/MorseRunner-original
python3 tests/pascal-reference/generate.py
python3 tests/pascal-reference/headless.py
python3 tests/pascal-reference/audio.py
python3 tests/pascal-reference/audio.py --qsk-off
```

The primitive `.f32` files generated by the first script live in `target/pascal-reference/`; copy them to `tests/fixtures/` when intentionally refreshing references. The scenario scripts copy their own outputs. Regeneration must be followed by the complete tests and review of any changed golden files.

## Support and remaining differences

- WPX/HST competition modes, official score strings/leaderboards, WAV recording, rate histogram and some original key bindings are not implemented.
- The receiver display is schematic station activity, not a measured FFT.
- GUI F3 intentionally sends TU + MY CALL, recruiting new callers using the original engine's TU+MYCALL rule; it is not the original TU-only F3 macro.
- Default practice settings and UI are modernized; use Realistic band to enable the original effects. Random seeds/PRNG sequence differ from the old binary.
- Receiver accumulation uses f64 for numerical stability; model inputs/outputs retain f32. Floating-point differences are measured in the reference tests.
- Original internal audio is linearly resampled only at the final device boundary; the OS/driver may process it differently from the old Windows audio stack.
- Keying rise length is capped at the dot length for extreme WPM values, avoiding the original short-dot/long-ramp buffer underflow. The UI currently caps WPM at 80.
- Debian 13 (Trixie) amd64 is the only supported and tested Linux distribution for this release. Windows 10/11 x86_64 has a portable ZIP and per-user installer; Windows 10 has been manually tested. macOS is not currently supported.

## Source and licensing

Selected Rust modules adapt algorithms and behavior from the published Delphi source for Morse Runner 1.68. `SOURCE-MAP.md` identifies the corresponding modules. The upstream MPL-2.0 license and copyright notices remain credited in the affected code. Third-party dependency notices are in `THIRD_PARTY.md` and the generated license bundle shipped with packages.

## Development Notes

This Rust reimplementation was developed with assistance from OpenAI Codex under the direction and review of the project author. Original Morse Runner 1.68 behavior was investigated with Reverse Engineer Anything (REA) 6.5.0 and GNU `objdump` on the original Windows executable, with reference to Alex Shovkoplyas's published [Delphi/Pascal source at commit `218a1ab`](https://github.com/VE3NEA/MorseRunner/tree/218a1ab53eafc509abc447131229b93f0839b74e). No original Windows executable or installer, recovered binary code, or disassembly output was incorporated into or distributed with the Rust project or Debian package. Selected algorithms and behavior are intentionally adapted or ported from the published Pascal source. Applicable upstream MPL-2.0 notices and Alex's attribution remain in those modules; project-specific contributions retain Vladimir's notices.

## Project author & maintainer

Vladimir Vecgailis, DM2VV.

OpenPGP fingerprint: `C73C 58F2 A466 2E48 537C 77CD 17E2 33FD 5237 08C0`. Public key: [OpenPGP key registry](https://keys.openpgp.org/vks/v1/by-fingerprint/C73C58F2A4662E48537C77CD17E233FD523708C0).

## Acknowledgements

With sincere gratitude and respect to Alex Shovkoplyas (VE3NEA), the original author of Morse Runner, whose published Delphi source made this Rust reimplementation possible: <https://github.com/VE3NEA/MorseRunner>. This project is not affiliated with or endorsed by the original author.

