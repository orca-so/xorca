use crate::{
    assert_program_success, pending_withdraw_data, state_data, token_account_data, token_mint_data,
    TestContext, ATA_PROGRAM_ID, ORCA_ID, SYSTEM_PROGRAM_ID, TOKEN_PROGRAM_ID, XORCA_ID,
    XORCA_PROGRAM_ID,
};
use solana_sdk::{clock::Clock, pubkey::Pubkey};
use xorca::{
    find_pending_withdraw_pda, find_state_address, State, TokenAccount, TokenMint, Withdraw,
    WithdrawInstructionArgs,
};

/// Sets up the basic test context with correct PDAs and initial mint accounts.
fn setup_base_withdraw_context(
    ctx: &mut TestContext,
    withdraw_index: u8,
) -> (Pubkey, Pubkey, Pubkey, Pubkey, Pubkey, i64) {
    let state_account = find_state_address().unwrap().0;
    let unstaker_signer = ctx.signer();
    ctx.write_account(
        state_account,
        XORCA_PROGRAM_ID,
        state_data!(
            escrowed_orca_amount => 10_000_000, // 10 ORCA
            update_authority => Pubkey::default(),
            cool_down_period_s => 7 * 24 * 60 * 60,
        ),
    )
    .unwrap();
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
    let unstaker_orca_ata = Pubkey::find_program_address(
        &[
            &unstaker_signer.to_bytes(),
            &TOKEN_PROGRAM_ID.to_bytes(),
            &ORCA_ID.to_bytes(),
        ],
        &ATA_PROGRAM_ID,
    )
    .0;
    ctx.write_account(
        unstaker_orca_ata,
        TOKEN_PROGRAM_ID,
        token_account_data!(
            mint => ORCA_ID,
            owner => unstaker_signer,
            amount => 10_000_000, // 10 ORCA
        ),
    )
    .unwrap();
    let pending_withdraw_account = find_pending_withdraw_pda(&unstaker_signer, &withdraw_index)
        .unwrap()
        .0;
    let current_timestamp = ctx.svm.get_sysvar::<Clock>().unix_timestamp;
    let withdrawable_timestamp = current_timestamp + 7 * 24 * 60 * 60;
    ctx.write_account(
        pending_withdraw_account,
        XORCA_PROGRAM_ID,
        pending_withdraw_data!(
            withdrawable_orca_amount => 10_000_000, // 10 ORCA
            withdrawable_timestamp => withdrawable_timestamp,
        ),
    )
    .unwrap();
    (
        state_account,
        vault_account,
        unstaker_orca_ata,
        unstaker_signer,
        pending_withdraw_account,
        withdrawable_timestamp,
    )
}

// --- Test Functions (continued) ---

#[test]
fn test_withdraw_success() {
    let mut ctx = TestContext::new();
    let withdraw_index = 0;
    let (
        state_account,
        vault_account,
        unstaker_orca_ata,
        unstaker_signer,
        pending_withdraw_account,
        withdrawable_timestamp,
    ) = setup_base_withdraw_context(&mut ctx, withdraw_index);
    // Update clock
    let mut clock: Clock = ctx.svm.get_sysvar::<Clock>();
    clock.unix_timestamp = withdrawable_timestamp;
    ctx.svm.set_sysvar::<Clock>(&clock);
    let ix = Withdraw {
        unstaker_account: unstaker_signer,
        state_account,
        vault_account,
        pending_withdraw_account,
        unstaker_orca_ata,
        orca_mint_account: ORCA_ID,
        system_program_account: SYSTEM_PROGRAM_ID,
        token_program_account: TOKEN_PROGRAM_ID,
    }
    .instruction(WithdrawInstructionArgs { withdraw_index });
    let result = ctx.send(ix);
    assert_program_success!(result);
    let vault_account_after = ctx.get_account::<TokenAccount>(vault_account).unwrap();
    let unstaker_orca_ata_after = ctx.get_account::<TokenAccount>(unstaker_orca_ata).unwrap();
    let xorca_mint_account_after = ctx.get_account::<TokenMint>(XORCA_ID).unwrap();
    let state_account_after = ctx.get_account::<State>(state_account).unwrap();
    assert_eq!(
        vault_account_after.data.amount, 990_000_000,
        "Vault account should have 990 ORCA"
    );
    assert_eq!(
        unstaker_orca_ata_after.data.amount, 20_000_000,
        "Unstaker Orca ATA should have 20 ORCA"
    );
    assert_eq!(
        xorca_mint_account_after.data.supply, 1_000_000_000_000,
        "xOrca supply should be 1000 xORCA"
    );
    assert_eq!(
        state_account_after.data.escrowed_orca_amount, 0,
        "Escrowed Orca amount should be 0 ORCA"
    );
}
