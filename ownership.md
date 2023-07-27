# Ownership in Rust

## Intro

The 2 most characteristic features of Rust's compiler are its Move semantics and borrow checker, which together constitute a model of ownership guaranteeing memory safety. In practice this means a few things:

1. whatever you do with a pointer in Rust, it will never be null or point to an incorrect memory location.
   > This is completely irrelevant to anyone programming in a garbage-collected language, and so completely irrelevant to us Web devs.
   >
2. a single variable cannot be mutated from several places at once

The second item is extremely helpful, because it means the following code can never occur:

```javascript
const array = [0]

for (const el of array) {
  addElToArray(el, array)
}

function addElToArray(el: number, array: number[]) {
  array.push(el) // INFINITE LOOP: adds an element on every iteration
  return array
}
```

But more importantly for daily use, it also makes the following code impossible:

```javascript
const x = [0, 1, 2]
const y = x

console.log(x) // error: use of moved value!
```

``` go
var n = 0

var p1 *int = &n
var p2 *int = &n

*p1 = 3
fmt.Println(n) // 3

*p2 = 4
fmt.Println(n) // 4
```

Let's see why this error happens.

## Move semantics

First let's show the previous code in its Rust version:

```rust
let x = vec![0, 1, 2];
let y = x;

println!("{:#?}", x); // error
```

What happens is the following:

* `x` is initialized with a vector (an array in JS)
* `x` is then **moved** into `y`. This means `y` has the same value `x` had originally (as expected), whereas `x` has been **dropped**.

Dropping a value means it is back to being uninitialized. And, whereas JS would simply say "undefined", the Rust compiler doesn't allow operations on unitialized variables. The full error reads as follows:

```rust
error[E0382]: borrow of moved value: `x`
  --> main.rs:14:23
  |
1 |     let x = vec![0, 1, 2];
  |         - move occurs because `x` has type `Vec<i32>`, which does not implement the `Copy` trait
2 |     let y = x;
  |             - value moved here
3 |
4 |     println!("{:#?}", x);
  |                       ^ value borrowed here after move
```

This is probably the error that pops up the most often when starting with Rust, so let's pay attention.

* the reason `x` was moved is because it "does not implement the `Copy` trait"

The `Copy` trait is a property of only those items that take less time/memory to copy than to build a new reference for. Considering that a reference (i.e. pointer) is length 64 bits on (most[^1]) 64 bits systems, all we're left with are integers[^2], single characters, booleans and [tuples made out of these](https://doc.rust-lang.org/std/primitive.tuple.html#trait-implementations-1). For any of these types, you would not get the error above, or any error really to a move. This is because these values are simply created anew wherever they are used: copied when assigned to a new variable, copied when passed to a function, etc.

```rust
let a = 1;
let b = a; // copy only, no move necessary
println!("{}", a); // OKlet tp = (1, true, 'a'); // all elements of the tuple implement Copy
let tp2 = tp; // tuple gets copied
println!("{:?}", tp); // OK
```

Other types, like vectors, are never copied because that could be extremely expensive. Instead, only the reference itself gets copied, along with some extra details. For instance, assigning a vector to a new variable copies pointer, length, and capacity (images from [Programming Rust, 2nd ed.](https://www.oreilly.com/library/view/programming-rust-2nd/9781492052586/)). The element themselves do not go anywhere.

![Memory representation of a vector in Rust](https://learning.oreilly.com/api/v2/epubs/urn:orm:book:9784873119786/files/images/pr2e_0407.png)
![Re-assignment of a vector in Rust](https://learning.oreilly.com/api/v2/epubs/urn:orm:book:9784873119786/files/images/pr2e_0410.png)

So that's it for what happens during a move. Next is borrowing.

## Borrowing

Borrowing means taking a reference to, as opposed to moving, an value.

If you look back at the error we saw above:

```rust
error[E0382]: borrow of moved value: `x`
  --> main.rs:14:23
  |
1 |     let x = vec![0, 1, 2];
  |         - move occurs because `x` has type `Vec<i32>`,
              which does not implement the `Copy` trait
2 |     let y = x;
  |             - value moved here
3 |
4 |     println!("{:#?}", x);
  |                       ^ value borrowed here after move
```

You can see the error occurred when `println!` tried to **borrow** `x`. In other words, it didn't try to move `x`, it just tried to get a reference to it. This is usually a good strategy to avoid move errors.

Consider the following code:

```rust
fn main() {
  let x = vec![0, 1, 2];

  println!("{:#?}", x);

  let y = x;
}
```

It will not return an error. The following code, on the other hand:

```rust
fn foo(v: Vec<i32>) { }

fn main() {
  let x = vec![0, 1, 2];

  foo(x);

  let y = x; // error[E0382]: use of moved value: `x`!
}
```

This of course makes sense: we're trying to move x in two places, but it became unitialized the first time we moved it into `foo`. To get `foo` to behave like `println!`, we need to change it so it only borrows `x`:

```rust
fn foo(v: &Vec<i32>) { } // added `&` before Vec<i32>

fn main() {
  let x = vec![0, 1, 2];

  foo(x);

  let y = x;
}
```

### `&`

`&` is our borrow operator, in this case telling us we can create as many **immutable** references to the `v` parameter as we want.
But we still get an error here: `error[E0308]: mismatched types`. This is simply because we're still passing `x` as a normal vector, whereas foo now only takes a reference to a vector. We just need to pass `&x` for the code to run.

```rust
fn foo(v: &Vec<i32>) { }

fn main() {
  let x = vec![0, 1, 2];

  foo(&x);

  let y = x;
}
```

Finally, on the question of why we passed `x` itself to `println!` even though it talks about borrowing: like `format!`, `writeln!` and some other macros, `println!` bends the rules by implicitly taking a reference to its arguments. This can be done with any macro, but is impossible to do with a normal function.

### `&mut`

Another type of reference can be achieved with `&mut` which returns a **mutable** reference. Opposite to `&` references, there can only be a single `&mut` reference to a given variable. This is of course to prevent a variable getting accessed from all over the place all at once.

``` rust
error[E0499]: cannot borrow `x` as mutable more than once at a time
  --> main.rs:14:15
   |
14 |   foo(&mut x, &mut x);
   |   --- ------  ^^^^^^ second mutable borrow occurs here
   |   |   |
   |   |   first mutable borrow occurs here
   |   first borrow later used by call
```

### Borrow checker

With `&` and `&mut` we can define what the **borrow checker** is:

* the process checking whether a variable's ownership was moved or borrowed
* the process checking that we are not borrowing an unitialized variable
* the process checking that there is at most one mutable reference to any entity

### `ref` pattern

`ref` is almost equivalent to `&`, except it is only used on the left side of variable declarations:

```rust
// ref_c1 and ref_c2 are equivalent
let ref ref_c1 = c;
let ref_c2 = &c;

// same for ref_c3 and ref_c4
let ref mut ref_c3 = c;
let ref_c4 = &mut c;
```

The [ref pattern](https://doc.rust-lang.org/rust-by-example/scope/borrow/ref.html): used when you want to destructure a field/member as a reference.

```rust
// `ref_to_x` is a reference to the `x` field of `point`.
let Point { x: ref ref_to_x, y: _ } = point;

// Destructure `mutable_tuple` to change the value of `last`.
let (_, ref mut last) = mutable_tuple;
```

### Common places for move errors

1. Passing a single variable to 2+ functions

   ```rust
   fn main() {
     let v = vec![0];
     foo(v);
     bar(v); // error[E0382]: use of moved value: `v`
   }
   ```
2. Trying to assign a variable with a vector element not implementing `Copy`:

   ```rust
   fn main() {
     let v = vec![0.to_string()];
     let x = v[0]; // error[E0507]: cannot move out of index of `Vec<String>`
   }
   ```

   The vector itself wasn't moved in `x`, so you might think the vector could just remember that the element is now unitialized. But the plumbery required to monitor the individual state of every element is a bit much so it isn't part of the language.
3. Looping over a vector, and trying to pass the vector to a function inside the loop:

   ```rust
   fn main() {
      let v = vec![0];
      for _ in v { // for-loop takes ownership of `v`
        foo(v); // error[E0382]: use of moved value: `v`
      }
   }
   ```

### Solving the errors in the previous section by borrowing

1. Passing a single variable to 2+ functions

   ```rust
   fn main() {
     let v = vec![0];
     foo(&v);
     bar(v); // OK
   }
   ```
2. Trying to assign a variable with a vector element not implementing `Copy`:

   ```rust
   fn main() {
     let v = vec![0.to_string()];
     let x = &v[0]; // OK
   }
   ```

   Solution with `std::mem::replace`

   ```rust
    fn main() {
      let mut v = vec![0.to_string()]; // declare v as mutable
      let x = std::mem::replace(&mut v[0], substitute_str); // OK, replace value immediately
    }
   ```
3. Looping over a vector, and trying to pass the vector to a function inside the loop:

   First incomplete solution

   ```rust
   fn main() {
     let v = vec![0];
     for _ in &v { // `v` is unitialized on next loop
       foo(v); // `foo` takes ownership during first pass
     }
   }
   ```

   Second incomplete solution

   ```rust
   fn main() {
     let v = vec![0];
     for _ in v { // for-loop takes ownership of `v`
       foo(&v); // referencing an unitialized variable
     }
   }
   ```

   Solution

   ```rust
   fn main() {
     let v = vec![0];
     for _ in &v {
       foo(&v);
     }
   }
   ```

### `mem::replace`, `mem::take` and `Option::take`

If we want a wait to take ownership of vector elements, we need a way to take the value without leaving the vector slot empty. This is exactly what `mem::replace` and `mem::take` allow us to do. Note that `mem::take` replaces the element by its default value, which means the element should implement a default using the `Default` trait.

For vectors of Options, we can also use `Option::take` to automatically do the work for us:

```rust
let mut v = vec![Some("john".to_string()), None, Some("apple".to_string())];

// mem::replace
let q = std::mem::replace(&mut v[0], None);

// mem::take, implicitly replaces with None
let q = std::mem::take(&mut v[0]);

// `take` automatically does the same thing as `replace` above
let q = v[0].take();
```

## Clone

This is the least recommended way, but likely the simplest way, to solve move issues: just `.clone()` the value (for collections implementing the `Clone` trait)! This is analogous to what happens in C++ when re-assigning a vector, which creates a full copy of the pointer and data. In rust we have:

```rust
let v = vec![0];
let x = v.clone(); // clone, not the original `v`
let y = v; // OK
```

The issue with `.clone()` is obviously one of performance and memory use. It's also one of pride, since you can't call yourself a Rust programmer until you've learned to use `.clone()` only in the few cases where it is actually necessary.

Cloning will often become tempting in cases where we want to move some value that only exists as a reference in the current scope. Typically:

```rust
// simply trying to change Vec<String> to Vec<Foo>
// Foo takes a String as argument
fn str_to_foo(v: &Vec<String>) -> Vec<Foo> {
    v.iter()
      // error: cannot move out of `*s` which is behind a shared reference
      .map(|s: &String| Foo(*s))
      .collect()
}
```

This error occurs because we didn't move `v` into the function (we assume for good reasons), but we still want to change the type of its contents, which requires a move.
Cloning makes an easy fix:

```rust
fn str_to_foo(v: &Vec<String>) -> Vec<Foo> {
    v.iter()
      // cloning also converts the reference to a normal value
      .map(|s| Foo(s.clone()))
      .collect()
}
```

In this case, there is no easy way to avoid cloning, except moving `v` instead of taking a reference. If this doesn't work with the design of your application, chances are the design is wrong.

The `debs` project has a fairly good example of a situation where cloning really looked like the better solution:

```rust
// package_json.rs

fn combine_deps_name_version(
    deps_info: &HashMap<PkgName, PackageLockDepInfo>,
    deps_list: Vec<PkgName>,
) -> Vec<PkgNameAndVersion> {
    deps_list
        .into_iter()
        .filter_map(|pkg_name| {
            deps_info
                .get(pkg_name)
                // clone a single field
                .and_then(|info| info.version.clone())
                .map(...)
        })
        .collect()
}
```

This looked necessary because of how the function `combine_deps_name_version` gets invoked twice over the same `deps_info` HashMap (once for production dependencies, and once for development dependencies):

```rust
// package_json.rs get_deps_version
combine_deps_name_version(&pkgs_info.packages, deps_lists.0, prefix);
combine_deps_name_version(&pkgs_info.packages, deps_lists.1, prefix);
```

The only alternative would be cloning `deps_info` in the first call, but that would be way more expensive than cloning just a single `info.version` field for only a few entries in the `deps_info`.

Out of habit I only use immutable variables (defined with `let` in Rust), but here the solution is actually to use mutability.
Recall that we mentioned `mem::replace` and `mem::take` as ways to take ownership of some element of a collection. Also recall their use: `std::mem::take(&mut el, substitute_value)` and `std::mem::take(&mut el)`. We need to take a `&mut` (mutable reference) to the element, which means the element has to come from a collection that allows mutation.
Let's make the necessary changes:

```rust
// package_json.rs

fn combine_deps_name_version(
    // borrow as mutable
    deps_info: &mut HashMap<PkgName, PackageLockDepInfo>,
    deps_list: Vec<PkgName>,
) -> Vec<PkgNameAndVersion> {
    deps_list
        .into_iter()
        .filter_map(|pkg_name| {
            deps_info
                // get a mutable reference to the HashMap entry
                .get_mut(pkg_name)
                // take and replace with the default empty String
                .and_then(|info| std::mem::take(info.version))
                .map(...)
        })
        .collect()
}
```

And that's it, it works. Since `info.version` is a String, `mem::take` replaces it with an empty `String` object. This is clearly, though marginally, less expensive than cloning, so it's a win in terms of performance.
On the other hand we did have to pay by making our collection mutable. In our case it is a very local change, but that are probably limits to how many mutable collections you want to tamper with.

### Areas where you **definitely** don't need `.clone()`

1. Using `iter` where you could use `into_iter`

  Both `iter` and `into_iter` return an iterator over a collection (e.g. vector) which can be used to `reduce`, `filter_map` and `flat_map_filter_post_prod_reduce` (the `post_prod` part may be fake).
  The difference between them:

* `iter_into` returns an iterator over the actual data
* `iter` returns an iterator over references (`&T`)

  In practice (example from [an old blog post](https://web.archive.org/web/20210120233744/https://xion.io/post/code/rust-borrowchk-tricks.html)):

```rust
  // error: cannot move out of borrowed content [E0507]
  Ok(results.iter().map(|r| r.ok().unwrap()).collect())

  // naive fix with .clone()
  Ok(results.iter().map(|r| r.clone().ok().unwrap()).collect())

  // actual fix
  Ok(results.into_iter().map(|r| r.ok().unwrap()).collect())
```

  NOTE: this doesn't mean you should never use `iter`. In fact, `iter` is the only way to iterate over a *reference* to a collection.

## Extra: "Move" and Affine/Linear type systems

1. Affine type systems: Every variable is used at most once.
2. Linear type systems: Every variable is used exactly once.

Rust's ownership/move semantics correspond to an affine type system, where a variable cannot be used twice.
A step further would be a linear type system, that forces all variables to also be used exactly once. Technically, this could be achieved with a linter throwing an error for unused variables. But this wouldn't allow Rust's compiler to optimize accordingly.
On this subject, Tweag (the main contributor to the Glasgow Haskell Compiler) has been working on [implementing linear types in Haskell](https://www.tweag.io/blog/2017-03-13-linear-types/) since 2017.

A linear Rust is discussed in [this post](https://faultlore.com/blah/linear-rust/). It incidentally constitutes a great introduction to the subject of "substructural types systems", which affine and linear type systems are examples of.

## Extra #2: Borrow checker, lifetimes and regions

Rust's borrow checker, in particular the part checking that we are borrowing a valid resource, corresponds to the concept of [regions, region-based memory management](https://web.eecs.umich.edu/~weimerw/2006-655/lectures/weimer-655-24.pdf).
In region-based memory management systems, all variables and pointers are assigned to exactly one region, which tells the compiler at which point all its members can be safely deallocated.

Another important aspect of Rust that corresponds to regions is [lifetimes](https://doc.rust-lang.org/rust-by-example/scope/lifetime.html).

[^1]: According to [Write Great Code Vol.1](https://nostarch.com/writegreatcode1_2e),  "64 bits" or "32 bits" refers to whichever is larger from the following: the largest register in the CPU, or the width of the data bus (the cable carrying data between the CPU and main memory). A pointer cannot be bigger than the largest register in the CPU (unverified assumption) so in the case where a system would be called "64 bits" because of its data bus, you could still expect pointers to be maximum 32 bits in length. This is, however, extremely unlikely to occur in modern systems, since a 32-bit pointer can only cover a 2^32 address space, which limits memory to 4GiB - less than modern RAM.

[^2]: Even 128-bit integers, if you store [one half in one register and the rest in another](https://godbolt.org/#g:!((g:!((g:!((h:codeEditor,i:(j:1,lang:c%2B%2B,source:%27__int128+f(__int128+x,+__int128+y)%0A%7B%0A++++return+x+%2B+y%3B%0A%7D%0A%27),l:%275%27,n:%270%27,o:%27C%2B%2B+source+%231%27,t:%270%27)),k:48.82709400697888,l:%274%27,m:100,n:%270%27,o:%27%27,s:0,t:%270%27),(g:!((h:compiler,i:(compiler:gsnapshot,filters:(b:%270%27,binary:%271%27,commentOnly:%270%27,demangle:%270%27,directives:%270%27,execute:%271%27,intel:%270%27,trim:%271%27),lang:c%2B%2B,libs:!(),options:%27-O3+-march%3Dnative%27,source:1),l:%275%27,n:%270%27,o:%27x86-64+gcc+(trunk)+(Editor+%231,+Compiler+%233)+C%2B%2B%27,t:%270%27)),k:51.17290599302112,l:%274%27,m:100,n:%270%27,o:%27%27,s:0,t:%270%27)),l:%272%27,n:%270%27,o:%27%27,t:%270%27)),version:4).
