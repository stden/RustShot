[← Back to Index](../index.md)

# Rust Development Practices

Standards and patterns used in RustShot to ensure performance, maintainability, and reliability.

## Coding Standards
* **Module Structure**: Separate business logic into modules (`capture.rs`, `record.rs`, etc.) and expose them via `lib.rs`.
* **Dependency Minimization**: Avoid non-essential crates. Implement simple functionality (like JSON formatting) internally when feasible.
* **Error Handling**: Standardized on `Result<T, String>` with informative error messages.

## Performance Guidelines
* **Zero-Copy**: Favor borrowing (`&[T]`) over cloning (`Vec<T>`).
* **Pre-allocation**: Always use `with_capacity()` when the final size is known or estimable.
* **SIMD & Optimization**: Rely on high-level iterator methods (`chunks_exact_mut`) which allow the compiler to auto-vectorize.
* **Integer Focus**: Avoid expensive float operations in tight loops (e.g., use distance squared).

## Testing & Quality
* **Naming**: `module_function_scenario` (e.g., `capture_resolve_index_success`).
* **Regression Testing**: Fixes must include a test case that would have caught the bug.
* **CI/CD**:
    - **Linting**: Strict clippy checks (`-D warnings`).
    - **Formatting**: Automated check on every PR.
    - **Multi-OS**: Testing on both Windows and Linux.

## Benchmarking
* Use **Criterion** for micro-benchmarks.
* Always measure baseline before and after any "optimization" commit.

## See Also
* [Source: Rust Best Practices](source-rust-practices.md)
* [Source: GitHub Actions CI/CD](source-github-pipeline.md)
