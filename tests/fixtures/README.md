# Test fixtures

These files are checked-in expected outputs for Rust regression tests. They are
not application assets and are not included in the Debian package.
Rust tests load them directly; ordinary test runs do not need the Pascal source
or compiler.

## File formats

- `*.f32` is a headerless sequence of IEEE-754 32-bit floating-point samples,
  each stored little-endian. These are raw numeric data, not WAV files.
- `*-timeline*.txt` and `station-timeline.txt` are deterministic text traces
  from scripted audio or station simulations.
- `operator-cases.csv` contains rows of operator-state input cases and expected
  Pascal results.
The F32 fixtures cover keying envelopes, quick and moving averages, AGC,
modulation, fading, and complete audio paths with QSK enabled or disabled. The
timeline fixtures cover operator/station states and audio events. Rust tests
compare computed results against these expected values, using exact text/state
comparisons or the tolerances defined by each numeric test.

## Regeneration

See [`../pascal-reference/README.md`](../pascal-reference/README.md) for the
required upstream checkout, compiler, commands and generation order. The
primitive numeric outputs are first written under `target/`; the documented
scripts copy scenario outputs into this directory. Regeneration may also depend
on `assets/calls.txt`.
