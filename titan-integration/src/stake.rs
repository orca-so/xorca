use borsh::BorshDeserialize;
use borsh::BorshSerialize;

use crate::constants::XORCA_STAKING_PROGRAM_ID;

#[derive(Debug)]
pub struct Stake {
    pub staker_account: solana_pubkey::Pubkey,
    pub vault_account: solana_pubkey::Pubkey,
    pub staker_orca_ata: solana_pubkey::Pubkey,
    pub staker_xorca_ata: solana_pubkey::Pubkey,
    pub xorca_mint_account: solana_pubkey::Pubkey,
    pub state_account: solana_pubkey::Pubkey,
    pub orca_mint_account: solana_pubkey::Pubkey,
    pub token_program_account: solana_pubkey::Pubkey,
}

impl Stake {
    pub fn instruction(&self, args: StakeInstructionArgs) -> solana_instruction::Instruction {
        self.instruction_with_remaining_accounts(args, &[])
    }
    #[allow(clippy::arithmetic_side_effects)]
    #[allow(clippy::vec_init_then_push)]
    pub fn instruction_with_remaining_accounts(
        &self,
        args: StakeInstructionArgs,
        remaining_accounts: &[solana_instruction::AccountMeta],
    ) -> solana_instruction::Instruction {
        let mut accounts = Vec::with_capacity(8 + remaining_accounts.len());
        accounts.push(solana_instruction::AccountMeta::new(
            self.staker_account,
            true,
        ));
        accounts.push(solana_instruction::AccountMeta::new(
            self.vault_account,
            false,
        ));
        accounts.push(solana_instruction::AccountMeta::new(
            self.staker_orca_ata,
            false,
        ));
        accounts.push(solana_instruction::AccountMeta::new(
            self.staker_xorca_ata,
            false,
        ));
        accounts.push(solana_instruction::AccountMeta::new(
            self.xorca_mint_account,
            false,
        ));
        accounts.push(solana_instruction::AccountMeta::new_readonly(
            self.state_account,
            false,
        ));
        accounts.push(solana_instruction::AccountMeta::new_readonly(
            self.orca_mint_account,
            false,
        ));
        accounts.push(solana_instruction::AccountMeta::new_readonly(
            self.token_program_account,
            false,
        ));
        accounts.extend_from_slice(remaining_accounts);
        let mut data = borsh::to_vec(&StakeInstructionData::new()).unwrap();
        let mut args = borsh::to_vec(&args).unwrap();
        data.append(&mut args);

        solana_instruction::Instruction {
            program_id: XORCA_STAKING_PROGRAM_ID,
            accounts,
            data,
        }
    }
}

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, Eq, PartialEq)]
pub struct StakeInstructionData {
    discriminator: u8,
}

impl StakeInstructionData {
    pub fn new() -> Self {
        Self { discriminator: 0 }
    }
}

impl Default for StakeInstructionData {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, Eq, PartialEq)]
pub struct StakeInstructionArgs {
    pub orca_stake_amount: u64,
}
