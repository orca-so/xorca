use pinocchio::pubkey::Pubkey;
use pinocchio_pubkey::pubkey;

pub const SYSTEM_PROGRAM_ID: Pubkey = pubkey!("11111111111111111111111111111111");

mod create_account;

pub use create_account::*;
