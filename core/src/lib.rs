#![no_std]

#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![warn(clippy::min_ident_chars)]
#![warn(clippy::missing_inline_in_public_items)]
#![warn(clippy::must_use_candidate)]
#![deny(unused_results)]

#[cfg(feature = "alloc")]
const VEC_SIZE: usize = 8;

mod trisult;

pub use trisult::*;
pub use trisult_derive::*;
