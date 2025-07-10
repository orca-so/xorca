use crate::{
    assert_program_error, assert_program_success, state_data, token_mint_data, TestContext,
    ORCA_ID, SYSTEM_PROGRAM_ID, TOKEN_PROGRAM_ID, XORCA_ID,
};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signer::Signer;
use xorca::{
    find_state_address, AccountDiscriminator, Initialize, InitializeInstructionArgs, State,
    XorcaStakingProgramError,
};

const INITIAL_UPGRADE_AUTHORITY_ID: Pubkey =
    solana_sdk::pubkey!("11111111111111111111111111111111");

/// Test 1a: Initialize Staking Pool with valid parameters
/// - staking pool account is initialized and has the expected data
/// - xOrca token mint is initialized at decimal and lst token freeze authority is not null
#[test]
fn test_initialize_success() {
    let mut ctx = TestContext::new();
    let (state_account, _) = find_state_address().unwrap();
    let cool_down_period_s: u64 = 100;

    // Write xOrca mint account with valid data
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

    // Write Orca mint account
    ctx.write_account(
        ORCA_ID,
        TOKEN_PROGRAM_ID,
        token_mint_data!(
            supply => 1000000000,
            decimals => 6,
            mint_authority_flag => 1,
            mint_authority => Pubkey::default(),
            is_initialized => true,
            freeze_authority_flag => 0,
            freeze_authority => Pubkey::default(),
        ),
    )
    .unwrap();

    // Define instruction
    let ix: solana_sdk::instruction::Instruction = Initialize {
        payer_account: ctx.signer(),
        state_account,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        update_authority_account: INITIAL_UPGRADE_AUTHORITY_ID,
        system_program_account: SYSTEM_PROGRAM_ID,
    }
    .instruction(InitializeInstructionArgs { cool_down_period_s });

    // Execute instruction
    let result = ctx.send(ix);
    assert_program_success!(result);

    // Print raw account data (first 64 bytes)
    let raw_account = ctx.get_raw_account(state_account).unwrap();
    println!(
        "Raw account data (first 64 bytes): {:?}",
        &raw_account.data[..64]
    );

    let state_account_after = ctx.get_account::<State>(state_account).unwrap();
    println!("Decoded State from client: {:?}", state_account_after.data);

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
        state_account_after.data.update_authority, INITIAL_UPGRADE_AUTHORITY_ID,
        "Update authority should be the initial upgrade authority"
    );
    assert_eq!(
        state_account_after.data.escrowed_orca_amount, 0,
        "Escrowed orca amount should be 0"
    );
}

/// Test 2: Staking pool has already been initialized for this contract
#[test]
fn test_initialize_state_already_exists() {
    let mut ctx = TestContext::new();
    let (state_account, _) = find_state_address().unwrap();
    let cool_down_period_s: u64 = 100;

    // Pre-initialize the state account
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

    // Write xOrca mint account
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

    // Write Orca mint account
    ctx.write_account(
        ORCA_ID,
        TOKEN_PROGRAM_ID,
        token_mint_data!(
            supply => 1000000000,
            decimals => 6,
            mint_authority_flag => 1,
            mint_authority => Pubkey::default(),
            is_initialized => true,
            freeze_authority_flag => 0,
            freeze_authority => Pubkey::default(),
        ),
    )
    .unwrap();

    // Define instruction
    let ix: solana_sdk::instruction::Instruction = Initialize {
        payer_account: ctx.signer(),
        state_account,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        update_authority_account: INITIAL_UPGRADE_AUTHORITY_ID,
        system_program_account: SYSTEM_PROGRAM_ID,
    }
    .instruction(InitializeInstructionArgs { cool_down_period_s });

    // Execute instruction - should fail
    let result = ctx.send(ix);
    assert_program_error!(
        result,
        XorcaStakingProgramError::StateAccountAlreadyInitialized
    );
}

/// Test 3a: xOrca token mint account is not a valid mint account
#[test]
fn test_initialize_invalid_xorca_mint_account() {
    let mut ctx = TestContext::new();
    let (state_account, _) = find_state_address().unwrap();
    let cool_down_period_s: u64 = 100;

    // Write invalid xOrca mint account (not owned by token program)
    ctx.write_account(
        XORCA_ID,
        SYSTEM_PROGRAM_ID, // Wrong owner
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

    // Write Orca mint account
    ctx.write_account(
        ORCA_ID,
        TOKEN_PROGRAM_ID,
        token_mint_data!(
            supply => 1000000000,
            decimals => 6,
            mint_authority_flag => 1,
            mint_authority => Pubkey::default(),
            is_initialized => true,
            freeze_authority_flag => 0,
            freeze_authority => Pubkey::default(),
        ),
    )
    .unwrap();

    // Define instruction
    let ix: solana_sdk::instruction::Instruction = Initialize {
        payer_account: ctx.signer(),
        state_account,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        update_authority_account: INITIAL_UPGRADE_AUTHORITY_ID,
        system_program_account: SYSTEM_PROGRAM_ID,
    }
    .instruction(InitializeInstructionArgs { cool_down_period_s });

    // Execute instruction - should fail
    let result = ctx.send(ix);
    assert_program_error!(result, XorcaStakingProgramError::IncorrectOwner);
}

/// Test 3b: xOrca token mint has already been frozen
#[test]
fn test_initialize_xorca_mint_frozen() {
    let mut ctx = TestContext::new();
    let (state_account, _) = find_state_address().unwrap();
    let cool_down_period_s: u64 = 100;

    // Write xOrca mint account with freeze authority set (frozen)
    ctx.write_account(
        XORCA_ID,
        TOKEN_PROGRAM_ID,
        token_mint_data!(
            supply => 0,
            decimals => 9,
            mint_authority_flag => 1,
            mint_authority => state_account,
            is_initialized => true,
            freeze_authority_flag => 1, // Has freeze authority
            freeze_authority => Pubkey::default(),
        ),
    )
    .unwrap();

    // Write Orca mint account
    ctx.write_account(
        ORCA_ID,
        TOKEN_PROGRAM_ID,
        token_mint_data!(
            supply => 1000000000,
            decimals => 6,
            mint_authority_flag => 1,
            mint_authority => Pubkey::default(),
            is_initialized => true,
            freeze_authority_flag => 0,
            freeze_authority => Pubkey::default(),
        ),
    )
    .unwrap();

    // Define instruction
    let ix: solana_sdk::instruction::Instruction = Initialize {
        payer_account: ctx.signer(),
        state_account,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        update_authority_account: INITIAL_UPGRADE_AUTHORITY_ID,
        system_program_account: SYSTEM_PROGRAM_ID,
    }
    .instruction(InitializeInstructionArgs { cool_down_period_s });

    // Execute instruction - should fail
    let result = ctx.send(ix);
    assert_program_error!(result, XorcaStakingProgramError::InvalidAccountData);
}

/// Test 3c: xOrca token mint has already been initialized (non-zero supply)
#[test]
fn test_initialize_xorca_mint_already_initialized() {
    let mut ctx = TestContext::new();
    let (state_account, _) = find_state_address().unwrap();
    let cool_down_period_s: u64 = 100;

    // Write xOrca mint account with non-zero supply
    ctx.write_account(
        XORCA_ID,
        TOKEN_PROGRAM_ID,
        token_mint_data!(
            supply => 1000000, // Non-zero supply
            decimals => 9,
            mint_authority_flag => 1,
            mint_authority => state_account,
            is_initialized => true,
            freeze_authority_flag => 0,
            freeze_authority => Pubkey::default(),
        ),
    )
    .unwrap();

    // Write Orca mint account
    ctx.write_account(
        ORCA_ID,
        TOKEN_PROGRAM_ID,
        token_mint_data!(
            supply => 1000000000,
            decimals => 6,
            mint_authority_flag => 1,
            mint_authority => Pubkey::default(),
            is_initialized => true,
            freeze_authority_flag => 0,
            freeze_authority => Pubkey::default(),
        ),
    )
    .unwrap();

    // Define instruction
    let ix: solana_sdk::instruction::Instruction = Initialize {
        payer_account: ctx.signer(),
        state_account,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        update_authority_account: INITIAL_UPGRADE_AUTHORITY_ID,
        system_program_account: SYSTEM_PROGRAM_ID,
    }
    .instruction(InitializeInstructionArgs { cool_down_period_s });

    // Execute instruction - should fail
    let result = ctx.send(ix);
    assert_program_error!(result, XorcaStakingProgramError::InvalidAccountData);
}

/// Test 3e: Orca token is tokenKeg, so token account has to be tokenKeg
#[test]
fn test_initialize_invalid_orca_mint_account() {
    let mut ctx = TestContext::new();
    let (state_account, _) = find_state_address().unwrap();
    let cool_down_period_s: u64 = 100;

    // Write xOrca mint account
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

    // Write Orca mint account with wrong owner
    ctx.write_account(
        ORCA_ID,
        SYSTEM_PROGRAM_ID, // Wrong owner
        token_mint_data!(
            supply => 1000000000,
            decimals => 6,
            mint_authority_flag => 1,
            mint_authority => Pubkey::default(),
            is_initialized => true,
            freeze_authority_flag => 0,
            freeze_authority => Pubkey::default(),
        ),
    )
    .unwrap();

    // Define instruction
    let ix: solana_sdk::instruction::Instruction = Initialize {
        payer_account: ctx.signer(),
        state_account,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        update_authority_account: INITIAL_UPGRADE_AUTHORITY_ID,
        system_program_account: SYSTEM_PROGRAM_ID,
    }
    .instruction(InitializeInstructionArgs { cool_down_period_s });

    // Execute instruction - should fail
    let result = ctx.send(ix);
    assert_program_error!(result, XorcaStakingProgramError::IncorrectOwner);
}

/// Test 4: User provided a xOrca token mint account that they do not have authority over
#[test]
fn test_initialize_invalid_xorca_mint_authority() {
    let mut ctx = TestContext::new();
    let (state_account, _) = find_state_address().unwrap();
    let cool_down_period_s: u64 = 100;

    // Write xOrca mint account with wrong mint authority
    ctx.write_account(
        XORCA_ID,
        TOKEN_PROGRAM_ID,
        token_mint_data!(
            supply => 0,
            decimals => 9,
            mint_authority_flag => 1,
            mint_authority => Pubkey::default(), // Wrong mint authority
            is_initialized => true,
            freeze_authority_flag => 0,
            freeze_authority => Pubkey::default(),
        ),
    )
    .unwrap();

    // Write Orca mint account
    ctx.write_account(
        ORCA_ID,
        TOKEN_PROGRAM_ID,
        token_mint_data!(
            supply => 1000000000,
            decimals => 6,
            mint_authority_flag => 1,
            mint_authority => Pubkey::default(),
            is_initialized => true,
            freeze_authority_flag => 0,
            freeze_authority => Pubkey::default(),
        ),
    )
    .unwrap();

    // Define instruction
    let ix: solana_sdk::instruction::Instruction = Initialize {
        payer_account: ctx.signer(),
        state_account,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        update_authority_account: INITIAL_UPGRADE_AUTHORITY_ID,
        system_program_account: SYSTEM_PROGRAM_ID,
    }
    .instruction(InitializeInstructionArgs { cool_down_period_s });

    // Execute instruction - should fail
    let result = ctx.send(ix);
    assert_program_error!(result, XorcaStakingProgramError::IncorrectAccountAddress);
}

/// Test 5: Invalid system account
#[test]
fn test_initialize_invalid_system_account() {
    let mut ctx = TestContext::new();
    let (state_account, _) = find_state_address().unwrap();
    let cool_down_period_s: u64 = 100;

    // Write xOrca mint account
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

    // Write Orca mint account
    ctx.write_account(
        ORCA_ID,
        TOKEN_PROGRAM_ID,
        token_mint_data!(
            supply => 1000000000,
            decimals => 6,
            mint_authority_flag => 1,
            mint_authority => Pubkey::default(),
            is_initialized => true,
            freeze_authority_flag => 0,
            freeze_authority => Pubkey::default(),
        ),
    )
    .unwrap();

    // Define instruction with wrong system program
    let ix: solana_sdk::instruction::Instruction = Initialize {
        payer_account: ctx.signer(),
        state_account,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        update_authority_account: INITIAL_UPGRADE_AUTHORITY_ID,
        system_program_account: TOKEN_PROGRAM_ID, // Wrong system program
    }
    .instruction(InitializeInstructionArgs { cool_down_period_s });

    // Execute instruction - should fail
    let result = ctx.send(ix);
    assert_program_error!(result, XorcaStakingProgramError::IncorrectAccountAddress);
}

/// Test 7: Insufficient lamports for rent for the staking pool account / mint initialization
#[test]
fn test_initialize_insufficient_lamports() {
    let (state_account, _) = find_state_address().unwrap();
    let cool_down_period_s: u64 = 100;

    // Create a new context with minimal lamports
    let mut poor_ctx = TestContext::new();
    // Drain most of the lamports to leave insufficient funds by writing a new account with minimal funds
    poor_ctx
        .svm
        .set_account(
            poor_ctx.signer.pubkey(),
            solana_sdk::account::Account {
                lamports: 1000, // Only 1000 lamports
                owner: solana_sdk::system_program::ID,
                executable: false,
                rent_epoch: 0,
                data: vec![],
            },
        )
        .unwrap();

    // Write xOrca mint account
    poor_ctx
        .write_account(
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

    // Write Orca mint account
    poor_ctx
        .write_account(
            ORCA_ID,
            TOKEN_PROGRAM_ID,
            token_mint_data!(
                supply => 1000000000,
                decimals => 6,
                mint_authority_flag => 1,
                mint_authority => Pubkey::default(),
                is_initialized => true,
                freeze_authority_flag => 0,
                freeze_authority => Pubkey::default(),
            ),
        )
        .unwrap();

    // Define instruction
    let ix: solana_sdk::instruction::Instruction = Initialize {
        payer_account: poor_ctx.signer(),
        state_account,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        update_authority_account: INITIAL_UPGRADE_AUTHORITY_ID,
        system_program_account: SYSTEM_PROGRAM_ID,
    }
    .instruction(InitializeInstructionArgs { cool_down_period_s });

    // Execute instruction - should fail due to insufficient funds
    let result = poor_ctx.send(ix);
    // This should fail with a system error, not a program error
    assert!(result.is_err(), "Should fail with insufficient funds");
}

/// Test invalid update authority
#[test]
fn test_initialize_invalid_update_authority() {
    let mut ctx = TestContext::new();
    let (state_account, _) = find_state_address().unwrap();
    let cool_down_period_s: u64 = 100;

    // Write xOrca mint account
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

    // Write Orca mint account
    ctx.write_account(
        ORCA_ID,
        TOKEN_PROGRAM_ID,
        token_mint_data!(
            supply => 1000000000,
            decimals => 6,
            mint_authority_flag => 1,
            mint_authority => Pubkey::default(),
            is_initialized => true,
            freeze_authority_flag => 0,
            freeze_authority => Pubkey::default(),
        ),
    )
    .unwrap();

    // Define instruction with wrong update authority
    let ix: solana_sdk::instruction::Instruction = Initialize {
        payer_account: ctx.signer(),
        state_account,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        update_authority_account: TOKEN_PROGRAM_ID, // Wrong update authority
        system_program_account: SYSTEM_PROGRAM_ID,
    }
    .instruction(InitializeInstructionArgs { cool_down_period_s });

    // Execute instruction - should fail
    let result = ctx.send(ix);
    assert_program_error!(result, XorcaStakingProgramError::IncorrectAccountAddress);
}

/// Test xOrca mint authority flag is 0 (no mint authority)
#[test]
fn test_initialize_xorca_mint_no_authority() {
    let mut ctx = TestContext::new();
    let (state_account, _) = find_state_address().unwrap();
    let cool_down_period_s: u64 = 100;

    // Write xOrca mint account with no mint authority
    ctx.write_account(
        XORCA_ID,
        TOKEN_PROGRAM_ID,
        token_mint_data!(
            supply => 0,
            decimals => 9,
            mint_authority_flag => 0, // No mint authority
            mint_authority => state_account,
            is_initialized => true,
            freeze_authority_flag => 0,
            freeze_authority => Pubkey::default(),
        ),
    )
    .unwrap();

    // Write Orca mint account
    ctx.write_account(
        ORCA_ID,
        TOKEN_PROGRAM_ID,
        token_mint_data!(
            supply => 1000000000,
            decimals => 6,
            mint_authority_flag => 1,
            mint_authority => Pubkey::default(),
            is_initialized => true,
            freeze_authority_flag => 0,
            freeze_authority => Pubkey::default(),
        ),
    )
    .unwrap();

    // Define instruction
    let ix: solana_sdk::instruction::Instruction = Initialize {
        payer_account: ctx.signer(),
        state_account,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        update_authority_account: INITIAL_UPGRADE_AUTHORITY_ID,
        system_program_account: SYSTEM_PROGRAM_ID,
    }
    .instruction(InitializeInstructionArgs { cool_down_period_s });

    // Execute instruction - should fail
    let result = ctx.send(ix);
    assert_program_error!(result, XorcaStakingProgramError::InvalidAccountData);
}
