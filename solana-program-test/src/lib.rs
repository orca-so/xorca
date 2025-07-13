#![cfg(test)]

use borsh::{BorshDeserialize, BorshSerialize};
use litesvm::{types::TransactionResult, LiteSVM};
use solana_sdk::{
    account::Account,
    compute_budget::ComputeBudgetInstruction,
    instruction::Instruction,
    message::{Message, VersionedMessage},
    native_token::LAMPORTS_PER_SOL,
    packet::PACKET_DATA_SIZE,
    program_error::ProgramError,
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
    system_instruction, system_program,
    transaction::VersionedTransaction,
};
use std::error::Error;
use xorca::DecodedAccount;

mod assertions;
mod tests;
mod utils;

pub const SYSTEM_PROGRAM_ID: Pubkey = system_program::ID;
pub const JITO_TIP_ADDRESS: Pubkey =
    solana_sdk::pubkey!("96gYZGLnJYVFmbjzopPSU6QiEV5fGqZNyN9nmNhvrZU5");
pub const TOKEN_PROGRAM_ID: Pubkey =
    solana_sdk::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
pub const ORCA_ID: Pubkey = solana_sdk::pubkey!("orcaEKTdK7LKz57vaAYr9QeNsVEPfiu6QeMU1kektZE");
pub const XORCA_ID: Pubkey = solana_sdk::pubkey!("xorcaYqbXUNz3474ubUMJAdu2xgPsew3rUCe5ughT3N");
pub const XORCA_PROGRAM_ID: Pubkey =
    solana_sdk::pubkey!("5kyCqwYt8Pk65g3cG45SaBa2CBvjjBuaWiE3ubf2JcwY");
pub const ATA_PROGRAM_ID: Pubkey =
    solana_sdk::pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");

struct TestContext {
    svm: LiteSVM,
    signer: Keypair,
    verify_tx_size: bool,
}

impl TestContext {
    pub fn new() -> Self {
        let signer = Keypair::new();
        let mut svm = LiteSVM::new()
            .with_sigverify(false)
            .with_blockhash_check(false)
            .with_log_bytes_limit(Some(100_000));
        svm.airdrop(&signer.pubkey(), LAMPORTS_PER_SOL).unwrap();
        svm.add_program(
            XORCA_PROGRAM_ID,
            include_bytes!("../../target/deploy/xorca_staking_program.so"),
        );
        Self {
            svm,
            signer,
            verify_tx_size: true,
        }
    }

    pub fn signer(&self) -> Pubkey {
        self.signer.pubkey()
    }

    pub fn write_account<T: BorshSerialize>(
        &mut self,
        address: Pubkey,
        owner: Pubkey,
        account: T,
    ) -> Result<(), Box<dyn Error>> {
        let data = borsh::to_vec(&account)?;
        self.write_raw_account(address, owner, data)
    }

    pub fn write_raw_account(
        &mut self,
        address: Pubkey,
        owner: Pubkey,
        data: Vec<u8>,
    ) -> Result<(), Box<dyn Error>> {
        self.svm.set_account(
            address,
            Account {
                data,
                owner,
                lamports: LAMPORTS_PER_SOL,
                executable: false,
                rent_epoch: 0,
            },
        )?;
        Ok(())
    }

    pub fn send(&mut self, ix: Instruction) -> TransactionResult {
        let result = self.sends(&[ix]);

        // Print logs from the result
        match &result {
            Ok(meta) => {
                println!("Transaction succeeded!");
                println!("Transaction logs:");
                for log in &meta.logs {
                    println!("  {}", log);
                }
                println!("Compute units consumed: {}", meta.compute_units_consumed);
            }
            Err(e) => {
                println!("Transaction failed with error: {:?}", e);
                // Access metadata directly from the error
                println!("Transaction logs:");
                for log in &e.meta.logs {
                    println!("  {}", log);
                }
                println!("Compute units consumed: {}", e.meta.compute_units_consumed);
            }
        }

        result
    }

    pub fn sends(&mut self, ix: &[Instruction]) -> TransactionResult {
        // Add compute budget instructions to make sure the instruction fits in a tx
        let msg = Message::new(
            &[
                &[
                    ComputeBudgetInstruction::set_compute_unit_limit(1_400_000),
                    ComputeBudgetInstruction::set_compute_unit_price(0),
                    system_instruction::transfer(&self.signer(), &JITO_TIP_ADDRESS, 0),
                ],
                ix,
            ]
            .concat(),
            Some(&self.signer()),
        );
        let tx = VersionedTransaction {
            signatures: vec![Signature::new_unique(); msg.header.num_required_signatures as usize],
            message: VersionedMessage::Legacy(msg),
        };
        let bytes = bincode::serialize(&tx).map_err(|_| ProgramError::Custom(0))?;
        if self.verify_tx_size {
            assert!(
                bytes.len() <= PACKET_DATA_SIZE,
                "Transaction of {} bytes is too large",
                bytes.len()
            );
        }
        self.svm.send_transaction(tx)
    }

    pub fn get_account<T: BorshDeserialize>(
        &self,
        address: Pubkey,
    ) -> Result<DecodedAccount<T>, Box<dyn Error>> {
        let account = self.get_raw_account(address)?;
        let data = T::deserialize(&mut account.data.as_slice())?;
        Ok(DecodedAccount {
            address,
            account,
            data,
        })
    }

    pub fn get_raw_account(&self, address: Pubkey) -> Result<Account, Box<dyn Error>> {
        let account = self
            .svm
            .get_account(&address)
            .ok_or(format!("Account not found: {}", address))?;
        Ok(account)
    }

    pub fn print_transaction_logs(&self, result: &TransactionResult) {
        match result {
            Ok(_) => {
                println!("Transaction succeeded!");
                // Note: For successful transactions, logs might not be directly accessible
                // You may need to check the transaction result structure for your specific LiteSVM version
            }
            Err(e) => {
                println!("Transaction failed with error: {:?}", e);
                // Access metadata directly from the error
                println!("Transaction logs:");
                for log in &e.meta.logs {
                    println!("  {}", log);
                }
                println!("Compute units consumed: {}", e.meta.compute_units_consumed);
            }
        }
    }
}
