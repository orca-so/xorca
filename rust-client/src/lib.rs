//! # xORCA Rust Client
//!
//! A Rust client library for interacting with the xORCA staking program on Solana.
//!
//! ## Features
//!
//! - **Type-safe interactions** with the xORCA staking program
//! - **WASM support** for use in web applications
//! - **Auto-generated code** from the program IDL using Codama
//! - **PDA utilities** for Program Derived Address derivation
//! - **Math utilities** with WASM compilation support
//! - **Serialization support** with optional serde integration
//!
//! ## Quick Start
//!
//! ```rust
//! use xorca::*;
//!
//! // Get the program ID
//! let program_id = XORCA_STAKING_PROGRAM_ID;
//!
//! // Derive state address PDA
//! let (state_address, _bump) = find_state_address().unwrap();
//! ```
//!
//! ## Features
//!
//! The crate supports several optional features:
//!
//! - `serde` - Enable serde serialization/deserialization
//! - `fetch` - Enable Solana client integration for fetching account data
//! - `floats` - Enable floating-point math operations (default)
//! - `wasm` - Enable WASM compilation for web use
//!
//! ## License
//!
//! This project is licensed under a custom license. See [LICENSE](../LICENSE) for details.

#![allow(unexpected_cfgs)]

pub mod conversion;
#[allow(clippy::all, unused_imports)]
mod generated;
#[cfg(feature = "wasm")]
mod math;
pub mod pda;
#[cfg(feature = "fetch")]
pub mod utils;

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

pub use conversion::*;
pub use pda::*;

#[cfg(feature = "wasm")]
pub use math::*;

#[cfg(feature = "fetch")]
pub use utils::*;

#[cfg(test)]
pub mod test_utils;
