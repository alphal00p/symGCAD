# Bounded three-parameter binary-segmentation research

This is an example-only driver around the existing public exact kernel. It does
not change production dispatch, resource defaults, operation caches, proof
schemas, or verification. The large Gluza calculation has **not been run** with
this driver. A completed algebra candidate would not establish CAD coverage.

## Why this experiment is distinct

The authentic Gluza33 `x4` discriminant capture has complete variable map
`[x2,x1,x7,x4,x6,x0,x5]`, degrees `(10,9)` for `p,p'`, and three surviving
parameters. Its capture SHA-256 is
`aab900ab332f90a1d0c9d32ec73cdd65b88d64b22a45d20e23ef77e33aa70289`;
the ordered operand digest is
`73189be967be55624321db0c39697b0d0d5cfeb8e8a49aa17869e21e477fb4b0`.
The earlier inspect-only prototype found parameter degree bounds `(166,152,90)`,
2,325,141 digits, 660 bits per digit, and a 1,534,593,060-bit encoded output
bound. A support-only calculation also bounds the total encoded input at
1,544,757,060 bits. These exceed several production limits, not just its
maximum of two parameters. The original plan is retained under
`artifacts/research/gluza33-three-parameter-segmentation-plan-01`.

Both the native CRT call and one exact finite-field image timed out at 300
seconds; see [the one-prime diagnostic](DETERMINISTIC_CRT_PRIME_IMAGE.md).
Neither timeout identifies a cheap internal stage. The completed A377 D32
segmentation used 1,534.7 seconds inside the integer resultant for a much smaller
210-million-bit bound. That is not a runtime or peak-memory prediction for
this operand. A full Cartesian numerical grid here would require more than
2.3 million images and is not proposed.

## Exactness and resource envelope

`examples/profile_multivariate_segmentation.rs` calls the unchanged
`resultant_segmentation::plan` and `try_resultant` APIs with explicit research
options: one to three surviving parameters, at most 2,500,000 digits/output
terms, and at most 2,000,000,000 encoded input and output bits. All other kernel
limits and admission thresholds remain their defaults. These are generic
shape/resource limits; no fixture name or checksum selects an algorithm.

The existing kernel clears positive rational denominators, bounds every
integer resultant coefficient by the Sylvester row-1-norm product, and chooses
a radix larger than twice that bound and the input coefficients. Its mixed
radix box also includes the input support, preserving the eliminated degrees.
The integer resultant is deterministic Ducos. Centered decoding checks bounds
and exactly re-encodes the answer before restoring rational scale. Increasing
the admitted parameter count does not alter this argument or add a probabilistic
termination criterion. See [the kernel note](BINARY_SEGMENTATION_RESULTANTS.md).

The driver adds the following constraints:

- Authenticate an at-most-8-MiB regular capture and its ordered operand digest.
  Validate bounded ASCII names before symbol registration; parse only the
  existing bounded literal sparse grammar in the full ordered map.
- Before the kernel call, account for rational denominator restoration across
  the entire possible output support. Its logical coefficient storage must fit
  2 billion bits. A conservative decimal/exponent/JSON bound must also fit a
  1-GiB sparse candidate. A failed preflight never enters the resultant.
- For discriminants, derive `p'` internally and compute the exact quotient
  `Disc(p)=(-1)^(m(m-1)/2) Res(p,p')/LC(p)`. Require zero polynomial remainder;
  reattach mapless constants and preserve signs, rational scale, and all
  leading-coefficient/repeated-root loci. This is a polynomial identity, not
  a domain restriction requiring the specialized leading coefficient nonzero.
- After exact LC division, recheck full map, eliminated support, parameter
  degree box, term count, actual rational coefficient bits and candidate bytes.
  The pre-division integer bound does **not** assert a bound on transient
  division memory or the quotient's coefficients.
- Stream coefficient strings and full exponent arrays into a bounded partial
  file, with a running SHA-256. Publish `candidate.json` atomically only after
  success; do not duplicate a giant expression or construct a terms JSON array.
  Candidate metadata explicitly reports independent verification and original
  coverage as false. No candidate is inserted into any production cache.

The maximum supervisor envelope is 7,200 seconds and 51,200 MiB. The parent
only handles bounded metadata and streaming hashes; all polynomial work,
including parsing, LC division and export, occurs inside the bounded worker.
Logical bit/text limits do not bound native intermediate allocations. The
outer worker supervisor remains essential.

Progress names only observable stages. In particular,
`segmentation_encoding_native_decode` is one composite call: no private phase
is guessed. Successful kernel statistics provide the actual substage timings.
`process.json` persists terminal status, times, RSS and stdout/stderr hashes
before fallible output preservation or post-run input hashes. Separate
`audit.json` accepts completion only for exit zero, untruncated output,
unchanged capture/binary, and a matching candidate hash. Static decline and
plan-only completion are distinct from algebra completion.

## Validation and reproducibility

Six new tests compare genuine three-parameter resultants/discriminants against
deterministic integer Ducos, covering signed rational scaling, symbolic leading
coefficients and degree-drop loci, repeated-factor zero, permuted/inactive maps,
malformed inputs/checksums, resource declines, and exact bounded sparse-stream
replay. Together with ten reused parser/profiler tests, **16 tests pass**.
Independent source review found no mathematical or routing blocker.

The development/test-profile executable was built with only the command-line
`profile.test.package.symgcad.debug=0` override; this is not a release timing
build. Test/build took 11/7 seconds with sampled aggregate peaks
848,796/829,520 KiB under a 2-GiB supervisor. The final pair fixture additionally
uses a negative scalar raised to the odd resultant exponent three; its affected
test passed, followed by a fresh build (10/5 seconds, 822,676/779,752 KiB).
The original build remains preserved separately. The final frozen executable is
`artifacts/research/multivariate-segmentation-driver-02/bin/profile_multivariate_segmentation`,
SHA-256 `7ae0868ec037c74212f54c613072dc9bf8159752816882962c665b113af39089`.
Source hash `16737a522b9f657ace6e75c68521ad4db04e677a52132a025cead21555a8f9de`
and the unchanged parser/kernel/manifest hashes are in
`artifacts/research/multivariate-segmentation-driver-02/source-binary.sha256`.
Test log SHA-256 is
`432b992593ad6adafb70cd437f46f9168e005f7865ca2890f6d54f6aa7f36802`;
build log SHA-256 is
`8a57abf4fdd2571d37ace5a27a26c2ef315e86b4ceb7783f3340e029707b6ba9`.

The final executable's authenticated **plan-only** replay completed in
0.041745 seconds / 7,044 KiB on CPU 3, with a 10-second/2-GiB worker cap.
The public planner confirms the earlier digit/output bound and the
1,544,757,060-bit encoded-input bound. Both input denominators are one; the
restored logical coefficient bound is 1,534,593,060 bits and the conservative
sparse-output bound is **674,356,426 bytes**, below 1 GiB. The report is
`artifacts/research/multivariate-segmentation-driver-02/gluza-plan-only/audit.json`.
Its SHA-256 is
`fb0db7b5f56748276a801e9a8e153a095cc04af717b6ffb84ce5177d8791cfa2`;
capture and binary hashes were unchanged. This replay did not compute an
encoded resultant, rational resultant, discriminant, or root inventory.

Build the example with `cargo build --locked --example
profile_multivariate_segmentation`. For an authenticated capture, a cheap
preflight uses `--capture-sha256 SHA --output FRESH_DIR --plan-only --seconds
10 --memory-mib 2048`. Omitting `--plan-only` permits computation. A larger
resource envelope requires explicit scheduling after the original campaign
ends; no production cap escalation or performance claim follows from this
preparation.
