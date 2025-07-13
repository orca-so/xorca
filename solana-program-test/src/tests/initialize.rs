use crate::{
    assert_program_error, assert_program_success, state_data, token_mint_data, TestContext,
    ORCA_ID, SYSTEM_PROGRAM_ID, TOKEN_PROGRAM_ID, XORCA_ID,
};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signer::Signer; // Used for ctx.signer()
use xorca::{
    find_state_address, AccountDiscriminator, Initialize, InitializeInstructionArgs, State,
    TokenMint, XorcaStakingProgramError, DEFAULT_ACCOUNT_LEN,
};

const INITIAL_UPDATE_AUTHORITY_ID: Pubkey = solana_sdk::pubkey!("11111111111111111111111111111111");

// Sets up the basic valid context for Initialize tests.
fn setup_base_initialize_context(ctx: &mut TestContext) -> Pubkey {
    let (state_account, _) = find_state_address().unwrap();
    ctx.write_account(
        XORCA_ID,
        TOKEN_PROGRAM_ID,
        token_mint_data!(
            supply => 0,
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
            supply => 1_000_000_000,
            decimals => 6,
            mint_authority_flag => 1,
            mint_authority => Pubkey::default(),
            is_initialized => true,
            freeze_authority_flag => 0,
            freeze_authority => Pubkey::default(),
        ),
    )
    .unwrap();
    state_account
}

// --- Helper Functions for Invalid Account Scenarios ---
fn make_state_already_initialized(ctx: &mut TestContext, state_account: Pubkey) {
    ctx.write_account(
        state_account,
        xorca::ID,
        state_data!(
            discriminator => AccountDiscriminator::State,
            cool_down_period_s => 50,
            update_authority => Pubkey::default(),
            escrowed_orca_amount => 0,
        ),
    )
    .unwrap();
}

fn make_xorca_mint_invalid_owner(ctx: &mut TestContext) {
    ctx.write_account(
        XORCA_ID,
        SYSTEM_PROGRAM_ID, // Wrong owner
        token_mint_data!(
            supply => 0, decimals => 9, mint_authority_flag => 1, mint_authority => Pubkey::new_unique(),
            is_initialized => true, freeze_authority_flag => 0, freeze_authority => Pubkey::default(),
        ),
    )
    .unwrap();
}

fn make_xorca_mint_frozen(ctx: &mut TestContext, state_account: Pubkey) {
    ctx.write_account(
        XORCA_ID,
        TOKEN_PROGRAM_ID,
        token_mint_data!(
            supply => 0, decimals => 9, mint_authority_flag => 1, mint_authority => state_account,
            is_initialized => true, freeze_authority_flag => 1, // Has freeze authority
            freeze_authority => Pubkey::default(),
        ),
    )
    .unwrap();
}

fn make_xorca_mint_already_initialized_supply(ctx: &mut TestContext, state_account: Pubkey) {
    ctx.write_account(
        XORCA_ID,
        TOKEN_PROGRAM_ID,
        token_mint_data!(
            supply => 1_000_000, // Non-zero supply
            decimals => 9, mint_authority_flag => 1, mint_authority => state_account,
            is_initialized => true, freeze_authority_flag => 0, freeze_authority => Pubkey::default(),
        ),
    )
    .unwrap();
}

fn make_orca_mint_invalid_owner(ctx: &mut TestContext) {
    ctx.write_account(
        ORCA_ID,
        SYSTEM_PROGRAM_ID, // Wrong owner
        token_mint_data!(
            supply => 1_000_000_000, decimals => 6, mint_authority_flag => 1, mint_authority => Pubkey::default(),
            is_initialized => true, freeze_authority_flag => 0, freeze_authority => Pubkey::default(),
        ),
    )
    .unwrap();
}

fn make_xorca_mint_invalid_authority(ctx: &mut TestContext) {
    ctx.write_account(
        XORCA_ID,
        TOKEN_PROGRAM_ID,
        token_mint_data!(
            supply => 0, decimals => 9, mint_authority_flag => 1,
            mint_authority => Pubkey::new_unique(), // Incorrect mint authority
            is_initialized => true, freeze_authority_flag => 0, freeze_authority => Pubkey::default(),
        ),
    )
    .unwrap();
}

fn make_xorca_mint_no_authority_flag(ctx: &mut TestContext, state_account: Pubkey) {
    ctx.write_account(
        XORCA_ID,
        TOKEN_PROGRAM_ID,
        token_mint_data!(
            supply => 0, decimals => 9, mint_authority_flag => 0, // No mint authority flag
            mint_authority => state_account, // Still set for completeness, but flag overrides
            is_initialized => true, freeze_authority_flag => 0, freeze_authority => Pubkey::default(),
        ),
    )
    .unwrap();
}

/// Test 1a: Initialize Staking Pool with valid parameters
#[test]
fn test_initialize_success() {
    let mut ctx = TestContext::new();
    let cool_down_period_s: i64 = 100;
    let state_account = setup_base_initialize_context(&mut ctx);
    let ix = Initialize {
        payer_account: ctx.signer(),
        state_account,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        update_authority_account: INITIAL_UPDATE_AUTHORITY_ID,
        system_program_account: SYSTEM_PROGRAM_ID,
    }
    .instruction(InitializeInstructionArgs { cool_down_period_s });
    let result = ctx.send(ix);
    assert_program_success!(result);
    let state_account_after = ctx.get_account::<State>(state_account).unwrap();
    assert_eq!(
        state_account_after.data.discriminator,
        AccountDiscriminator::State,
        "State account discriminator should be State"
    );
    assert_eq!(
        state_account_after.data.cool_down_period_s, cool_down_period_s,
        "Cool down period should be 100"
    );
    assert_eq!(
        state_account_after.data.update_authority, INITIAL_UPDATE_AUTHORITY_ID,
        "Update authority should be the initial upgrade authority"
    );
    assert_eq!(
        state_account_after.data.escrowed_orca_amount, 0,
        "Escrowed orca amount should be 0"
    );
    assert_eq!(state_account_after.data.padding1, [0; 7]);
    assert_eq!(state_account_after.data.padding2, [0; 1992]);
    let xorca_mint_account_after = ctx.get_account::<TokenMint>(XORCA_ID).unwrap();
    assert_eq!(xorca_mint_account_after.data.supply, 0);
    assert_eq!(xorca_mint_account_after.data.decimals, 9);
    assert_eq!(xorca_mint_account_after.data.mint_authority_flag, 1);
    assert_eq!(xorca_mint_account_after.data.mint_authority, state_account);
    assert_eq!(xorca_mint_account_after.data.is_initialized, true);
    assert_eq!(xorca_mint_account_after.data.freeze_authority_flag, 0);
}

/// Test 2: Staking pool has already been initialized for this contract
#[test]
fn test_initialize_state_already_exists() {
    let mut ctx = TestContext::new();
    let cool_down_period_s: i64 = 7 * 24 * 60 * 60;
    let state_account = setup_base_initialize_context(&mut ctx);
    make_state_already_initialized(&mut ctx, state_account);
    let ix = Initialize {
        payer_account: ctx.signer(),
        state_account,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        update_authority_account: INITIAL_UPDATE_AUTHORITY_ID,
        system_program_account: SYSTEM_PROGRAM_ID,
    }
    .instruction(InitializeInstructionArgs { cool_down_period_s });
    let result = ctx.send(ix);
    assert_program_error!(
        result,
        XorcaStakingProgramError::StateAccountAlreadyInitialized
    );
}

/// Test 3a: xOrca token mint account is not a valid mint account (wrong owner)
#[test]
fn test_initialize_invalid_xorca_mint_account_owner() {
    let mut ctx = TestContext::new();
    let cool_down_period_s: i64 = 100;
    let state_account = setup_base_initialize_context(&mut ctx);
    make_xorca_mint_invalid_owner(&mut ctx);
    let ix = Initialize {
        payer_account: ctx.signer(),
        state_account,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        update_authority_account: INITIAL_UPDATE_AUTHORITY_ID,
        system_program_account: SYSTEM_PROGRAM_ID,
    }
    .instruction(InitializeInstructionArgs { cool_down_period_s });
    let result = ctx.send(ix);
    assert_program_error!(result, XorcaStakingProgramError::IncorrectOwner);
}

/// Test 3b: xOrca token mint has already been frozen (freeze authority set)
#[test]
fn test_initialize_xorca_mint_frozen() {
    let mut ctx = TestContext::new();
    let cool_down_period_s: i64 = 100;
    let state_account = setup_base_initialize_context(&mut ctx);
    make_xorca_mint_frozen(&mut ctx, state_account);
    let ix = Initialize {
        payer_account: ctx.signer(),
        state_account,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        update_authority_account: INITIAL_UPDATE_AUTHORITY_ID,
        system_program_account: SYSTEM_PROGRAM_ID,
    }
    .instruction(InitializeInstructionArgs { cool_down_period_s });
    let result = ctx.send(ix);
    assert_program_error!(result, XorcaStakingProgramError::InvalidAccountData);
}

/// Test 3c: xOrca token mint has already been initialized (non-zero supply)
#[test]
fn test_initialize_xorca_mint_already_initialized_supply() {
    let mut ctx = TestContext::new();
    let cool_down_period_s: i64 = 100;
    let state_account = setup_base_initialize_context(&mut ctx);
    make_xorca_mint_already_initialized_supply(&mut ctx, state_account);
    let ix = Initialize {
        payer_account: ctx.signer(),
        state_account,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        update_authority_account: INITIAL_UPDATE_AUTHORITY_ID,
        system_program_account: SYSTEM_PROGRAM_ID,
    }
    .instruction(InitializeInstructionArgs { cool_down_period_s });
    let result = ctx.send(ix);
    assert_program_error!(result, XorcaStakingProgramError::InvalidAccountData);
}

/// Test 3d: Orca token is not a valid mint account (wrong owner)
#[test]
fn test_initialize_invalid_orca_mint_account_owner() {
    let mut ctx = TestContext::new();
    let cool_down_period_s: i64 = 100;
    let state_account = setup_base_initialize_context(&mut ctx);
    make_orca_mint_invalid_owner(&mut ctx);
    let ix = Initialize {
        payer_account: ctx.signer(),
        state_account,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        update_authority_account: INITIAL_UPDATE_AUTHORITY_ID,
        system_program_account: SYSTEM_PROGRAM_ID,
    }
    .instruction(InitializeInstructionArgs { cool_down_period_s });
    let result = ctx.send(ix);
    assert_program_error!(result, XorcaStakingProgramError::IncorrectOwner);
}

/// Test 4: User provided a xOrca token mint account that they do not have authority over
#[test]
fn test_initialize_invalid_xorca_mint_authority() {
    let mut ctx = TestContext::new();
    let cool_down_period_s: i64 = 100;
    let state_account = setup_base_initialize_context(&mut ctx);
    make_xorca_mint_invalid_authority(&mut ctx);
    let ix = Initialize {
        payer_account: ctx.signer(),
        state_account,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        update_authority_account: INITIAL_UPDATE_AUTHORITY_ID,
        system_program_account: SYSTEM_PROGRAM_ID,
    }
    .instruction(InitializeInstructionArgs { cool_down_period_s });
    let result = ctx.send(ix);
    assert_program_error!(result, XorcaStakingProgramError::IncorrectAccountAddress);
}

/// Test 5: Invalid system account
#[test]
fn test_initialize_invalid_system_account() {
    let mut ctx = TestContext::new();
    let cool_down_period_s: i64 = 100;
    let state_account = setup_base_initialize_context(&mut ctx);
    let ix = Initialize {
        payer_account: ctx.signer(),
        state_account,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        update_authority_account: INITIAL_UPDATE_AUTHORITY_ID,
        system_program_account: TOKEN_PROGRAM_ID, // Wrong system program
    }
    .instruction(InitializeInstructionArgs { cool_down_period_s });
    let result = ctx.send(ix);
    assert_program_error!(result, XorcaStakingProgramError::IncorrectAccountAddress);
}

/// Test 6: Insufficient lamports for rent for the staking pool account / mint initialization
#[test]
fn test_initialize_insufficient_lamports() {
    let cool_down_period_s: i64 = 100;
    let mut poor_payer_ctx = TestContext::new();
    poor_payer_ctx
        .svm
        .set_account(
            poor_payer_ctx.signer.pubkey(),
            solana_sdk::account::Account {
                lamports: 1000, // Only 1000 lamports for poor payer
                owner: solana_sdk::system_program::ID,
                executable: false,
                rent_epoch: 0,
                data: vec![],
            },
        )
        .unwrap();
    let state_account = setup_base_initialize_context(&mut poor_payer_ctx);
    let ix = Initialize {
        payer_account: poor_payer_ctx.signer(),
        state_account,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        update_authority_account: INITIAL_UPDATE_AUTHORITY_ID,
        system_program_account: TOKEN_PROGRAM_ID, // Wrong system program
    }
    .instruction(InitializeInstructionArgs { cool_down_period_s });
    let result = poor_payer_ctx.send(ix);
    assert!(result.is_err(), "Should fail with insufficient funds");
}

/// Test 7:  Test invalid update authority (provided public key is not system program ID)
#[test]
fn test_initialize_invalid_update_authority_address() {
    let mut ctx = TestContext::new();
    let cool_down_period_s: i64 = 100;
    let state_account = setup_base_initialize_context(&mut ctx);
    let ix = Initialize {
        payer_account: ctx.signer(),
        state_account,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        update_authority_account: TOKEN_PROGRAM_ID,
        system_program_account: TOKEN_PROGRAM_ID, // Wrong system program
    }
    .instruction(InitializeInstructionArgs { cool_down_period_s });
    let result = ctx.send(ix);
    assert_program_error!(result, XorcaStakingProgramError::IncorrectAccountAddress);
}

/// Test 8:  Test xOrca mint authority flag is 0 (no mint authority)
#[test]
fn test_initialize_xorca_mint_no_authority_flag() {
    let mut ctx = TestContext::new();
    let cool_down_period_s: i64 = 100;
    let state_account = setup_base_initialize_context(&mut ctx);
    make_xorca_mint_no_authority_flag(&mut ctx, state_account);
    let ix = Initialize {
        payer_account: ctx.signer(),
        state_account,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        update_authority_account: INITIAL_UPDATE_AUTHORITY_ID,
        system_program_account: SYSTEM_PROGRAM_ID,
    }
    .instruction(InitializeInstructionArgs { cool_down_period_s });
    let result = ctx.send(ix);
    assert_program_error!(result, XorcaStakingProgramError::InvalidAccountData);
}
