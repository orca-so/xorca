use crate::{
    assert_program_error, assert_program_success, pending_withdraw_data, state_data,
    token_account_data, token_mint_data, TestContext, ATA_PROGRAM_ID, ORCA_ID, SYSTEM_PROGRAM_ID,
    TOKEN_PROGRAM_ID, XORCA_ID, XORCA_PROGRAM_ID,
};
use solana_sdk::{clock::Clock, pubkey::Pubkey};
use xorca::{
    find_pending_withdraw_pda, find_state_address, State, TokenAccount, TokenMint, Withdraw,
    WithdrawInstructionArgs, XorcaStakingProgramError,
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

// --- Invalid Account Configuration Helpers for Withdraw ---

// State Account
fn make_state_account_invalid_owner(ctx: &mut TestContext, state_account: Pubkey) {
    ctx.write_account(
        state_account,
        TOKEN_PROGRAM_ID, // Incorrect owner
        state_data!(
            escrowed_orca_amount => 0,
            update_authority => Pubkey::default(),
            cool_down_period_s => 7 * 24 * 60 * 60,
        ),
    )
    .unwrap();
}

fn make_state_account_invalid_seeds(ctx: &mut TestContext) -> Pubkey {
    let invalid_state_account =
        Pubkey::find_program_address(&[b"invalid_seed"], &XORCA_PROGRAM_ID).0;
    ctx.write_account(
        invalid_state_account,
        XORCA_PROGRAM_ID,
        state_data!(
            escrowed_orca_amount => 0,
            update_authority => Pubkey::default(),
            cool_down_period_s => 7 * 24 * 60 * 60,
        ),
    )
    .unwrap();
    invalid_state_account
}

// Pending Withdraw Account
fn make_pending_withdraw_invalid_owner(ctx: &mut TestContext, pending_withdraw_account: Pubkey) {
    ctx.write_account(
        pending_withdraw_account,
        TOKEN_PROGRAM_ID, // Incorrect owner
        // Initializing with default data as if it were a new account for the first time
        pending_withdraw_data!(
            withdrawable_orca_amount => 0,
            withdrawable_timestamp => 0,
            padding1 => [0; 7],
            padding2 => [0; 968],
        ),
    )
    .unwrap();
}

fn make_pending_withdraw_invalid_seeds(ctx: &mut TestContext, unstaker_signer: Pubkey) -> Pubkey {
    let invalid_pending_withdraw_account = find_pending_withdraw_pda(&unstaker_signer, &(100))
        .unwrap()
        .0;
    ctx.write_account(
        invalid_pending_withdraw_account,
        XORCA_PROGRAM_ID, // Correct owner, but invalid seeds in test context
        // Initializing with default data as if it were a new account for the first time
        pending_withdraw_data!(
            withdrawable_orca_amount => 0,
            withdrawable_timestamp => 0,
            padding1 => [0; 7],
            padding2 => [0; 968],
        ),
    )
    .unwrap();
    invalid_pending_withdraw_account
}

// Unstaker ORCA ATA
fn make_unstaker_orca_ata_invalid_program_owner(ctx: &mut TestContext, unstaker_orca_ata: Pubkey) {
    ctx.write_account(
        unstaker_orca_ata,
        ATA_PROGRAM_ID, // Incorrect program owner for the account itself (should be TOKEN_PROGRAM_ID)
        token_account_data!(
            mint => ORCA_ID,
            owner => ctx.signer(),
            amount => 10_000_000,
        ),
    )
    .unwrap();
}

fn make_unstaker_orca_ata_invalid_mint_in_data(ctx: &mut TestContext, unstaker_orca_ata: Pubkey) {
    ctx.write_account(
        unstaker_orca_ata,
        TOKEN_PROGRAM_ID,
        token_account_data!(
            mint => XORCA_ID, // Incorrect mint
            owner => ctx.signer(),
            amount => 10_000_000,
        ),
    )
    .unwrap();
}

fn make_unstaker_orca_ata_invalid_owner_in_data(ctx: &mut TestContext, unstaker_orca_ata: Pubkey) {
    ctx.write_account(
        unstaker_orca_ata,
        TOKEN_PROGRAM_ID,
        token_account_data!(
            mint => ORCA_ID,
            owner => Pubkey::new_unique(), // Incorrect owner
            amount => 10_000_000,
        ),
    )
    .unwrap();
}

// Vault Account
fn make_vault_account_invalid_program_owner(ctx: &mut TestContext, vault_account: Pubkey) {
    ctx.write_account(
        vault_account,
        XORCA_PROGRAM_ID, // Incorrect owner
        token_account_data!(
            mint => ORCA_ID,
            owner => find_state_address().unwrap().0,
            amount => 1_000_000_000,
        ),
    )
    .unwrap();
}

fn make_vault_account_invalid_mint_in_data(ctx: &mut TestContext, vault_account: Pubkey) {
    ctx.write_account(
        vault_account,
        TOKEN_PROGRAM_ID,
        token_account_data!(
            mint => XORCA_ID, // Incorrect mint
            owner => find_state_address().unwrap().0,
            amount => 1_000_000_000,
        ),
    )
    .unwrap();
}

fn make_vault_account_invalid_owner_in_data(ctx: &mut TestContext, vault_account: Pubkey) {
    ctx.write_account(
        vault_account,
        TOKEN_PROGRAM_ID,
        token_account_data!(
            mint => ORCA_ID,
            owner => Pubkey::new_unique(), // Incorrect owner
            amount => 1_000_000_000,
        ),
    )
    .unwrap();
}

// ORCA Mint Account
fn make_orca_mint_invalid_owner(ctx: &mut TestContext) {
    ctx.write_account(
        ORCA_ID,
        SYSTEM_PROGRAM_ID, // Incorrect owner
        token_mint_data!(
            supply => 1_000_000_000_000,
            decimals => 6,
            mint_authority_flag => 1,
            mint_authority => Pubkey::default(),
            is_initialized => true,
            freeze_authority_flag => 0,
            freeze_authority => Pubkey::default(),
        ),
    )
    .unwrap();
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
    // Update clock to be at or after withdrawable_timestamp
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
        vault_account_after.data.amount,
        990_000_000, // Initial 1,000 ORCA - 10 ORCA withdrawn = 990 ORCA
        "Vault account should have 990 ORCA"
    );
    assert_eq!(
        xorca_mint_account_after.data.supply,
        1_000_000_000_000, // 1,000 xORCA
        "xORCA supply should be 1000 xORCA"
    );
    assert_eq!(
        unstaker_orca_ata_after.data.amount,
        20_000_000, // Initial 10 ORCA + 10 ORCA withdrawn = 20 ORCA
        "Unstaker Orca ATA should have 20 ORCA"
    );
    assert_eq!(
        state_account_after.data.escrowed_orca_amount, 0,
        "Escrowed Orca amount should be 0 ORCA"
    );
}

#[test]
fn test_withdraw_fail_cooldown_not_over() {
    let mut ctx = TestContext::new();
    let withdraw_index = 0;
    let (
        state_account,
        vault_account,
        unstaker_orca_ata,
        unstaker_signer,
        pending_withdraw_account,
        _withdrawable_timestamp, // Don't advance the clock
    ) = setup_base_withdraw_context(&mut ctx, withdraw_index);
    // Clock is intentionally not advanced, so it's before the withdrawable_timestamp
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
    assert_program_error!(result, XorcaStakingProgramError::CoolDownPeriodStillActive);
}

#[test]
fn test_withdraw_invalid_withdraw_index() {
    let mut ctx = TestContext::new();
    let withdraw_index = 0;
    let (
        state_account,
        vault_account,
        unstaker_orca_ata,
        unstaker_signer,
        _,
        withdrawable_timestamp,
    ) = setup_base_withdraw_context(&mut ctx, withdraw_index);
    let mut clock: Clock = ctx.svm.get_sysvar::<Clock>();
    clock.unix_timestamp = withdrawable_timestamp;
    ctx.svm.set_sysvar::<Clock>(&clock);
    let incorrect_withdraw_index = 1;
    let non_existent_pending_withdraw_account =
        find_pending_withdraw_pda(&unstaker_signer, &incorrect_withdraw_index)
            .unwrap()
            .0;
    let ix = Withdraw {
        unstaker_account: unstaker_signer,
        state_account,
        vault_account,
        pending_withdraw_account: non_existent_pending_withdraw_account,
        unstaker_orca_ata,
        orca_mint_account: ORCA_ID,
        system_program_account: SYSTEM_PROGRAM_ID,
        token_program_account: TOKEN_PROGRAM_ID,
    }
    .instruction(WithdrawInstructionArgs {
        withdraw_index: incorrect_withdraw_index,
    });
    let result = ctx.send(ix);
    assert_program_error!(result, XorcaStakingProgramError::IncorrectOwner);
}

/// --- Withdraw Invalid Account Tests ---

/// ### State Account Tests
#[test]
fn test_withdraw_invalid_state_account_owner() {
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
    let mut clock: Clock = ctx.svm.get_sysvar::<Clock>();
    clock.unix_timestamp = withdrawable_timestamp;
    ctx.svm.set_sysvar::<Clock>(&clock);
    make_state_account_invalid_owner(&mut ctx, state_account); // Make state account have wrong owner
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
    assert_program_error!(result, XorcaStakingProgramError::IncorrectOwner);
}

#[test]
fn test_withdraw_invalid_state_account_seeds() {
    let mut ctx = TestContext::new();
    let withdraw_index = 0;
    let (
        _state_account,
        vault_account,
        unstaker_orca_ata,
        unstaker_signer,
        pending_withdraw_account,
        withdrawable_timestamp,
    ) = setup_base_withdraw_context(&mut ctx, withdraw_index);
    let mut clock: Clock = ctx.svm.get_sysvar::<Clock>();
    clock.unix_timestamp = withdrawable_timestamp;
    ctx.svm.set_sysvar::<Clock>(&clock);
    let invalid_state_account = make_state_account_invalid_seeds(&mut ctx);
    let ix = Withdraw {
        unstaker_account: unstaker_signer,
        state_account: invalid_state_account,
        vault_account,
        pending_withdraw_account,
        unstaker_orca_ata,
        orca_mint_account: ORCA_ID,
        system_program_account: SYSTEM_PROGRAM_ID,
        token_program_account: TOKEN_PROGRAM_ID,
    }
    .instruction(WithdrawInstructionArgs { withdraw_index });
    let result = ctx.send(ix);
    assert_program_error!(result, XorcaStakingProgramError::InvalidSeeds);
}

/// ### Pending Withdraw Account Tests

#[test]
fn test_withdraw_invalid_pending_withdraw_account_owner() {
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
    let mut clock: Clock = ctx.svm.get_sysvar::<Clock>();
    clock.unix_timestamp = withdrawable_timestamp;
    ctx.svm.set_sysvar::<Clock>(&clock);
    make_pending_withdraw_invalid_owner(&mut ctx, pending_withdraw_account); // Make pending withdraw account have wrong owner
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
    assert_program_error!(result, XorcaStakingProgramError::IncorrectOwner);
}

#[test]
fn test_withdraw_invalid_pending_withdraw_account_seeds() {
    let mut ctx = TestContext::new();
    let withdraw_index = 0;
    let (
        state_account,
        vault_account,
        unstaker_orca_ata,
        unstaker_signer,
        _,
        withdrawable_timestamp,
    ) = setup_base_withdraw_context(&mut ctx, withdraw_index);
    let mut clock: Clock = ctx.svm.get_sysvar::<Clock>();
    clock.unix_timestamp = withdrawable_timestamp;
    ctx.svm.set_sysvar::<Clock>(&clock);
    let invalid_pending_withdraw_account =
        make_pending_withdraw_invalid_seeds(&mut ctx, unstaker_signer);
    let ix = Withdraw {
        unstaker_account: unstaker_signer,
        state_account,
        vault_account,
        pending_withdraw_account: invalid_pending_withdraw_account,
        unstaker_orca_ata,
        orca_mint_account: ORCA_ID,
        system_program_account: SYSTEM_PROGRAM_ID,
        token_program_account: TOKEN_PROGRAM_ID,
    }
    .instruction(WithdrawInstructionArgs { withdraw_index });
    let result = ctx.send(ix);
    assert_program_error!(result, XorcaStakingProgramError::InvalidSeeds);
}

/// ### Unstaker ORCA ATA Tests
#[test]
fn test_withdraw_invalid_unstaker_orca_ata_program_owner() {
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
    let mut clock: Clock = ctx.svm.get_sysvar::<Clock>();
    clock.unix_timestamp = withdrawable_timestamp;
    ctx.svm.set_sysvar::<Clock>(&clock);
    make_unstaker_orca_ata_invalid_program_owner(&mut ctx, unstaker_orca_ata); // Make unstaker ORCA ATA have wrong program owner
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
    assert_program_error!(result, XorcaStakingProgramError::IncorrectOwner);
}

#[test]
fn test_withdraw_invalid_unstaker_orca_ata_mint_in_data() {
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
    let mut clock: Clock = ctx.svm.get_sysvar::<Clock>();
    clock.unix_timestamp = withdrawable_timestamp;
    ctx.svm.set_sysvar::<Clock>(&clock);
    make_unstaker_orca_ata_invalid_mint_in_data(&mut ctx, unstaker_orca_ata); // Make unstaker ORCA ATA have wrong mint in data
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
    assert_program_error!(result, XorcaStakingProgramError::InvalidAccountData);
}

#[test]
fn test_withdraw_invalid_unstaker_orca_ata_owner_in_data() {
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
    let mut clock: Clock = ctx.svm.get_sysvar::<Clock>();
    clock.unix_timestamp = withdrawable_timestamp;
    ctx.svm.set_sysvar::<Clock>(&clock);
    make_unstaker_orca_ata_invalid_owner_in_data(&mut ctx, unstaker_orca_ata); // Make unstaker ORCA ATA have wrong owner in data
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
    assert_program_error!(result, XorcaStakingProgramError::InvalidAccountData);
}

/// ### Vault Account Tests
#[test]
fn test_withdraw_invalid_vault_account_program_owner() {
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
    let mut clock: Clock = ctx.svm.get_sysvar::<Clock>();
    clock.unix_timestamp = withdrawable_timestamp;
    ctx.svm.set_sysvar::<Clock>(&clock);
    make_vault_account_invalid_program_owner(&mut ctx, vault_account);
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
    assert_program_error!(result, XorcaStakingProgramError::IncorrectOwner);
}

#[test]
fn test_withdraw_invalid_vault_account_mint_in_data() {
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
    let mut clock: Clock = ctx.svm.get_sysvar::<Clock>();
    clock.unix_timestamp = withdrawable_timestamp;
    ctx.svm.set_sysvar::<Clock>(&clock);
    make_vault_account_invalid_mint_in_data(&mut ctx, vault_account);
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
    assert_program_error!(result, XorcaStakingProgramError::InvalidAccountData);
}

#[test]
fn test_withdraw_invalid_vault_account_owner_in_data() {
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
    let mut clock: Clock = ctx.svm.get_sysvar::<Clock>();
    clock.unix_timestamp = withdrawable_timestamp;
    ctx.svm.set_sysvar::<Clock>(&clock);
    make_vault_account_invalid_owner_in_data(&mut ctx, vault_account); // Make vault account have wrong owner in data
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
    assert_program_error!(result, XorcaStakingProgramError::InvalidAccountData);
}

/// ### ORCA Mint Account Tests
#[test]
fn test_withdraw_invalid_orca_mint_owner() {
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

    let mut clock: Clock = ctx.svm.get_sysvar::<Clock>();
    clock.unix_timestamp = withdrawable_timestamp;
    ctx.svm.set_sysvar::<Clock>(&clock);
    make_orca_mint_invalid_owner(&mut ctx); // Make ORCA mint have wrong owner
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
    assert_program_error!(result, XorcaStakingProgramError::IncorrectOwner);
}

/// ### Invalid Program ID Tests
#[test]
fn test_withdraw_invalid_system_program_id() {
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
    let mut clock: Clock = ctx.svm.get_sysvar::<Clock>();
    clock.unix_timestamp = withdrawable_timestamp;
    ctx.svm.set_sysvar::<Clock>(&clock);
    let invalid_system_program_id = Pubkey::new_unique();
    let ix = Withdraw {
        unstaker_account: unstaker_signer,
        state_account,
        vault_account,
        pending_withdraw_account,
        unstaker_orca_ata,
        orca_mint_account: ORCA_ID,
        system_program_account: invalid_system_program_id, // Pass the invalid system program ID
        token_program_account: TOKEN_PROGRAM_ID,
    }
    .instruction(WithdrawInstructionArgs { withdraw_index });
    let result = ctx.send(ix);
    assert_program_error!(result, XorcaStakingProgramError::IncorrectAccountAddress);
}

#[test]
fn test_withdraw_invalid_token_program_id() {
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
    let mut clock: Clock = ctx.svm.get_sysvar::<Clock>();
    clock.unix_timestamp = withdrawable_timestamp;
    ctx.svm.set_sysvar::<Clock>(&clock);
    let invalid_token_program_id = Pubkey::new_unique();
    let ix = Withdraw {
        unstaker_account: unstaker_signer,
        state_account,
        vault_account,
        pending_withdraw_account,
        unstaker_orca_ata,
        orca_mint_account: ORCA_ID,
        system_program_account: SYSTEM_PROGRAM_ID,
        token_program_account: invalid_token_program_id, // Pass the invalid token program ID
    }
    .instruction(WithdrawInstructionArgs { withdraw_index });
    let result = ctx.send(ix);
    assert_program_error!(result, XorcaStakingProgramError::IncorrectAccountAddress);
}
