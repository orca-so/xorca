use crate::error::ErrorCode;
use borsh::{BorshDeserialize, BorshSerialize};
use pinocchio::ProgramResult;
use shank::ShankType;

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, ShankType)]
pub enum Event<'a> {
    Stake {
        orca_stake_amount: &'a u64,
        vault_orca_amount: &'a u64,
        vault_escrowed_orca_amount: &'a u64,
        xorca_mint_supply: &'a u64,
        xorca_to_mint: &'a u64,
    },
    Unstake {
        xorca_unstake_amount: &'a u64,
        vault_orca_amount: &'a u64,
        vault_escrowed_orca_amount: &'a u64,
        xorca_mint_supply: &'a u64,
        withdrawable_orca_amount: &'a u64,
        cool_down_period_s: &'a i64,
        withdraw_index: &'a u8,
    },
    Withdraw {
        vault_escrowed_orca_amount: &'a u64,
        withdrawable_orca_amount: &'a u64,
        cool_down_period_s: &'a i64,
        withdraw_index: &'a u8,
    },
}

pub fn sol_log_data(data: &[&[u8]]) {
    #[cfg(target_os = "solana")]
    unsafe {
        pinocchio::syscalls::sol_log_data(data as *const _ as *const u8, data.len() as u64)
    };

    #[cfg(not(target_os = "solana"))]
    core::hint::black_box(data);
}

impl<'a> Event<'a> {
    pub fn emit(&self) -> ProgramResult {
        let data = borsh::to_vec(self).map_err(|_| ErrorCode::EmitEventError)?;
        crate::event::sol_log_data(&[&data]);
        Ok(())
    }
}
