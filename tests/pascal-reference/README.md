# Pascal reference generators

The `.pas` files in this directory are project-specific test drivers, not the
original Morse Runner modules. The Python scripts assemble temporary build
directories under `target/`, copy selected published Delphi units from a
separate local checkout, add headless GUI/audio stubs and compile the drivers
with Free Pascal. They then save reference outputs under `tests/fixtures/`.

The default expected source path is `../MorseRunner-original` relative to the
repository root. That directory is not included in this repository. Set
`MORSE_RUNNER_UPSTREAM` to the absolute path of a local checkout containing the
published Delphi sources before regenerating anything:

```sh
export MORSE_RUNNER_UPSTREAM=/path/to/MorseRunner-original
```

Use Free Pascal 3.2.2 in Delphi mode (the recorded reference environment is
Linux amd64), Python 3, and run these commands from the repository root, in
order:

```sh
python3 tests/pascal-reference/generate.py
python3 tests/pascal-reference/headless.py
python3 tests/pascal-reference/audio.py
python3 tests/pascal-reference/audio.py --qsk-off
```

`generate.py` creates primitive DSP/keyer fixtures and `operator-cases.csv`;
`headless.py` creates `station-timeline.txt`; the two `audio.py` runs create the
full-audio and audio-timeline fixtures for both QSK settings. Intermediate
copies and compiler outputs stay under `target/`. Numerical fixtures created by
the primitive driver are staged under `target/pascal-reference/`; review them
and copy the intended results into `tests/fixtures/` as part of an intentional
fixture refresh.

The scripts adapt GUI and platform plumbing for headless execution and inject a
deterministic random stream. They are validation harnesses, not a supported way
to build the application. Review fixture diffs whenever the upstream source,
compiler, adaptations or fixture bytes change. The source reference and module
mapping are documented in [`../../SOURCE-MAP.md`](../../SOURCE-MAP.md).
