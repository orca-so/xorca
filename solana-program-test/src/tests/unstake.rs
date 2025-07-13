use crate::{
    assert_program_success, state_data, token_account_data, token_mint_data, TestContext,
    ATA_PROGRAM_ID, ORCA_ID, SYSTEM_PROGRAM_ID, TOKEN_PROGRAM_ID, XORCA_ID, XORCA_PROGRAM_ID,
};
use solana_sdk::{clock::Clock, pubkey::Pubkey};
use xorca::{
    find_pending_withdraw_pda, find_state_address, PendingWithdraw, State, TokenAccount, TokenMint,
    Unstake, UnstakeInstructionArgs, DEFAULT_ACCOUNT_LEN,
};

/// Sets up the basic test context with correct PDAs and initial mint accounts.
fn setup_base_unstake_context(
    ctx: &mut TestContext,
) -> (Pubkey, Pubkey, Pubkey, Pubkey, Pubkey, u8) {
    let state_account = find_state_address().unwrap().0;
    let unstaker_signer = ctx.signer();
    ctx.write_account(
        state_account,
        XORCA_PROGRAM_ID,
        state_data!(
            escrowed_orca_amount => 0, // 0 ORCA
            update_authority => Pubkey::default(),
            cool_down_period_s => 7 * 24 * 60 * 60,
        ),
    )
    .unwrap();
    ctx.pad_account(state_account, DEFAULT_ACCOUNT_LEN).unwrap();
    ctx.write_account(
        XORCA_ID,
        TOKEN_PROGRAM_ID,
        token_mint_data!(
            supply => 1_000_000_000_000, // 1,000 xORCA
            decimals => 9,
            mint_authority_flag => 1,
            mint_authority => state_account,
            is_initialized => true,
            freeze_authority_flag => 0,
            freeze_authority => Pubkey::default(),
        ),
    )
    .unwrap();
    ctx.write_account(
        ORCA_ID,
        TOKEN_PROGRAM_ID,
        token_mint_data!(
            supply => 1_000_000_000_000, // 1_000_000 ORCA
            decimals => 6,
            mint_authority_flag => 1,
            mint_authority => Pubkey::default(),
            is_initialized => true,
            freeze_authority_flag => 0,
            freeze_authority => Pubkey::default(),
        ),
    )
    .unwrap();
    let vault_account = Pubkey::find_program_address(
        &[
            &state_account.to_bytes(),
            &TOKEN_PROGRAM_ID.to_bytes(),
            &ORCA_ID.to_bytes(),
        ],
        &ATA_PROGRAM_ID,
    )
    .0;
    ctx.write_account(
        vault_account,
        TOKEN_PROGRAM_ID,
        token_account_data!(
            mint => ORCA_ID,
            owner => state_account,
            amount => 1_000_000_000, // 1,000 ORCA
        ),
    )
    .unwrap();
    let unstaker_xorca_ata = Pubkey::find_program_address(
        &[
            &unstaker_signer.to_bytes(),
            &TOKEN_PROGRAM_ID.to_bytes(),
            &XORCA_ID.to_bytes(),
        ],
        &ATA_PROGRAM_ID,
    )
    .0;
    ctx.write_account(
        unstaker_xorca_ata,
        TOKEN_PROGRAM_ID,
        token_account_data!(
            mint => XORCA_ID,
            owner => unstaker_signer,
            amount => 10_000_000_000, // 10 xORCA
        ),
    )
    .unwrap();
    let withdraw_index = 0;
    let pending_withdraw_account = find_pending_withdraw_pda(&unstaker_signer, &withdraw_index)
        .unwrap()
        .0;
    (
        state_account,
        vault_account,
        unstaker_xorca_ata,
        unstaker_signer,
        pending_withdraw_account,
        withdraw_index,
    )
}

// --- Test Functions ---

#[test]
fn test_unstake_success_1_1_exchange() {
    let mut ctx = TestContext::new();
    let (
        state_account,
        vault_account,
        unstaker_xorca_ata,
        unstaker_signer,
        pending_withdraw_account,
        withdraw_index,
    ) = setup_base_unstake_context(&mut ctx);
    let unstake_amount = 10_000_000_000; // 10 xORCA
    let ix = Unstake {
        unstaker_account: unstaker_signer,
        state_account,
        vault_account,
        pending_withdraw_account,
        unstaker_xorca_ata,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        system_program_account: SYSTEM_PROGRAM_ID,
        token_program_account: TOKEN_PROGRAM_ID,
    }
    .instruction(UnstakeInstructionArgs {
        unstake_amount,
        withdraw_index,
    });
    let result = ctx.send(ix);
    assert_program_success!(result);
    let vault_account_after = ctx.get_account::<TokenAccount>(vault_account).unwrap();
    let unstaker_xorca_ata_after = ctx.get_account::<TokenAccount>(unstaker_xorca_ata).unwrap();
    let xorca_mint_account_after = ctx.get_account::<TokenMint>(XORCA_ID).unwrap();
    let state_account_after = ctx.get_account::<State>(state_account).unwrap();
    let current_timestamp = ctx.svm.get_sysvar::<Clock>().unix_timestamp;

    println!("current_timestamp: {}", current_timestamp);
    assert_eq!(
        vault_account_after.data.amount,
        1_000_000_000, // vault's orca remains unchanged 1,000 ORCA
        "Vault account should have 1,000 ORCA"
    );
    assert_eq!(
        unstaker_xorca_ata_after.data.amount, 0,
        "Staker xOrca ATA should have 0 xORCA"
    );
    assert_eq!(
        xorca_mint_account_after.data.supply,
        990_000_000_000, // 990 xORCA (9 decimals)
        "xOrca supply should be 1 xORCA"
    );
    assert_eq!(
        state_account_after.data.escrowed_orca_amount,
        10_000_000, // 10 ORCA
        "Escrowed Orca amount should be 10 ORCA"
    );
    let pending_withdraw_after = ctx
        .get_account::<PendingWithdraw>(pending_withdraw_account)
        .unwrap();
    assert_eq!(
        pending_withdraw_after.data.withdrawable_orca_amount,
        10_000_000, // 10 ORCA
        "Pending withdraw amount should be 10 ORCA"
    );
    assert_eq!(
        pending_withdraw_after.data.withdrawable_timestamp,
        current_timestamp + 7 * 24 * 60 * 60,
        "Pending withdraw timestamp should be 7 days from now"
    );
    assert_eq!(pending_withdraw_after.data.padding1, [0; 7]);
    assert_eq!(pending_withdraw_after.data.padding2, [0; 2024]);
}
