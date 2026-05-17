# Trisult &emsp; [![Version]][crates.io] [![License]][crates.io] [![Docs]][crates.io]

[Version]: https://img.shields.io/crates/v/trisult.svg
[License]: https://img.shields.io/crates/l/trisult.svg
[crates.io]: https://crates.io/crates/trisult
[Docs]: https://docs.rs/trisult/badge.svg
[docs.rs]: https://docs.rs/trisult/latest/trisult

An accumulating alternative to Rust's standard `Result<T, E>`.

While `Result` is designed to short-circuit on the first failure,
many workflows - such as parsers, compilers, and complex data validators -
need to collect and report multiple errors and warnings at once.

**Trisult** provides a robust, context-aware mechanism for batching these diagnostics without sacrificing ergonomics.

1. [Features](#features)
2. [Usage](#usage)
   - [Basic Example](#basic-example)
   - [Capturing Context](#capturing-context)
   - [Auto Stacking Contexts](#auto-stacking-contexts)
   - [Accumulator Selection](#accumulator-selection)
3. [Performance](#performance)
4. [License](#license)

## Features

- **The `Trisult` Enum**: A diagnostic-aware alternative to `Result`.
  It accumulates warnings alongside the value on success (`Trisult::Ok(Diagnosed)`),
  and collects both errors and warnings on failure (`Trisult::Err(Diagnoses)`).

- **Idiomatic Combinators**: Chain operations naturally using familiar methods like `.map()` and `.and_then()`.
  Warnings from previous steps are safely preserved and carried forward.

- **Rich Context Tracking**: Tie diagnostics to precise source locations, AST nodes,
  or custom application states using the `CapturedContext` and `ContextStack` traits.

- **Configurable Accumulation**: Control memory usage and verbosity via `AccumulatorKind`.
  Use `All` to collect everything, or `Most` to keep only the highest-priority diagnostic
  (e.g., preserving an `Error` over a `Warning`).

- **`#[no_std]` by Default**: Works out-of-the-box in resource-constrained environments
  using zero-allocation accumulators.

- **Optional `alloc`**: Disable the `alloc` feature (backed by `smallvec`) to
  disable accumulating an arbitrary number of diagnostics when heap allocation is available.
  (`alloc` is enabled by default)

## Usage

First, add `trisult` to your `Cargo.toml`:

```toml
[dependencies]
trisult = "0.3"
```

### Basic Example

The easiest way to use `Trisult` is with the `#[trisult]` macro,
which allows you to seamlessly accumulate `warn!` and `error!` diagnostics.

The macro transforms a function returning an `Option<T>` internally into one that returns a `Trisult`.

```rust
use trisult::{trisult, Trisult, Diagnosed, NoLoc};

// Define your own warning and error types
#[derive(Debug)]
pub enum MyWarn { Deprecated, Unconventional }

#[derive(Debug)]
pub enum MyErr { MissingField, InvalidFormat }

// A type alias for convenience
pub type MyResult<T> = Trisult<T, MyWarn, MyErr, NoLoc>;

#[trisult]
fn parse_version(version: &str) -> MyResult<i32> {
    match version {
        "v2" => Some(2),
        "v1" => {
            // Emits a non-fatal warning
            warn!(MyWarn::Deprecated, NoLoc);
            Some(1)
        }
        _ => {
            // Emits a fatal error
            error!(MyErr::InvalidFormat, NoLoc);
            None
        }
    }
}

#[trisult]
fn parse_config(version: &str) -> MyResult<i32> {
    // Use `tri!` to unpack sub-operations while accumulating their diagnostics
    let v = tri!(parse_version(version))?;
    Some(v)
}

fn main() {
    match parse_config("v1") {
        Trisult::Ok(Diagnosed(val, warnings)) => {
            println!("Success: {}", val); // Prints: 1
            for warn in warnings {
                println!("Warning: {:?}", warn.value);
            }
        }
        Trisult::Err(diagnoses) => {
            for diag in diagnoses {
                println!("Failed with: {:?}", diag.value);
            }
        }
    }
}
```

### Capturing Context

Often, it's useful to tie your warnings and errors to specific file locations, line numbers, or context stacks.

This is natively supported by `Trisult` using `CapturedContext`.
You can pass any type implementing `CapturedContext` as the context parameter to `warn!` or `error!`.

```rust
use trisult::{trisult, Trisult, NoLoc};

#[derive(Debug, Clone)]
pub struct Span(usize, usize);

impl std::fmt::Display for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.0, self.1)
    }
}

#[trisult]
fn parse_with_context(input: &str, span: Span) -> Trisult<String, String, String, Span> {
    if input.is_empty() {
        error!("Empty input".to_string(), span);
        return None;
    }

    Some(input.to_string())
}
```

### Auto Stacking Contexts

For deeply nested parsers or workflows,
you might want to maintain a "stack trace" of where a diagnostic occurred (e.g. `/parent_node/child_node/attribute`).
`Trisult` provides an auto-stacking feature if your context implements `ContextStackMut`.

By defining a `segment` in the `#[trisult]` macro and identifying your stack argument with `#[context]`,
the macro will automatically `push` the segment onto the stack before executing the function
and safely `pop` it off upon exiting - even on early returns.

```rust
use trisult::{trisult, Trisult, ContextStack, ContextStackMut};

// A simple stack that joins string segments with '/'
#[derive(Debug, Default, Clone)]
pub struct TraceStack {
    pub path: Vec<&'static str>,
}

impl ContextStack for TraceStack {
    type Captured = String;
}

impl ContextStackMut for TraceStack {
    type Segment = &'static str;

    fn capture(&self) -> Self::Captured {
        if self.path.is_empty() {
            "/".to_string()
        } else {
            format!("/{}", self.path.join("/"))
        }
    }

    fn push(&mut self, segment: Self::Segment) {
        self.path.push(segment);
    }

    fn pop(&mut self) {
        self.path.pop();
    }
}

#[derive(Debug)]
pub enum MyWarn { MinorIssue }

#[derive(Debug)]
pub enum MyErr { FatalIssue }

pub type MyResult<T> = Trisult<T, MyWarn, MyErr, String>;

#[trisult(segment = "child_node")]
fn parse_child(#[context] stack: &mut TraceStack) -> MyResult<()> {
    warn!(MyWarn::MinorIssue); // Captured as "/parent_node/child_node"

    // Early returns safely pop the stack segment
    error!(MyErr::FatalIssue); // Captured as "/parent_node/child_node"
    None
}

#[trisult(segment = "parent_node")]
fn parse_parent(#[context] stack: &mut TraceStack) -> MyResult<()> {
    tri!(parse_child(stack))?;
    Some(())
}

fn main() {
    let mut stack = TraceStack::default();
    let res = parse_parent(&mut stack);

    // The stack is safely popped back to its original state
    assert!(stack.path.is_empty());
}
```

### Accumulator Selection

Sometimes you want the caller to decide how diagnostics are accumulated at runtime -
for example, collecting all warnings during a deep analysis (`trisult::All`),
but failing fast and saving memory during a quick validation pass (`trisult::Most`).

Instead of duplicating your function,
you can inject the accumulator kind dynamically by tagging a generic parameter with `#[kind]`.

The macro will automatically use the caller's generic argument to initialize the internal state.

```rust
use trisult::{custom_trisult, trisult, AccAlloc, Trisult, Diagnosed, NoLoc, All, Most};

// Define your own warning and error types
#[derive(Debug)]
pub enum MyWarn { Deprecated, Unconventional }

#[derive(Debug)]
pub enum MyErr { MissingField, InvalidFormat }

// Helper macro to inject allocator of accumulators into result type
custom_trisult!(MyResult<T>(MyWarn, MyErr));

#[trisult]
fn parse_dynamic<
    #[kind] T: AccAlloc // injected at compile time
>(input: &str) -> MyResult<i32, T> /* also, the allocator type should be injected into the result type */ {
    warn!(MyWarn::Deprecated, NoLoc);
    warn!(MyWarn::Unconventional, NoLoc);

    if input.is_empty() {
        error!(MyErr::MissingField, NoLoc);
        return None;
    }

    Some(42)
}

// It doesn't force you to write allocator type every time.
// If it is not specified, it defaults to default allocator:
#[trisult]
fn something_other() -> MyResult<()> {
    todo!()
}

fn main() {
    // Caller A: Collect everything
    let exhaustive_res = parse_dynamic::<All>("data");
    if let Trisult::Ok(Diagnosed(_, warnings)) = exhaustive_res {
        assert_eq!(warnings.into_iter().count(), 2);
    }

    // Caller B: Only keep the most severe diagnostic
    let fast_res = parse_dynamic::<Most>("data");
    if let Trisult::Ok(Diagnosed(_, warnings)) = fast_res {
        assert_eq!(warnings.into_iter().count(), 1);
    }
}
```

## Performance

- [Benchmark](https://trisult.sharp0802.com/criterion/report)
  - Ran on AMD Ryzen 9 9955HX, Linux 6.18.31-1-lts.
  - See `bench.sh` for more details.

It's strongly recommended to use `Most` accumulator over `All` in performance critical section.

As you can see in benchmark given above,
While `Most` mode causes only near to 0.6ns overhead per single call,
`All` mode can cause almost 1.5ns per single call.

## License

This project is licensed under either of

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or <https://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <https://opensource.org/licenses/MIT>)

at your option.
