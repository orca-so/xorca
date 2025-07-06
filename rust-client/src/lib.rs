#![allow(unexpected_cfgs)]

#[allow(clippy::all, unused_imports)]
mod generated;
#[cfg(feature = "wasm")]
mod math;
mod pda;

pub use generated::accounts::*;
pub use generated::errors::*;
pub use generated::instructions::*;
pub use generated::programs::XORCA_STAKING_PROGRAM_ID as ID;
pub use generated::programs::*;
#[cfg(feature = "fetch")]
pub use generated::shared::*;
pub use generated::types::*;

#[cfg(feature = "fetch")]
pub(crate) use generated::*;

pub use pda::*;

#[cfg(feature = "wasm")]
pub use math::*;
