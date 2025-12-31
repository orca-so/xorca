//! Instruction types

#![allow(clippy::too_many_arguments)]
#![allow(deprecated)]

use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;
use solana_sysvar::__private::ProgramError;
use std::mem::size_of;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SwapInstructionBaseIn {
    // SOURCE amount to transfer, output to DESTINATION is based on the exchange rate
    pub amount_in: u64,
    /// Minimum amount of DESTINATION token to output, prevents excessive slippage
    pub minimum_amount_out: u64,
}

/// Instructions supported by the AmmInfo program.
#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub enum AmmInstruction {
    /// Swap coin or pc from pool with orderbook disable, base amount_in with a slippage of minimum_amount_out
    ///
    ///   0. `[]` Spl Token program id
    ///   1. `[writable]` AMM Account
    ///   2. `[]` $authority derived from `create_program_address(&[AUTHORITY_AMM, &[nonce]])`.
    ///   3. `[writable]` AMM coin vault Account to swap FROM or To.
    ///   4. `[writable]` AMM pc vault Account to swap FROM or To.
    ///   5. `[writable]` User source token Account.
    ///   6. `[writable]` User destination token Account.
    ///   7. `[signer]` User wallet Account
    SwapBaseInV2(SwapInstructionBaseIn),
}

impl AmmInstruction {
    /// Packs a [AmmInstruction](enum.AmmInstruction.html) into a byte buffer.
    pub fn pack(&self) -> Result<Vec<u8>, ProgramError> {
        let mut buf = Vec::with_capacity(size_of::<Self>());
        match &*self {
            Self::SwapBaseInV2(SwapInstructionBaseIn {
                amount_in,
                minimum_amount_out,
            }) => {
                buf.push(16);
                buf.extend_from_slice(&amount_in.to_le_bytes());
                buf.extend_from_slice(&minimum_amount_out.to_le_bytes());
            }
        }
        Ok(buf)
    }
}

/// Creates a 'swap base in v2' instruction.
pub fn swap_base_in_v2(
    amm_program: &Pubkey,
    amm_pool: &Pubkey,
    amm_authority: &Pubkey,
    amm_coin_vault: &Pubkey,
    amm_pc_vault: &Pubkey,
    user_token_source: &Pubkey,
    user_token_destination: &Pubkey,
    user_source_owner: &Pubkey,

    amount_in: u64,
    minimum_amount_out: u64,
) -> Result<Instruction, ProgramError> {
    let data = AmmInstruction::SwapBaseInV2(SwapInstructionBaseIn {
        amount_in,
        minimum_amount_out,
    })
    .pack()?;

    let accounts = vec![
        // spl token
        AccountMeta::new_readonly(spl_token::id(), false),
        // amm
        AccountMeta::new(*amm_pool, false),
        AccountMeta::new_readonly(*amm_authority, false),
        AccountMeta::new(*amm_coin_vault, false),
        AccountMeta::new(*amm_pc_vault, false),
        // user
        AccountMeta::new(*user_token_source, false),
        AccountMeta::new(*user_token_destination, false),
        AccountMeta::new_readonly(*user_source_owner, true),
    ];

    Ok(Instruction {
        program_id: *amm_program,
        accounts,
        data,
    })
}
