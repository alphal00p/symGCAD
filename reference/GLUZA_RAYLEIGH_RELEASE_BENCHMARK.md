# Negative Gluza release controls

The original negative Gluza30 and Gluza33 problems each completed and passed
independent verification in three fresh release processes. Native upstream
GCAD timed out in all six corresponding 60-second trials. These are censored
controls, so no finite speedup ratio is reported. The frozen baseline parity
denominator is unchanged.

| Original problem | Release public solve median | Verification median | Raw cells | Native outcomes |
| --- | ---: | ---: | ---: | --- |
| Gluza30, F<0 | 0.185212 s | 0.020968 s | 1 in every trial | 3 timeouts |
| Gluza33, F<0 | 0.164014 s | 0.020934 s | 3 in every trial | 3 timeouts |

Both engines use the exact same original variables, constraints, rational
parameters, variable groups, explicit order, and no merging. Every trial uses
one worker pinned to CPU6, an 8 GiB memory limit, and a 60-second solve limit.
Rust verification has its own 60-second limit. Input mathematical fields and
orders are compared directly in the retained report. No kinematic
specialization, coordinate restriction, or input region was introduced.

The order for Gluza30 is `[x2,x6,x7,x5,x1,x4,x0]`; for Gluza33 it is
`[x6,x1,x4,x0,x2,x7,x5,x3]`. Rust receives the explicit guide expressions in
`gluza30_rayleigh_guide.json` and `gluza33_rayleigh_guide.json`. All exact guide
checking, normalization, restoration, startup and export costs are included
in the public solve measurements. Earlier guide and order research is outside
these fixed-order controls. The algebra caches are cold.

Six native trials ran first, followed by Rust Gluza30/Gluza33 in each of three
rounds. Completion-mode campaigns stop after the first successful repetition,
so rounds one and two use separate fresh campaigns after the initial runs;
all initial measurements remain included. The execution protocol and every
trial's input, result, verification and ledger hashes are recorded.

## Phase costs and memory

| Median internal phase | Gluza30 | Gluza33 |
| --- | ---: | ---: |
| Ordering and exact guide planning | 0.005125 s | 0.007999 s |
| Base projection | 0.002666 s | 0.005969 s |
| Base lifting and fiber restoration | 0.000759 s | 0.002444 s |
| Internal total | 0.008631 s | 0.016514 s |

The phase medians are measured independently and need not sum to the median
total. Public process times additionally include startup and export. Maximum
Rust solve RSS was 6,156/11,680 KiB; maximum verification RSS was
6,172/6,184 KiB. Native maximum RSS was 647,692/180,784 KiB before the
timeouts. Projection growth, all phase samples, and process CPU measurements
are retained in the report.

## Reproduction and hashes

* Comparison: `artifacts/bench/gluza-rayleigh-release-01/comparison.json`,
  SHA-256 `f2039deee37d9a392aef2aeb66ebd6dbe5c16eb25c17f7d2fa91ab40ca9f344e`.
* Release worker: `artifacts/bin/symgcad-linear-negative-256527b`,
  SHA-256 `c9018af88c284ee7de2e6886fe5c3c21d937af16258ee045f33b41126f869c02`.
  Source `256527b` includes the `18cd5d0` proof engine and additive phase
  accounting. Its adjacent build metadata records the path-only dependency
  patch and isolated build command; Symbolica is unchanged.
* Native controls: `artifacts/bench/gluza-rayleigh-native-control-01/`,
  summary SHA-256 `c92e7e7b9bf526c92730c1fce74985267ff12c471e33414bfab61fe470fae53f`.
  Upstream revision: `fb6ebe3aa0e3fdd073f410d01050a4ed7e235f4e`.
* Rust campaign configurations and ledgers are in
  `gluza-rayleigh-release-01/{gluza30,gluza33}/` and
  `gluza-rayleigh-release-01/repeat-{1,2}/{gluza30,gluza33}/`.

The release also reverified the historical eight-cell fixed-A446 proof and
twelve-cell massless symbolic proof. These compatibility checks and hashes
are in `gluza-rayleigh-release-01/historical/summary.json`.

These negative-domain proofs do not count as positive-domain solutions or as a
complete sign arrangement of either polynomial. Positive Gluza30 was solved
subsequently by a separate [bilinear proof and release campaign](GLUZA30_BILINEAR_RELEASE_BENCHMARK.md);
positive Gluza33 remains unresolved.
