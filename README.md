# Trisult

An accumulating alternative to Rust's standard `Result<T, E>`.

While `Result` is designed to short-circuit on the first failure,
many workflows - such as parsers, compilers, and complex data validators -
need to collect and report multiple errors and warnings at once.

**Trisult** provides a robust, context-aware mechanism for batching these diagnostics without sacrificing ergonomics.

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

- **Optional `alloc` Feature**: Enable the `alloc` feature (backed by `smallvec`) to accumulate
  an arbitrary number of diagnostics when heap allocation is available.
