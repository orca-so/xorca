use borsh::{BorshDeserialize, BorshSerialize};
use pinocchio::{instruction::Seed, pubkey::Pubkey};
use shank::ShankAccount;

use super::{AccountDiscriminator, ProgramAccount};

#[derive(Debug, Clone, Copy, Eq, PartialEq, BorshSerialize, BorshDeserialize, ShankAccount)]
#[repr(C)]
pub struct StakingPool {
    discriminator: AccountDiscriminator,
    pub stake_token_mint: Pubkey,
    pub lst_token_mint: Pubkey,
    pub wind_up_period_s: u64,
    pub cool_down_period_s: u64,
    pub update_authority: Pubkey,
    pub escrowed_stake_token_amount: u64,
}

impl StakingPool {
    pub fn seeds(stake_token_mint: &Pubkey) -> Vec<Seed<'_>> {
        vec![Seed::from(b"staking_pool"), Seed::from(stake_token_mint)]
    }
}

impl ProgramAccount for StakingPool {
    const LEN: usize = 2048;
    const DISCRIMINATOR: AccountDiscriminator = AccountDiscriminator::StakingPool;
}
