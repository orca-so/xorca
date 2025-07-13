use crate::{
    assert_program_error, assert_program_success, pending_withdraw_data, state_data,
    token_account_data, token_mint_data, TestContext, ATA_PROGRAM_ID, ORCA_ID, SYSTEM_PROGRAM_ID,
    TOKEN_PROGRAM_ID, XORCA_ID, XORCA_PROGRAM_ID,
};
use solana_sdk::{clock::Clock, pubkey::Pubkey};
use xorca::{
    find_pending_withdraw_pda, find_state_address, PendingWithdraw, State, TokenAccount, TokenMint,
    Unstake, UnstakeInstructionArgs, XorcaStakingProgramError, DEFAULT_ACCOUNT_LEN,
};

/// Sets up the basic test context with correct PDAs and initial mint accounts.
fn setup_base_unstake_context(
    ctx: &mut TestContext,
    withdraw_index: u8,
) -> (Pubkey, Pubkey, Pubkey, Pubkey, Pubkey) {
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
    let pending_withdraw_account = find_pending_withdraw_pda(&unstaker_signer, &withdraw_index)
        .unwrap()
        .0;
    (
        state_account,
        vault_account,
        unstaker_xorca_ata,
        unstaker_signer,
        pending_withdraw_account,
    )
}

fn set_balances_for_more_than_1_1_exchange(
    ctx: &mut TestContext,
    state_account: Pubkey,
    vault_account: Pubkey,
    unstaker_xorca_ata: Pubkey,
    unstaker_signer: Pubkey,
) {
    ctx.write_account(
        state_account,
        XORCA_PROGRAM_ID,
        state_data!(
            escrowed_orca_amount => 85_286_845_854, // 85,286.845854 ORCA
            update_authority => Pubkey::default(),
            cool_down_period_s => 30 * 24 * 60 * 60, // 30 days
        ),
    )
    .unwrap();
    ctx.pad_account(state_account, DEFAULT_ACCOUNT_LEN).unwrap();
    ctx.write_account(
        XORCA_ID,
        TOKEN_PROGRAM_ID,
        token_mint_data!(
            supply => 53_854_483_292_939_239, // 53,854,483.292939239 xORCA
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
        vault_account,
        TOKEN_PROGRAM_ID,
        token_account_data!(
            mint => ORCA_ID,
            owner => state_account,
            amount => 84_934_688_959_145, // 84,934,688.959145 ORCA
        ),
    )
    .unwrap();
    ctx.write_account(
        unstaker_xorca_ata,
        TOKEN_PROGRAM_ID,
        token_account_data!(
            mint => XORCA_ID,
            owner => unstaker_signer,
            amount => 95_611_428_484, // 95.611428484 xORCA
        ),
    )
    .unwrap();
}

// --- Invalid Account Configuration Helpers ---

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
    ctx.pad_account(state_account, DEFAULT_ACCOUNT_LEN).unwrap();
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
    ctx.pad_account(invalid_state_account, DEFAULT_ACCOUNT_LEN)
        .unwrap();
    invalid_state_account
}

// Vault Account
fn make_vault_account_invalid_owner(ctx: &mut TestContext, vault_account: Pubkey) {
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

// xORCA Mint Account
fn make_xorca_mint_invalid_owner(ctx: &mut TestContext) {
    ctx.write_account(
        XORCA_ID,
        SYSTEM_PROGRAM_ID, // Incorrect owner
        token_mint_data!(
            supply => 1_000_000_000_000,
            decimals => 9,
            mint_authority_flag => 1,
            mint_authority => find_state_address().unwrap().0,
            is_initialized => true,
            freeze_authority_flag => 0,
            freeze_authority => Pubkey::default(),
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

// Unstaker xORCA ATA
fn make_unstaker_xorca_ata_invalid_program_owner(
    ctx: &mut TestContext,
    unstaker_xorca_ata: Pubkey,
) {
    ctx.write_account(
        unstaker_xorca_ata,
        ATA_PROGRAM_ID, // Incorrect program owner for the account itself (should be TOKEN_PROGRAM_ID)
        token_account_data!(
            mint => XORCA_ID,
            owner => ctx.signer(),
            amount => 10_000_000_000,
        ),
    )
    .unwrap();
}

fn make_unstaker_xorca_ata_invalid_mint_in_data(ctx: &mut TestContext, unstaker_xorca_ata: Pubkey) {
    ctx.write_account(
        unstaker_xorca_ata,
        TOKEN_PROGRAM_ID,
        token_account_data!(
            mint => ORCA_ID, // Incorrect mint
            owner => ctx.signer(),
            amount => 10_000_000_000,
        ),
    )
    .unwrap();
}

fn make_unstaker_xorca_ata_invalid_owner_in_data(
    ctx: &mut TestContext,
    unstaker_xorca_ata: Pubkey,
) {
    ctx.write_account(
        unstaker_xorca_ata,
        TOKEN_PROGRAM_ID,
        token_account_data!(
            mint => XORCA_ID,
            owner => Pubkey::new_unique(), // Incorrect owner
            amount => 10_000_000_000,
        ),
    )
    .unwrap();
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
            padding2 => [0; 2024],
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
        SYSTEM_PROGRAM_ID,
        // Initializing with default data as if it were a new account for the first time
        pending_withdraw_data!(
            withdrawable_orca_amount => 0,
            withdrawable_timestamp => 0,
            padding1 => [0; 7],
            padding2 => [0; 2024],
        ),
    )
    .unwrap();
    invalid_pending_withdraw_account
}

// --- Test Functions (continued) ---

#[test]
fn test_unstake_success_1_1_exchange() {
    let mut ctx = TestContext::new();
    let withdraw_index = 0;
    let (
        state_account,
        vault_account,
        unstaker_xorca_ata,
        unstaker_signer,
        pending_withdraw_account,
    ) = setup_base_unstake_context(&mut ctx, withdraw_index);
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

#[test]
fn test_unstake_success_more_than_1_1_exchange() {
    let mut ctx = TestContext::new();
    let withdraw_index = 2;
    let (
        state_account,
        vault_account,
        unstaker_xorca_ata,
        unstaker_signer,
        pending_withdraw_account,
    ) = setup_base_unstake_context(&mut ctx, withdraw_index);
    set_balances_for_more_than_1_1_exchange(
        &mut ctx,
        state_account,
        vault_account,
        unstaker_xorca_ata,
        unstaker_signer,
    );
    let unstake_amount = 58_238_823_121; // 58.238823121 xORCA
    let mut initial_clock = ctx.svm.get_sysvar::<Clock>();
    let current_timestamp = 1752397740;
    initial_clock.unix_timestamp = current_timestamp;
    ctx.svm.set_sysvar::<Clock>(&initial_clock);
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

    assert_eq!(
        vault_account_after.data.amount, 84_934_688_959_145,
        "Vault account should have 84,934,688.959145 ORCA"
    );
    assert_eq!(
        unstaker_xorca_ata_after.data.amount, 37_372_605_363,
        "Staker xOrca ATA should have 37.372605363 xORCA"
    );
    assert_eq!(
        xorca_mint_account_after.data.supply, 53_854_425_054_116_118,
        "xOrca supply should be 53854425.054116118 xORCA"
    );
    assert_eq!(
        state_account_after.data.escrowed_orca_amount, 85_378_602_918,
        "Escrowed Orca amount should be 85,378.602918 ORCA"
    );
    let pending_withdraw_after = ctx
        .get_account::<PendingWithdraw>(pending_withdraw_account)
        .unwrap();
    assert_eq!(
        pending_withdraw_after.data.withdrawable_orca_amount, 91_757_064,
        "Pending withdraw amount should be 91.757064 ORCA"
    );
    assert_eq!(
        pending_withdraw_after.data.withdrawable_timestamp,
        current_timestamp + 30 * 24 * 60 * 60,
        "Pending withdraw timestamp should be 30 days from now"
    );
    assert_eq!(pending_withdraw_after.data.padding1, [0; 7]);
    assert_eq!(pending_withdraw_after.data.padding2, [0; 2024]);
}

/// --- Unstake Invalid Account Tests ---

/// ### State Account Tests

#[test]
fn test_unstake_invalid_state_account_owner() {
    let mut ctx = TestContext::new();
    let withdraw_index = 0;
    let (
        state_account,
        vault_account,
        unstaker_xorca_ata,
        unstaker_signer,
        pending_withdraw_account,
    ) = setup_base_unstake_context(&mut ctx, withdraw_index);
    let unstake_amount = 10_000_000_000;

    make_state_account_invalid_owner(&mut ctx, state_account); // Make state account have wrong owner

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
    assert_program_error!(result, XorcaStakingProgramError::IncorrectOwner);
}

#[test]
fn test_unstake_invalid_state_account_seeds() {
    let mut ctx = TestContext::new();
    let withdraw_index = 0;
    let (_, vault_account, unstaker_xorca_ata, unstaker_signer, pending_withdraw_account) =
        setup_base_unstake_context(&mut ctx, withdraw_index);
    let unstake_amount = 10_000_000_000;
    let invalid_state_account = make_state_account_invalid_seeds(&mut ctx);
    let ix = Unstake {
        unstaker_account: unstaker_signer,
        state_account: invalid_state_account,
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
    assert_program_error!(result, XorcaStakingProgramError::InvalidSeeds);
}

/// ### Vault Account Tests

#[test]
fn test_unstake_invalid_vault_account_owner() {
    let mut ctx = TestContext::new();
    let withdraw_index = 0;
    let (
        state_account,
        vault_account,
        unstaker_xorca_ata,
        unstaker_signer,
        pending_withdraw_account,
    ) = setup_base_unstake_context(&mut ctx, withdraw_index);
    let unstake_amount = 10_000_000_000;
    make_vault_account_invalid_owner(&mut ctx, vault_account);
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
    assert_program_error!(result, XorcaStakingProgramError::IncorrectOwner);
}

#[test]
fn test_unstake_invalid_vault_account_mint_in_data() {
    let mut ctx = TestContext::new();
    let withdraw_index = 0;
    let (
        state_account,
        vault_account,
        unstaker_xorca_ata,
        unstaker_signer,
        pending_withdraw_account,
    ) = setup_base_unstake_context(&mut ctx, withdraw_index);
    let unstake_amount = 10_000_000_000;
    make_vault_account_invalid_mint_in_data(&mut ctx, vault_account);
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
    assert_program_error!(result, XorcaStakingProgramError::InvalidAccountData);
}

#[test]
fn test_unstake_invalid_vault_account_owner_in_data() {
    let mut ctx = TestContext::new();
    let withdraw_index = 0;
    let (
        state_account,
        vault_account,
        unstaker_xorca_ata,
        unstaker_signer,
        pending_withdraw_account,
    ) = setup_base_unstake_context(&mut ctx, withdraw_index);
    let unstake_amount = 10_000_000_000;
    make_vault_account_invalid_owner_in_data(&mut ctx, vault_account); // Make vault account have wrong owner in data
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
    assert_program_error!(result, XorcaStakingProgramError::InvalidAccountData);
}

/// ### xORCA Mint Account Tests

#[test]
fn test_unstake_invalid_xorca_mint_owner() {
    let mut ctx = TestContext::new();
    let withdraw_index = 0;
    let (
        state_account,
        vault_account,
        unstaker_xorca_ata,
        unstaker_signer,
        pending_withdraw_account,
    ) = setup_base_unstake_context(&mut ctx, withdraw_index);
    let unstake_amount = 10_000_000_000;
    make_xorca_mint_invalid_owner(&mut ctx); // Make xORCA mint have wrong owner
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
    assert_program_error!(result, XorcaStakingProgramError::IncorrectOwner);
}

/// ### ORCA Mint Account Tests
#[test]
fn test_unstake_invalid_orca_mint_owner() {
    let mut ctx = TestContext::new();
    let withdraw_index = 0;
    let (
        state_account,
        vault_account,
        unstaker_xorca_ata,
        unstaker_signer,
        pending_withdraw_account,
    ) = setup_base_unstake_context(&mut ctx, withdraw_index);
    let unstake_amount = 10_000_000_000;
    make_orca_mint_invalid_owner(&mut ctx); // Make ORCA mint have wrong owner
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
    assert_program_error!(result, XorcaStakingProgramError::IncorrectOwner);
}

/// ### Unstaker xORCA ATA Tests
#[test]
fn test_unstake_invalid_unstaker_xorca_ata_program_owner() {
    let mut ctx = TestContext::new();
    let withdraw_index = 0;
    let (
        state_account,
        vault_account,
        unstaker_xorca_ata,
        unstaker_signer,
        pending_withdraw_account,
    ) = setup_base_unstake_context(&mut ctx, withdraw_index);
    let unstake_amount = 10_000_000_000;
    make_unstaker_xorca_ata_invalid_program_owner(&mut ctx, unstaker_xorca_ata); // Make unstaker xORCA ATA have wrong program owner
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
    assert_program_error!(result, XorcaStakingProgramError::IncorrectOwner);
}

#[test]
fn test_unstake_invalid_unstaker_xorca_ata_mint_in_data() {
    let mut ctx = TestContext::new();
    let withdraw_index = 0;
    let (
        state_account,
        vault_account,
        unstaker_xorca_ata,
        unstaker_signer,
        pending_withdraw_account,
    ) = setup_base_unstake_context(&mut ctx, withdraw_index);
    let unstake_amount = 10_000_000_000;

    make_unstaker_xorca_ata_invalid_mint_in_data(&mut ctx, unstaker_xorca_ata); // Make unstaker xORCA ATA have wrong mint in data

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
    assert_program_error!(result, XorcaStakingProgramError::InvalidAccountData);
}

#[test]
fn test_unstake_invalid_unstaker_xorca_ata_owner_in_data() {
    let mut ctx = TestContext::new();
    let withdraw_index = 0;
    let (
        state_account,
        vault_account,
        unstaker_xorca_ata,
        unstaker_signer,
        pending_withdraw_account,
    ) = setup_base_unstake_context(&mut ctx, withdraw_index);
    let unstake_amount = 10_000_000_000;
    make_unstaker_xorca_ata_invalid_owner_in_data(&mut ctx, unstaker_xorca_ata); // Make unstaker xORCA ATA have wrong owner in data
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
    assert_program_error!(result, XorcaStakingProgramError::InvalidAccountData);
}

/// ### Pending Withdraw Account Tests
#[test]
fn test_unstake_invalid_pending_withdraw_account_owner() {
    let mut ctx = TestContext::new();
    let withdraw_index = 0;
    let (
        state_account,
        vault_account,
        unstaker_xorca_ata,
        unstaker_signer,
        pending_withdraw_account,
    ) = setup_base_unstake_context(&mut ctx, withdraw_index);
    let unstake_amount = 10_000_000_000;
    make_pending_withdraw_invalid_owner(&mut ctx, pending_withdraw_account); // Make pending withdraw account have wrong owner
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
    assert_program_error!(result, XorcaStakingProgramError::IncorrectOwner);
}

#[test]
fn test_unstake_invalid_pending_withdraw_account_seeds() {
    let mut ctx = TestContext::new();
    let withdraw_index = 0;
    let (state_account, vault_account, unstaker_xorca_ata, unstaker_signer, _) =
        setup_base_unstake_context(&mut ctx, withdraw_index);
    let unstake_amount = 10_000_000_000;
    let invalid_pending_withdraw_account =
        make_pending_withdraw_invalid_seeds(&mut ctx, unstaker_signer);
    let ix = Unstake {
        unstaker_account: unstaker_signer,
        state_account,
        vault_account,
        pending_withdraw_account: invalid_pending_withdraw_account,
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
    assert_program_error!(result, XorcaStakingProgramError::InvalidSeeds);
}

/// ### Invalid Program ID Tests
#[test]
fn test_unstake_invalid_system_program_id() {
    let mut ctx = TestContext::new();
    let withdraw_index = 0;
    let (
        state_account,
        vault_account,
        unstaker_xorca_ata,
        unstaker_signer,
        pending_withdraw_account,
    ) = setup_base_unstake_context(&mut ctx, withdraw_index);
    let unstake_amount = 10_000_000_000;

    let invalid_system_program_id = Pubkey::new_unique();
    let ix = Unstake {
        unstaker_account: unstaker_signer,
        state_account,
        vault_account,
        pending_withdraw_account,
        unstaker_xorca_ata,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        system_program_account: invalid_system_program_id, // Pass the invalid system program ID
        token_program_account: TOKEN_PROGRAM_ID,
    }
    .instruction(UnstakeInstructionArgs {
        unstake_amount,
        withdraw_index,
    });
    let result = ctx.send(ix);
    assert_program_error!(result, XorcaStakingProgramError::IncorrectAccountAddress);
}

#[test]
fn test_unstake_invalid_token_program_id() {
    let mut ctx = TestContext::new();
    let withdraw_index = 0;
    let (
        state_account,
        vault_account,
        unstaker_xorca_ata,
        unstaker_signer,
        pending_withdraw_account,
    ) = setup_base_unstake_context(&mut ctx, withdraw_index);
    let unstake_amount = 10_000_000_000;

    let invalid_token_program_id = Pubkey::new_unique();
    let ix = Unstake {
        unstaker_account: unstaker_signer,
        state_account,
        vault_account,
        pending_withdraw_account,
        unstaker_xorca_ata,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        system_program_account: SYSTEM_PROGRAM_ID,
        token_program_account: invalid_token_program_id, // Pass the invalid token program ID
    }
    .instruction(UnstakeInstructionArgs {
        unstake_amount,
        withdraw_index,
    });
    let result = ctx.send(ix);
    assert_program_error!(result, XorcaStakingProgramError::IncorrectAccountAddress);
}
