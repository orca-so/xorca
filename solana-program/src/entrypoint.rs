use borsh::de::BorshDeserialize;
use pinocchio::{account_info::AccountInfo, pubkey::Pubkey, ProgramResult};
use pinocchio_log::log;

use crate::{
    error::ErrorCode,
    instructions::{self, Instruction},
};

#[cfg(target_os = "solana")]
use pinocchio::{default_allocator, default_panic_handler, program_entrypoint};

#[cfg(target_os = "solana")]
program_entrypoint!(process_instruction);

#[cfg(target_os = "solana")]
default_allocator!();

#[cfg(target_os = "solana")]
default_panic_handler!();

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    if program_id != &crate::ID {
        return Err(ErrorCode::IncorrectProgramId.into());
    }
    let mut instruction_data = instruction_data;
    let instruction = Instruction::deserialize(&mut instruction_data)
        .map_err(|_| ErrorCode::UnknownInstructionDiscriminator)?;
    log!("Instruction: {}", instruction.to_string().as_str());
    match &instruction {
        Instruction::StakingPoolInitialize => {
            instructions::staking_pool_initialize::process_instruction(accounts)?;
        }
        Instruction::Stake {
            stake_amount,
            claim_index,
        } => {
            instructions::stake::process_instruction(accounts, stake_amount, claim_index)?;
        }
        Instruction::Unstake {
            unstake_amount,
            withdraw_index,
        } => {
            instructions::unstake::process_instruction(accounts, unstake_amount, withdraw_index)?;
        }
        Instruction::Withdraw { withdraw_index } => {
            instructions::withdraw::process_instruction(accounts, withdraw_index)?;
        }
    }
    Ok(())
}
