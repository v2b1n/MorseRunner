# Source-to-Rust map

The implementation was developed with reference to the published Morse Runner
1.68 Delphi source in the upstream repository at commit
[`218a1ab53eafc509abc447131229b93f0839b74e`](https://github.com/VE3NEA/MorseRunner/tree/218a1ab53eafc509abc447131229b93f0839b74e).
That revision contains the upstream MPL-2.0 `LICENSE`; the license was added by
Alex Shovkoplyas in commit
[`e7ac6e2cd5b82f54d39dd3e1a3c690b1463bfee2`](https://github.com/VE3NEA/MorseRunner/commit/e7ac6e2cd5b82f54d39dd3e1a3c690b1463bfee2),
and the Pascal units carry the MPL notice. The upstream `Readme.txt` identifies
Morse Runner 1.68 and Alex Shovkoplyas's 2004–2006 copyright.

The original Windows binary and installer were separately inspected locally with
Reverse Engineer Anything (REA) 6.5.0 and GNU `objdump` for behavioral reference.
No original executable, installer, recovered binary code, or disassembly output
was incorporated into or distributed with the Rust project or Debian package.

This map describes implementation references and validation coverage. It is not
a legal determination of authorship or project licensing. Upstream MPL-2.0 and
copyright notices remain in the adapted modules.

| Original | Rust | Fidelity / verification |
| --- | --- | --- |
| DxOper.pas | operator.rs / Operator | Full practice-mode states, matcher, patience, replies and timers; 24,576 original-Pascal cases |
| DxStn.pas | simulation.rs / Station::dx, event, tick | Skills, speed, serial, report, amplitude, offsets, copying/collisions, operator lifecycle; full 1500-block timeline |
| Station.pas | operator.rs / text, report; simulation.rs / Station | Message templates, cut digits, corrected errors, block state transitions |
| MyStn.pas | simulation.rs / Me, Piece, update_call, abort_tx | Deferred piece expansion, TX queue, safe suffix correction, start/end notifications |
| Contest.pas | simulation.rs / Engine::render, me_finished | 11025 Hz / 512 samples, noise, complex mixing, QSK, filters, modulation, AGC, timers, arrivals and confirmation |
| Qsb.pas | dsp.rs / Fading | Three-pass filtered complex noise, Rayleigh gain, warm-up and 128-sample interpolation; numerical Pascal fixture |
| QrmStn.pas | simulation.rs / qrm and Kind::Qrm | Persistent independent interference stations, message choices, patience and timeouts |
| QrnStn.pas | simulation.rs / qrn | Original burst length/amplitude/sparse impulses, along with Contest's background impulses |
| RndFunc.pas | dsp.rs / Random | Original normal/clipped Gaussian, Rayleigh, U-shaped and Poisson distributions; reproducible xorshift stream replaces OS-specific PRNG |
| VCL/MorseKey.pas | morse.rs / encode, envelope | Original integrated Blackman–Harris ramps, rounding, gaps, trailing marker and padding; sample fixtures |
| VCL/QuickAvg.pas | dsp.rs / Average | Cascaded boxcars; numerical fixture and QSB test |
| VCL/MovAvg.pas | dsp.rs / Receiver | Paired three-pass complex LPFs, gain normalization, warm-filter swaps every ten blocks; impulse and whole-path fixtures |
| VCL/Mixers.pas | dsp.rs / Modulator | Original period rounding and sine/cosine recurrence; numerical fixture |
| VCL/VolumCtl.pas | dsp.rs / Agc | Original 155-sample attack/hold, 621-point symmetric log envelope, nonlinear gain law; impulse and full-path fixtures |
| Log.pas | simulation.rs / log, verify_last | Remote completion required; NIL/DUP/RST/NR priority and delayed verification |

## Important porting details uncovered by differential tests

- QSB initialization warms the default filter, then warms the selected slow
  bandwidth, and may warm a third time for flutter. The order matters to the
  shared random stream and subsequent station parameters.
- Station.SendText evaluates NrAsText for its second replacement even when no
  placeholder remains. Those unused formatting draws still affect future events.
- Local report text is expanded when its queued piece starts, not when queued.
- QRM patience is drawn before its callsign, as in its constructor.
- Phase variables are Single but expressions containing TWO_PI and trig evaluate
  at higher precision. Preserve those assignments/wraps to avoid RIT phase drift.
- A contact is verified only after the DX operator reaches osDone. A local log
  entry or complete-sounding reply alone is insufficient.

The GUI is a new implementation. Competition modes, score serialization and
legacy platform drivers remain outside this port. See README for explicit differences.
