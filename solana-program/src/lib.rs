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

declare_id!("8joqMXgaBjc2gGtPVGdZ2tBMxzRJ8igw2SCZQAPky5CE");

// Hardcoded deployer address - only this address can call initialize
#[cfg(not(feature = "test"))]
pub const DEPLOYER_ADDRESS: Pubkey = pubkey!("BQGjVjG8ZJW4m4hXybjLRB367idYyAHWbyjPBeL2w1hq");

// For testing, we use a different deployer address that we can generate a keypair for
#[cfg(feature = "test")]
pub const DEPLOYER_ADDRESS: Pubkey = pubkey!("BQGjVjG8ZJW4m4hXybjLRB367idYyAHWbyjPBeL2w1hq");
