#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../README.md"))]
#![no_std]
#![warn(missing_docs)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![warn(clippy::min_ident_chars)]
#![warn(clippy::missing_inline_in_public_items)]
#![warn(clippy::must_use_candidate)]
#![deny(unused_results)]

#[cfg(feature = "alloc")]
const VEC_SIZE: usize = 8;

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
