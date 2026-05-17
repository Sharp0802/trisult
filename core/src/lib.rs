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
pub use trisult_macros::*;
