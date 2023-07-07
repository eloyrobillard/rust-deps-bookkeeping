# Development Guide

Rust's creator thoughts, for context: [https://graydon2.dreamwidth.org/307291.html](https://graydon2.dreamwidth.org/307291.html)

Rust Design Patterns:  [https://rust-unofficial.github.io/patterns](https://rust-unofficial.github.io/patterns)

## Setup

You will first need to [install Rust](https://www.rust-lang.org/tools/install).

`cargo run` to run the cli.

`cargo test` to run all tests in the project.

If you are using VS Code, I recommend installing the `rust-analyzer` for in-code error messages. The rest of this document will assume it is installed.

## Basics

Ownership, `Move` and borrowing are all explained in [ownership.md](./ownership.md).

### `type`, `struct` and tuple structs

The `type` keyword creates a type alias, similar to `type` in TypeScript. When checking the types produced by `rust-analyzer`, the real type will show up instead of the alias.

``` rust
type PkgName = String;
```

On the other hand, `struct` and tuple structs declare a new type that will show up as such in the `rust-analyzer` tooltips.

``` rust
// struct
struct Foo {
    date: DateTime<FixedOffset>,
    age: u32,
}

// fields can be declared just like in TypeScript
// either with `:` or directly if a local variable name
// matches with the field's name
let foo = Foo {
    date,
    age: 5,
}

// tuple struct
struct PkgNameAndVersion(pub PkgName, pub Version);

let pkg_name_and_version = PkgNameAndVersion(pkg_name, version);
```

### Packages and Crates, Navigating the Module Hierarchy

Crates come in two types: library and binary crates.
A rust package can contain several crates:

* 1 library crate, with `lib.rs` as entry point
* 1 binary crate, with `main.rs` as entry point
* 1+ binary crates inside `src/bin`. The entry point can be any `foo.rs`, where foo is the name of the binary to be compiled.

E.g. package:

``` bash
src
 |-- bin
      |-- foo.rs # defines a binary called `foo`
 |-- main.rs # defines a binary named after the package
 |-- lib.rs # defines a library crate
 ...
```

Generally speaking, packages containing binary crates will also contain a library crate containing code used by several of the binary crates. Even for single-binary projects, the [Rust Book](https://doc.rust-lang.org/stable/book/ch12-03-improving-error-handling-and-modularity.html) recommends putting most of the code on the library side.

#### Module Hierarchy

Inside a crate, except for the entry point, creating a new file is the same as creating a submodule that can be declared inside the parent module with:

``` rust
// main.rs/lib.rs/parent module
mod foo;
```

⚠️ Note that a submodule must be declared inside its parent module to be used in the project.

``` rust
mod foo;

use foo::do_stuff;
use bar::do_stuff; // error, undeclared module
```

There are two equivalent ways to declare a submodule:

* `foo.rs`
* `foo/mod.rs`

The second option makes it clear from a file hierarchy perspective why declaring some `foo.rs` next to `main.rs` or `lib.rs` automatically makes it a submodule of these two, with `mod.rs` as entry point.

Another example is defining a `tests` module inside a file:

``` rust
// foo.rs
...

mod tests {
...
}
```

In this case, the hierarchy becomes `foo::tests`, which in file hierarchy translates to `foo/tests.rs` or `foo/tests/mod.rs`.

``` bash
src
 |-- lib.rs
 |-- foo.rs (with tests module inside)
 or
 |-- foo
      |-- tests.rs
 or
 |-- foo
      |-- tests
           |-- mod.rs
```

### Unit Testing

Unit testing in Rust can be achieved by defining a `tests` module preceded by a `#[cfg(tests)]` declaration.

``` rust
// source code
...

#[cfg(test)]
mod tests {
    // import all from the parent module
    use super::*;

    // define a test
    #[test]
    fn test_name() {
        assert!(1 == 1);
        assert_eq!(1, 1);
        assert_neq!(0, 1);
    }
}

```

Async tests are defined in the same way, except they are declared with the `#[tokio::test]` macro. The test runtime is single-threaded by default and can be modified to `#[tokio::test(flavor = "multi_thread", worker_threads = 1)]` for multi-threading (the number of threads defaults to the number of cpus on the system).

#### Running tests

Use `cargo test` to run all tests.
To run a specific (set of) test(s), add its module path: `cargo test test_name` will only run `test_name`. If `test_name` is defined in several tests modules, you can use `cargo test foo::tests::test_name`.
More generally, `cargo test` also matches partial names, so `cargo test test_name` would also run `test_name_foo_bar` if it exists.

Options can also be passed to the run command. To signal options you first need to pass `--` (`cargo test -- --option`). Some useful options are:

1. `--nocapture`:

    Show the output for successful tests

2. `--test-threads=1`:

    This prevents tests from being run concurrently, useful if several FIO tests write to the same `output.txt` file.

#### Where to define your unit tests

1. in the same file as the code being tested

    This is the easiest solution and is practiced in official packages like [rustup](https://github.com/rust-lang/rustup/blob/master/src/config.rs) even in relatively large files (1000+ lines in the example above).

2. in the same folder as the file being tested (usually `src/`), inside `<file being tested>/tests.rs`

    This solution is used in the [standard library](https://github.com/rust-lang/rust/tree/master/library/std/src).

    ``` bash
    src
     |-- foo.rs
     |-- foo
          |-- tests.rs
    ```

    ``` rust
    // foo.rs
    // import tests module and announce to the test runtime
    #[cfg(test)]
    mod tests;

    // tests.rs
    use super::*;

    #[test]
    fn test_name() {
        ...
    }
    ```

    NOTE: Naming a folder after a source file allows its contents to be detected as submodules of that file. Additionally, `foo.rs` is equivalent to `foo/mod.rs`: `mod.rs` is automatically detected as the entry point to the `foo` module.
    See the `deprecated` module for an example.

[coercion](https://doc.rust-lang.org/reference/type-coercions.html)

#### Testing stdout

To test the output of a file writing to stdout, change the file to get `mut writer: impl std::io:Write` as a parameter. For stdout you will need to pass `&mut std::io::stdout()`. In your tests, you can instead pass a vector to collect the output:

``` rust
let mut bytes: Vec<u8> = Vec::new();

foo(&mut bytes);

// your output is currently a vector of utf8 bytes
// you need to translate that to a String
let output = String::from_utf8(bytes).unwrap();

assert_eq!(output, ...);
```

#### Testing a CLI

[Reference](https://rust-cli.github.io/book/tutorial/testing.html)

The main tool you will need to test your CLI is the function `Command::cargo_bin` which is available from the package `assert_cmd::prelude` (NOT from `std::process::Command`). This function will allow you to run your CLI binary as part of the test.

Here's an example from [rustlings](https://github.com/rust-lang/rustlings/blob/main/tests/integration_tests.rs) (small exercises to learn Rust basics):

``` rust
 #[test]
fn run_single_compile_success() {
    Command::cargo_bin("rustlings")
        .unwrap()
        .args(&["run", "compSuccess"])
        .current_dir("tests/fixture/success/")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Usage: debs <COMMAND>"));
}
```

* `args` takes a reference to an array of arguments
* `arg` is an alternative to `args` when passing one argument at a time
  * you might think you could always `arg("run compSuccess")` and that would count as 2 arguments but not no, only "run" would be read in this case
* `stderr` (and `stdout`) allow you to test the output. The easiest way to do so is by using `predicate` from `predicates::prelude`.
  * `predicate::str::contains` returns a struct implementing the `eval` method, which `stderr` will pass the error output to

## Shipping a Rust binary as an npm package

The process for turning a Rust binary into an npm package is very easy:

1. output binaries for all targeted platforms
   1. add the target: `rustup target add <target>`
   2. build the binary: `cargo build --release --target <target>`
        MacOS targets are: x86_64-apple-darwin and aarch64-apple-darwin
   3. all these targets will go to `target/<target>`

2. create a `package.json` going more or less like:

    ``` json
    {
        "name": "debs",
        "bin": "bin/debs",
        "scripts": {
            "preinstall": "node npm/preinstall.js"
        },
        ...
    }
    ```

3. create the `npm/preinstall.js` script which uses `process` to detect the correct binary name for your architecture and OS, before copying it to the binary path specified in `package.json` (`bin/debs` in our case)
4. `npx debs` will now launch your binary

## Extras

### `Lazy`, and global variables in Rust

Using `once_cell::sync::Lazy` allows initializing a global (static) variable with a dynamic value, which requires memory allocation like `Vec`.

#### Why We Need Lazy Initialization

`static` means the size of the variable must be known at compile time. This is because static memory objects are bundled together with the binary.

Memory allocation, on the other hand, is done by interacting with the heap (dynamic memory storeprovided by the OS) and can only be done at runtime.

In this case what we then want is just to pass a reference to the dynamic value, since the size of a pointer is fixed. The issue here is memory addresses cannot be resolved at compile time, since the OS hasn't allocated the base address of the program, from which all others can be derived.

This is why we use `Lazy`, so the "static" pointer is resolved at runtime, once we are sure the OS has given us an address.

### Naming convention: as_, to_, into_

[Naming (and more) conventions in Rust](https://rust-lang.github.io/api-guidelines/naming.html)

| Prefix | Cost      | Ownership                                                                                    |
| ------ | --------- | -------------------------------------------------------------------------------------------- |
| as_    | Free      | borrowed -> borrowed                                                                         |
| to_    | Expensive | borrowed -> borrowed <br> borrowed -> owned (non-Copy types) <br>owned -> owned (Copy types) |

### Temporary mutability

[Reference](https://rust-unofficial.github.io/patterns/idioms/temporary-mutability.html)

This pattern allows variable initialization using mutation (`.sort()` below) while keeping the variable immutable.

```rust
let data = {
    let mut data = get_vec();
    data.sort();
    data
};
```

### Use borrowed types for arguments

[Reference](https://rust-unofficial.github.io/patterns/idioms/coercion-arguments.html)

Owned types vs borrowed types: `&Vec<T> → &[T]`、`&String → &str`、`&Box[T] → &T`, etc.

Essentially, the compiler can automatically coerce down a type like `&String` to the type of its contained data (`String` is a wrapper for `&str` with dynamic memory allocation on top).
Furthermore, all the usual functions (`into_iter()`, `len()`) are really implemented for the borrowed type, i.e. `&str`, not for the owned type, i.e. `String`. So you don't lose any functionality.

With that in mind, some reasons you should prefer passing the borrowed type as an argument are:

1. It adds flexibility: you can now pass either `String` or `&str`, either `Vec<T>` or `&[T]` to your function.
2. It removes extra indirection: passing `&String` is a little bit like passing `&&str` where `&str` will do fine.

I used this in the `old` function for example.
