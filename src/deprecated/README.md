This module showcases how modules are resolved in Rust:

1. `deprecated/mod.rs` is just the same as `deprecated.rs`
2. `deprecated/tests.rs` is a submodule of `deprecated`, and is equivalent to `deprecated/tests/mod.rs`

```bash
deprecated
 |-- mod.rs
 |-- tests
      |-- mod.rs
```
