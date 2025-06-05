use borsh::{BorshDeserialize, BorshSerialize};
use pinocchio::instruction::Seed;
use shank::ShankAccount;

use super::{AccountDiscriminator, ProgramAccount};

#[derive(Debug, Clone, Copy, Eq, PartialEq, BorshSerialize, BorshDeserialize, ShankAccount)]
#[repr(C)]
pub struct StakingPool {
    discriminator: AccountDiscriminator,
}

impl StakingPool {
    pub fn seeds() -> Vec<Seed<'static>> {
        vec![Seed::from(b"staking_pool")]
    }
}

impl ProgramAccount for StakingPool {
    const LEN: usize = 2048;
    const DISCRIMINATOR: AccountDiscriminator = AccountDiscriminator::StakingPool;
}
