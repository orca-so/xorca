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
    signature::Keypair,
    signer::Signer,
    transaction::VersionedTransaction,
};
use solana_sdk::{system_instruction, system_program};
use std::{cell::RefCell, error::Error, rc::Rc};
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
    solana_sdk::pubkey!("StaKE6XNKVVhG8Qu9hDJBqCW3eRe7MDGLz17nJZetLT");
pub const ATA_PROGRAM_ID: Pubkey =
    solana_sdk::pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");
// In test mode, we use a different deployer address that we can generate a keypair for
pub const DEPLOYER_ADDRESS: Pubkey =
    solana_sdk::pubkey!("9C6hybhQ6Aycep9jaUnP6uL9ZYvDjUp1aSkFWPUFJtpj");

struct TestContext {
    svm: Rc<RefCell<LiteSVM>>,
    signer: Keypair,
    verify_tx_size: bool,
}

impl TestContext {
    pub fn new() -> Self {
        // Create a deterministic keypair for testing
        // We'll use a deterministic seed to ensure consistent keypair generation
        let signer_seed = [
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e,
            0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c,
            0x1d, 0x1e, 0x1f, 0x20,
        ];
        let signer = Keypair::new_from_array(signer_seed);

        let mut svm = LiteSVM::new()
            .with_blockhash_check(false)
            .with_log_bytes_limit(Some(100_000));

        // Fund the test signer / deployer
        svm.airdrop(&signer.pubkey(), LAMPORTS_PER_SOL).unwrap();

        // Add the program
        svm.add_program(
            XORCA_PROGRAM_ID,
            include_bytes!("../../target/deploy/xorca_staking_program.so"),
        )
        .unwrap();
        Self {
            svm: Rc::new(RefCell::new(svm)),
            signer,
            verify_tx_size: true,
        }
    }

    pub fn new_signer(svm: Rc<RefCell<LiteSVM>>) -> Self {
        let signer: Keypair = Keypair::new();
        svm.borrow_mut()
            .airdrop(&signer.pubkey(), LAMPORTS_PER_SOL)
            .unwrap();
        Self {
            svm,
            signer,
            verify_tx_size: true,
        }
    }

    pub fn signer(&self) -> Pubkey {
        self.signer.pubkey()
    }

    pub fn signer_ref(&self) -> &Keypair {
        &self.signer
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
        self.svm.borrow_mut().set_account(
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

    pub fn sends(&mut self, ix: &[Instruction]) -> TransactionResult {
        let recent_blockhash = self.svm.borrow().latest_blockhash();

        // Add compute budget instructions to make sure the instruction fits in a tx
        let msg = Message::new_with_blockhash(
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
            &recent_blockhash,
        );

        let tx = VersionedTransaction {
            signatures: vec![self.signer.sign_message(&msg.serialize())],
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
        self.svm.borrow_mut().send_transaction(tx)
    }

    pub fn sends_with_signers(
        &self,
        ix: &[Instruction],
        signers: &[&Keypair],
    ) -> TransactionResult {
        let recent_blockhash = self.svm.borrow().latest_blockhash();

        // Add compute budget instructions to make sure the instruction fits in a tx
        let msg = Message::new_with_blockhash(
            &[
                &[
                    ComputeBudgetInstruction::set_compute_unit_limit(1_400_000),
                    ComputeBudgetInstruction::set_compute_unit_price(0),
                    system_instruction::transfer(&signers[0].pubkey(), &JITO_TIP_ADDRESS, 0),
                ],
                ix,
            ]
            .concat(),
            Some(&signers[0].pubkey()),
            &recent_blockhash,
        );

        let tx = VersionedTransaction {
            signatures: signers
                .iter()
                .map(|s| s.sign_message(&msg.serialize()))
                .collect(),
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
        self.svm.borrow_mut().send_transaction(tx)
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
            .borrow()
            .get_account(&address)
            .ok_or(format!("Account not found: {}", address))?;
        Ok(account)
    }

    // Helper methods to expose SVM functionality
    pub fn get_sysvar<T: solana_sdk::sysvar::Sysvar>(&self) -> T {
        self.svm.borrow().get_sysvar::<T>()
    }

    pub fn set_sysvar<T: solana_sdk::sysvar::Sysvar>(&self, sysvar: &T) {
        self.svm.borrow_mut().set_sysvar::<T>(sysvar);
    }

    pub fn set_account(&self, address: Pubkey, account: Account) -> Result<(), Box<dyn Error>> {
        self.svm.borrow_mut().set_account(address, account)?;
        Ok(())
    }
}
