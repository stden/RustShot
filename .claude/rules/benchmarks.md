---
paths:
  - "benches/**"
  - "scripts/benchmark_guard.ps1"
---

# Benchmarking RustShot

<!-- Path-scoped so it costs no context until someone touches a benchmark. -->

**One run is not evidence.** Noise on the development machine reaches ±25%.
Criterion has reported "Performance has improved" for byte-identical code, and has
reported a 25% regression for a change that was actually neutral. Before believing
any delta:

- take min-of-N (N ≥ 3), or measure both variants in the same process;
- re-run the *unchanged* code against its own baseline first, to see the noise floor;
- distrust a single criterion verdict entirely — it compares against a stored
  baseline measured at a different time, so thermal drift reads as a real change.

Quote a number only if it is dated, attributed to a machine, and labelled as one
run — or measured properly. A fresh unverified figure is no better than the stale
one it replaces.

## Two benchmark targets, different jobs

- `benches/performance_gate.rs` — deterministic, CI-gated on pull requests, owns the
  recorded baselines. Judge changes with this one.
- `benches/benchmarks/` — exploratory, never run by CI, and it probes live screen
  capture, so it can stall on an idle or busy session.

## Benchmarks and budgets move together

`tests/bench_functions/benchmark_helpers.rs` asserts that the ids declared by
`performance_gate.rs` and the ids budgeted in `scripts/benchmark_guard.ps1` are the
**same set**. Add or remove a benchmark without its budget and the suite goes red —
that is deliberate: a benchmark with no budget is measured on every pull request and
enforces nothing, and a budget with no benchmark aborts the job twenty minutes in.

Those cross-checks also count raw occurrences of `bench_function(` and
`Benchmark =`, so a comment containing either substring will fail them.

The guard compares the **upper** bound of the median confidence interval against an
absolute ceiling, and the **lower** bound of the median change against the relative
threshold. Existing stable budgets sit at roughly 2.75–4x the measured upper bound;
match that, and leave headroom for a slower CI runner.
