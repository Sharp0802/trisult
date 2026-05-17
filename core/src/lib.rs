#![doc = include_str!("../README.md")]
#![no_std]
#![warn(
    missing_docs,
    clippy::pedantic,
    clippy::nursery,
    clippy::min_ident_chars,
    clippy::missing_inline_in_public_items,
    clippy::must_use_candidate
)]
#![deny(unused_results)]
#![allow(clippy::type_complexity)]

#[cfg(feature = "alloc")]
const VEC_SIZE: usize = 2;

mod acc;
mod context;
mod contextual;
mod diagnosis;
mod iter;
mod traits;
mod trisult;

pub use acc::*;
pub use context::*;
pub use contextual::*;
pub use diagnosis::*;
pub use iter::*;
pub use traits::*;
pub use trisult::*;

/// Transforms a function to automatically accumulate `Trisult` diagnostics.
///
/// This macro provides an ergonomic way to batch diagnostics by generating internal state
/// to collect `warn!` and `error!` emissions. It conceptually transforms a function
/// returning an `Option<T>` into one that returns a `Trisult`.
///
/// See [`README`](crate) for examples.
pub use trisult_macros::trisult;
