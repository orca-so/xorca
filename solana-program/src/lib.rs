pub mod assertions;
pub mod cpi;
pub mod entrypoint;
pub mod error;
pub mod event;
pub mod instructions;
pub mod pda;
pub mod state;
pub mod util;

use pinocchio::pubkey::Pubkey;
use pinocchio_pubkey::{declare_id, pubkey};

declare_id!("StaKE6XNKVVhG8Qu9hDJBqCW3eRe7MDGLz17nJZetLT");

// Hardcoded deployer address - only this address can call initialize
#[cfg(not(feature = "test"))]
pub const DEPLOYER_ADDRESS: Pubkey = pubkey!("94kZD71sbTKhqhcvY9D9Ra5BsLzKRZgznbBbQpBWmKrT");

// For testing, we use a different deployer address that we can generate a keypair for
#[cfg(feature = "test")]
pub const DEPLOYER_ADDRESS: Pubkey = pubkey!("9C6hybhQ6Aycep9jaUnP6uL9ZYvDjUp1aSkFWPUFJtpj");
