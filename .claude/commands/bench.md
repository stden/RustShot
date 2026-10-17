Run benchmarks for RustShot.

Argument: $ARGUMENTS (optional filter, e.g. "capture" or "selection")

1. Run the Criterion benchmark target from the project root:
   - No filter: `cargo bench --bench benchmarks -- --noplot`
   - With a filter: `cargo bench --bench benchmarks -- --noplot <filter>`
2. Report the benchmark results clearly, including the benchmark groups/cases that ran and
   the key timings.
3. If Criterion reports a regression or improvement versus an existing baseline, mention it.
4. If benchmarks fail to compile or run, show the error and stop.
