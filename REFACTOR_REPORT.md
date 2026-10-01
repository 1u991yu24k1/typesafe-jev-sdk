# Refactor / verification report

Repository: https://github.com/1u991yu24k1/typesafe-jev-sdk
Baseline: `92fba3c06ffc04c12d06f6703e64ae376237cbbc`

## Changes

- `JevRequestBuilder`: replace the unnecessary `Option<HashMap<...>>` with an empty map, use direct insertion, and allocate the default model only after successful validation when no custom model exists. Add `Default` without changing the existing constructor or method signatures.
- `TypeSafeClient`: pass the stored parsed `Url` directly to reqwest (a clone, without reparsing its string on every request), make the fallback endpoint allocation lazy, and simplify proxy conversion and response handling. Connection pooling and current timeout/status behavior are preserved.
- Correct the explicitly approved `Usage::exceeds_budget` bug: return true when input OR output exceeds its corresponding limit; equality is within budget.
- Replace the live API test with local HTTP mocks. Add builder validation/replacement tests, serialization and malformed-payload tests, timeout/error/redaction checks, and isolated child-process configuration tests. Tests never require a real API key or contact the API.
- Add reproducible timing and allocation benchmarks, a production-line coverage summarizer, and development commands. Apply rustfmt and fix existing test-only Clippy warnings.

Public API signatures and JSON field shapes are unchanged. `Default` for the builder is additive. The budget comparison correction is the only intended behavioral change.

## Verification

Toolchain: rustc 1.99.0 (b940084d7 2026-09-28), cargo 1.99.0, cargo-llvm-cov 0.9.1, Linux x86_64.

Passed:

```sh
cargo test --locked
cargo test --locked --all-targets
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
git diff --check
```

45 tests pass: 23 module tests and 22 contract tests. The configuration test additionally checks nine isolated environment configurations. Bench executables also run under `--all-targets`.

Independent compatibility check: the 20 new contract tests unrelated to the approved budget correction also pass against an isolated baseline checkout; the two budget regression tests fail on the old behavior and pass on the corrected behavior.

### Coverage

Commands, run in separate baseline and refactored checkouts / coverage target directories:

```sh
# Baseline: skip the old live-API test (requires real credentials).
cargo llvm-cov --locked --lcov --output-path baseline.lcov -- --skip client::client_tests::test_build_client
# Refactored checkout: no skipped tests.
cargo llvm-cov --locked --lcov --output-path current.lcov
python3 scripts/production_coverage.py current.lcov
```

Production-only instrumented line coverage:

- Baseline: 44 / 145 = **30.34%**
- Refactored: 130 / 130 = **100.00%**

The script counts LCOV `DA` records in `src/*.rs` before each `#[cfg(test)]` module. This avoids inflating coverage by counting executed test implementation lines. It is line coverage, not branch coverage or proof that every possible failure is handled. Generated/uninstrumented code has no line denominator. Different formatting and simpler implementation change the denominator.

For completeness, unfiltered cargo-llvm-cov source-file line coverage (including inline tests) is 68.75% -> 99.37%. The baseline ran 10 tests and skipped its one live API test; refactored coverage ran all 45 tests.

### Performance

```sh
cargo bench --locked --bench builder
cargo bench --locked --bench allocations
```

The same benchmark source was added to the baseline solely for measurement. Both versions used the same locked dependencies, toolchain and release profile. To reuse compiled dependencies safely, the package itself was explicitly cleaned with `cargo clean --release -p typesafe-jev-sdk` before switching source trees; compilation of each source tree was confirmed, and the resulting executables were saved separately. Coverage target directories were separate. Three alternating baseline/refactor timing runs were taken after compilation; each run reports a median of 9 x 200,000 builds. Raw measurements are in `measurements/benchmarks.txt` in the delivery archive.

Median of the three reported medians:

| Workload | Baseline | Refactored | Observed reduction |
| --- | ---: | ---: | ---: |
| Default-model request build | 154.9 ns | 114.2 ns | 26.3% |
| Custom-model request build | 170.2 ns | 126.5 ns | 25.7% |

Allocation-count benchmark:

| Workload | Baseline | Refactored |
| --- | ---: | ---: |
| Default-model request build | 5 | 5 |
| Custom-model request build | 6 | 5 |

The saved default-model allocation is not claimed: only the custom-model path removes an allocation (16.7% fewer allocation calls for this small fixture). Timing and allocation measurements use separate binaries so counting-allocator instrumentation does not distort timing. The counting allocator is confined to the benchmark, never the library.

These are small request-construction microbenchmarks on a shared cloud machine with noticeable timing variation. They do not predict end-to-end API latency or guarantee the same percentage improvement elsewhere. URL reparsing removal was code-reviewed but not separately benchmarked.

## Remaining compatibility limitations

- `TypeSafeClient::new` still panics for invalid environment configuration, matching its existing signature and behavior.
- `system_one` still attempts response deserialization for non-2xx HTTP statuses. Changing this policy was outside the approved refactor.
- `Usage::tokens()` can overflow if the mathematical sum exceeds `u64::MAX`; this existing behavior was deliberately left unchanged.
- Tests exercise local HTTP transport and configuration, not a live HTTPS API or an external proxy service.
- At the initial offline handoff, no remote branch or commit had been published. Publication is a separate, subsequently authorized step; no PR, merge or deployment is included.

## Applying the patch

From a clean checkout at the baseline commit:

```sh
git apply --check refactor.patch
git apply refactor.patch
cargo test --locked
```

The delivery archive includes the complete updated source tree (without build output or `.git`), the patch, this report, and raw measurements. Patch application was checked against a fresh baseline archive, and the applied files were compared byte-for-byte with the delivered source tree.
