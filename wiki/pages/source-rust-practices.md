[← Back to Index](../index.md)

# Source: Rust Best Practices

* **Type**: Technical Documentation / Tutorial
* **Original Title**: Лучшие практики Rust: код и производительность
* **File**: `docs/rust-best-practices.md` in the central `iopm` repo (moved out of RustShot as general, non-project-specific guidance)
* **Topic**: Rust development, Performance, Testing, Project structure.

## Key Takeaways
1. **Project Structure**: Use `lib.rs` to expose modules for integration tests and benchmarks. Keep `main.rs` minimal (CLI only).
2. **Dependency Management**: Prefer writing small logic manually (e.g., manual JSON formatting in `settings.rs`) over adding large dependencies like `serde`.
3. **Performance**:
    - Avoid cloning in hot paths; use borrowing (`&[u8]`).
    - Use `with_capacity()` for `String` and `Vec`.
    - Prefer integer arithmetic over floating point (e.g., compare squared distances).
    - **Trust the Compiler**: Standard `chunks_exact_mut` is often faster than manual `unsafe` pointers due to SIMD auto-vectorization.
4. **Error Handling**: Use `Result<T, String>` for simplicity in small products. Errors should contain expected/actual values and context.
5. **Unsafe Rules**: Only use `unsafe` for FFI or if benchmarks prove a significant gain. Always document safety invariants.
6. **Testing**: Focus on boundary cases, error messages, and regression tests. Use the `module_function_scenario` naming convention.
7. **Benchmarks**: Use `Criterion`. Establish a baseline before optimizing and verify gains with `cargo bench`.

## Linked Concepts
* [Rust Development Practices](rust-development-practices.md)
* [Architecture Overview](architecture-overview.md)
