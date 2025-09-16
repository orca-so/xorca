pub mod assertions;
pub mod cpi;
pub mod entrypoint;
pub mod error;
pub mod event;
pub mod instructions;
pub mod pda;
pub mod state;
pub mod util;

use pinocchio_pubkey::{declare_id, pubkey};

declare_id!("5kyCqwYt8Pk65g3cG45SaBa2CBvjjBuaWiE3ubf2JcwY");

// Hardcoded deployer address - only this address can call initialize
#[cfg(not(feature = "test"))]
pub const DEPLOYER_ADDRESS: pinocchio::pubkey::Pubkey =
    pubkey!("GwH3Hiv5mACLX3ufTw1pFsrhSPon5tdw252DBs4Rx4PV");

// For testing, we use a different deployer address that we can generate a keypair for
#[cfg(feature = "test")]
pub const DEPLOYER_ADDRESS: pinocchio::pubkey::Pubkey =
    pubkey!("9C6hybhQ6Aycep9jaUnP6uL9ZYvDjUp1aSkFWPUFJtpj");
