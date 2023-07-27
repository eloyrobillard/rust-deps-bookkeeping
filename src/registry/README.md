This module showcases how modules are resolved in Rust:

1. `registry/mod.rs` is just the same as `registry.rs`
2. `registry/tests.rs` is a submodule of `registry`, and is equivalent to `registry/tests/mod.rs`:

```bash
registry
 |-- mod.rs
 |-- tests
      |-- mod.rs
```
