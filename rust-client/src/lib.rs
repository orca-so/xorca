#![allow(unexpected_cfgs)]

#[allow(clippy::all, unused_imports)]
mod generated;

pub use generated::accounts::*;
pub use generated::errors::*;
pub use generated::instructions::*;
pub use generated::programs::XORCA_ID as ID;
pub use generated::programs::*;
#[cfg(feature = "fetch")]
pub use generated::shared::*;
pub use generated::types::*;
