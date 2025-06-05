#![cfg(test)]

use litesvm::LiteSVM;

use solana_sdk::{
    native_token::LAMPORTS_PER_SOL, pubkey::Pubkey, signature::Keypair, signer::Signer,
};
use std::str::FromStr;

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
            Pubkey::from_str("5kyCqwYt8Pk65g3cG45SaBa2CBvjjBuaWiE3ubf2JcwY").unwrap(),
            include_bytes!("../../target/deploy/xorca_staking_program.so"),
        );
        Self {
            svm,
            signer,
            verify_tx_size: true,
        }
    }
}
