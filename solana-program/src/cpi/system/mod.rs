use pinocchio::pubkey::Pubkey;
use pinocchio_pubkey::pubkey;

pub const CLOCK_SYSVAR_ID: Pubkey = pubkey!("SysvarC1ock11111111111111111111111111111111");

mod clock;

pub use clock::*;
