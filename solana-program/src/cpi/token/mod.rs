use pinocchio::pubkey::Pubkey;
use pinocchio_pubkey::pubkey;

mod state;
pub use state::*;

mod token2022_state;
pub use token2022_state::*;

pub const ORCA_MINT_ID: Pubkey = pubkey!("orcaEKTdK7LKz57vaAYr9QeNsVEPfiu6QeMU1kektZE");
pub const XORCA_MINT_ID: Pubkey = pubkey!("xorcaYqbXUNz3474ubUMJAdu2xgPsew3rUCe5ughT3N");

// Token program IDs
pub const SPL_TOKEN_PROGRAM_ID: Pubkey = pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
pub const TOKEN_2022_PROGRAM_ID: Pubkey = pubkey!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");
