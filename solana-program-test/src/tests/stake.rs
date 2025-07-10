use crate::{
    assert_program_error, assert_program_success, state_data, token_account_data, token_mint_data,
    TestContext, ATA_PROGRAM_ID, ORCA_ID, TOKEN_PROGRAM_ID, XORCA_ID, XORCA_PROGRAM_ID,
};
use solana_sdk::pubkey::Pubkey;
use xorca::{
    find_state_address, Stake, StakeInstructionArgs, State, TokenAccount, TokenMint,
    XorcaStakingProgramError, DEFAULT_ACCOUNT_LEN,
};

/// Test 1a: Stake token for xOrca with valid parameters
/// - stake token is transferred from staker to vault
/// - xORCA is minted to the staker
/// - exchange rate should not change
/// - no escrowed ORCA
/// - 1:1 exchange rate
#[test]
fn test_stake_success_at_1_1_exchange_rate() {
    let mut ctx = TestContext::new();
    let (state_account, _) = find_state_address().unwrap();

    // Write state account
    ctx.write_account(
        state_account,
        XORCA_PROGRAM_ID,
        state_data!(
            escrowed_orca_amount => 0,
            update_authority => Pubkey::default(),
            cool_down_period_s => 7 * 24 * 60 * 60, // 7 days
        ),
    )
    .unwrap();
    ctx.pad_account(state_account, DEFAULT_ACCOUNT_LEN).unwrap();

    // Write xOrca mint account with valid data
    ctx.write_account(
        XORCA_ID,
        TOKEN_PROGRAM_ID,
        token_mint_data!(
            supply => 0,
            decimals => 9,
            mint_authority_flag => 1,
            mint_authority => XORCA_PROGRAM_ID,
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
            supply => 1_000_000_000_000, // 1,000,000 ORCA
            decimals => 6,
            mint_authority_flag => 1,
            mint_authority => Pubkey::default(),
            is_initialized => true,
            freeze_authority_flag => 0,
            freeze_authority => Pubkey::default(),
        ),
    )
    .unwrap();

    // Write Vault Orca ata
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
            amount => 0, // Vault has no ORCA
        ),
    )
    .unwrap();

    // Write staker Orca ata
    let staker_orca_ata = Pubkey::find_program_address(
        &[
            &ctx.signer().to_bytes(),
            &TOKEN_PROGRAM_ID.to_bytes(),
            &ORCA_ID.to_bytes(),
        ],
        &ATA_PROGRAM_ID,
    )
    .0;
    ctx.write_account(
        staker_orca_ata,
        TOKEN_PROGRAM_ID,
        token_account_data!(
            mint => ORCA_ID,
            owner => ctx.signer(),
            amount => 1_000_000, // owns 1 ORCA
        ),
    )
    .unwrap();

    // Write staker xOrca ata
    let staker_xorca_ata = Pubkey::find_program_address(
        &[
            &ctx.signer().to_bytes(),
            &TOKEN_PROGRAM_ID.to_bytes(),
            &XORCA_ID.to_bytes(),
        ],
        &ATA_PROGRAM_ID,
    )
    .0;
    ctx.write_account(
        staker_xorca_ata,
        TOKEN_PROGRAM_ID,
        token_account_data!(
            mint => XORCA_ID,
            owner => ctx.signer(),
            amount => 0, // staker has no xORCA
        ),
    )
    .unwrap();

    // Define instruction
    let ix: solana_sdk::instruction::Instruction = Stake {
        staker_account: ctx.signer(),
        state_account,
        vault_account,
        staker_orca_ata,
        staker_xorca_ata,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
    }
    .instruction(StakeInstructionArgs {
        stake_amount: 1_000_000,
    });

    // Execute instruction
    let result = ctx.send(ix);
    assert_program_success!(result);

    // Check accounts
    let vault_account_after = ctx.get_account::<TokenAccount>(vault_account).unwrap();
    println!(
        "Decoded Vault Account from client: {:?}",
        vault_account_after.data
    );

    let staker_orca_ata_after = ctx.get_account::<TokenAccount>(staker_orca_ata).unwrap();
    println!(
        "Decoded Staker Orca ATA from client: {:?}",
        staker_orca_ata_after.data
    );

    let staker_xorca_ata_after = ctx.get_account::<TokenAccount>(staker_xorca_ata).unwrap();
    println!(
        "Decoded Staker xOrca ATA from client: {:?}",
        staker_xorca_ata_after.data
    );

    let xorca_mint_account_after = ctx.get_account::<TokenMint>(XORCA_ID).unwrap();
    println!(
        "Decoded xOrca Mint Account from client: {:?}",
        staker_xorca_ata_after.data
    );

    let state_account_after = ctx.get_account::<State>(state_account).unwrap();
    println!(
        "Decoded State Account from client: {:?}",
        state_account_after.data
    );

    assert_eq!(
        vault_account_after.data.amount, 1_000_000,
        "Vault account should have 1 ORCA"
    );
    assert_eq!(
        staker_orca_ata_after.data.amount, 0,
        "Staker Orca ATA should have 0 ORCA"
    );
    assert_eq!(
        staker_xorca_ata_after.data.amount, 1_000_000_000,
        "Staker xOrca ATA should have 1 xORCA"
    );
    assert_eq!(
        xorca_mint_account_after.data.supply, 1_000_000_000,
        "xOrca supply should be 1 xORCA"
    );
    assert_eq!(
        state_account_after.data.escrowed_orca_amount, 0,
        "Escrowed Orca amount should be unchanged (0 ORCA)"
    );

    // TODO: Test exchange rate still 1:1 after stake
}

/// Test 1b: Stake token for xOrca with valid parameters
/// - stake token is transferred from staker to vault
/// - xORCA is minted to the staker
/// - exchange rate should not change
/// - escrowed ORCA > 0
/// - 1:x (x > 1) exchange rate
#[test]
fn test_stake_success_at_more_than_1_1_exchange_rate() {
    let mut ctx = TestContext::new();
    let (state_account, _) = find_state_address().unwrap();

    // Write state account
    ctx.write_account(
        state_account,
        XORCA_PROGRAM_ID,
        state_data!(
            escrowed_orca_amount => 39_232_982_923, // 39,232.982923 ORCA
            update_authority => Pubkey::default(),
            cool_down_period_s => 7 * 24 * 60 * 60, // 7 days
        ),
    )
    .unwrap();
    ctx.pad_account(state_account, DEFAULT_ACCOUNT_LEN).unwrap();

    // Write xOrca mint account with valid data
    ctx.write_account(
        XORCA_ID,
        TOKEN_PROGRAM_ID,
        token_mint_data!(
            supply => 358_384_859_821_223, // 358,384.859821223 xORCA
            decimals => 9,
            mint_authority_flag => 1,
            mint_authority => XORCA_PROGRAM_ID,
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
            supply => 1_000_000_000_000, // 1,000,000 ORCA
            decimals => 6,
            mint_authority_flag => 1,
            mint_authority => Pubkey::default(),
            is_initialized => true,
            freeze_authority_flag => 0,
            freeze_authority => Pubkey::default(),
        ),
    )
    .unwrap();

    // Write Vault Orca ata
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
            amount => 923_384_268_587, // Vault has 923,384.268587 ORCA
        ),
    )
    .unwrap();

    // Write staker Orca ata
    let staker_orca_ata = Pubkey::find_program_address(
        &[
            &ctx.signer().to_bytes(),
            &TOKEN_PROGRAM_ID.to_bytes(),
            &ORCA_ID.to_bytes(),
        ],
        &ATA_PROGRAM_ID,
    )
    .0;
    ctx.write_account(
        staker_orca_ata,
        TOKEN_PROGRAM_ID,
        token_account_data!(
            mint => ORCA_ID,
            owner => ctx.signer(),
            amount => 5_123_538, // owns 5.123538 ORCA
        ),
    )
    .unwrap();

    // Write staker xOrca ata
    let staker_xorca_ata = Pubkey::find_program_address(
        &[
            &ctx.signer().to_bytes(),
            &TOKEN_PROGRAM_ID.to_bytes(),
            &XORCA_ID.to_bytes(),
        ],
        &ATA_PROGRAM_ID,
    )
    .0;
    ctx.write_account(
        staker_xorca_ata,
        TOKEN_PROGRAM_ID,
        token_account_data!(
            mint => XORCA_ID,
            owner => ctx.signer(),
            amount => 12_823_658_283, // staker has 12.823658283 xORCA
        ),
    )
    .unwrap();

    // Define instruction
    let ix: solana_sdk::instruction::Instruction = Stake {
        staker_account: ctx.signer(),
        state_account,
        vault_account,
        staker_orca_ata,
        staker_xorca_ata,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
    }
    .instruction(StakeInstructionArgs {
        stake_amount: 2_384_964, // stake 2.384964 ORCA
    });

    // Execute instruction
    let result = ctx.send(ix);
    assert_program_success!(result);

    // Check accounts
    let vault_account_after = ctx.get_account::<TokenAccount>(vault_account).unwrap();
    println!(
        "Decoded Vault Account from client: {:?}",
        vault_account_after.data
    );

    let staker_orca_ata_after = ctx.get_account::<TokenAccount>(staker_orca_ata).unwrap();
    println!(
        "Decoded Staker Orca ATA from client: {:?}",
        staker_orca_ata_after.data
    );

    let staker_xorca_ata_after = ctx.get_account::<TokenAccount>(staker_xorca_ata).unwrap();
    println!(
        "Decoded Staker xOrca ATA from client: {:?}",
        staker_xorca_ata_after.data
    );

    let xorca_mint_account_after = ctx.get_account::<TokenMint>(XORCA_ID).unwrap();
    println!(
        "Decoded xOrca Mint Account from client: {:?}",
        staker_xorca_ata_after.data
    );

    let state_account_after = ctx.get_account::<State>(state_account).unwrap();
    println!(
        "Decoded State Account from client: {:?}",
        state_account_after.data
    );

    assert_eq!(
        vault_account_after.data.amount, 923_386_653_551,
        "Vault account should have 923,386.653551 ORCA"
    );
    assert_eq!(
        staker_orca_ata_after.data.amount, 2_738_574,
        "Staker Orca ATA should have 2.738574 ORCA"
    );
    assert_eq!(
        staker_xorca_ata_after.data.amount, 13_790_387_622,
        "Staker xOrca ATA should have 13.790387622 xORCA"
    );
    assert_eq!(
        xorca_mint_account_after.data.supply, 358_385_826_550_562,
        "xOrca supply should be 358,385.826550562 xORCA"
    );
    assert_eq!(
        state_account_after.data.escrowed_orca_amount, 39_232_982_923,
        "Escrowed Orca amount should be unchanged (39,232.982923 ORCA)"
    );

    // TODO: Test exchange rate still 1:x after stake
}

/// Test 2a: Invalid state account, invalid owner
#[test]
fn test_stake_invalid_state_account_invalid_owner() {
    let mut ctx = TestContext::new();
    let (state_account, _) = find_state_address().unwrap();

    // Write state account
    ctx.write_account(
        state_account,
        TOKEN_PROGRAM_ID, // invalid owner
        state_data!(
            escrowed_orca_amount => 0, // 0 ORCA
            update_authority => Pubkey::default(),
            cool_down_period_s => 7 * 24 * 60 * 60, // 7 days
        ),
    )
    .unwrap();
    ctx.pad_account(state_account, DEFAULT_ACCOUNT_LEN).unwrap();

    // Write xOrca mint account with valid data
    ctx.write_account(
        XORCA_ID,
        TOKEN_PROGRAM_ID,
        token_mint_data!(
            supply => 0,
            decimals => 9,
            mint_authority_flag => 1,
            mint_authority => XORCA_PROGRAM_ID,
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
            supply => 1_000_000_000_000, // 1,000,000 ORCA
            decimals => 6,
            mint_authority_flag => 1,
            mint_authority => Pubkey::default(),
            is_initialized => true,
            freeze_authority_flag => 0,
            freeze_authority => Pubkey::default(),
        ),
    )
    .unwrap();

    // Write Vault Orca ata
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
            amount => 0, // Vault has no ORCA
        ),
    )
    .unwrap();

    // Write staker Orca ata
    let staker_orca_ata = Pubkey::find_program_address(
        &[
            &ctx.signer().to_bytes(),
            &TOKEN_PROGRAM_ID.to_bytes(),
            &ORCA_ID.to_bytes(),
        ],
        &ATA_PROGRAM_ID,
    )
    .0;
    ctx.write_account(
        staker_orca_ata,
        TOKEN_PROGRAM_ID,
        token_account_data!(
            mint => ORCA_ID,
            owner => ctx.signer(),
            amount => 1_000_000, // owns 1 ORCA
        ),
    )
    .unwrap();

    // Write staker xOrca ata
    let staker_xorca_ata = Pubkey::find_program_address(
        &[
            &ctx.signer().to_bytes(),
            &TOKEN_PROGRAM_ID.to_bytes(),
            &XORCA_ID.to_bytes(),
        ],
        &ATA_PROGRAM_ID,
    )
    .0;
    ctx.write_account(
        staker_xorca_ata,
        TOKEN_PROGRAM_ID,
        token_account_data!(
            mint => XORCA_ID,
            owner => ctx.signer(),
            amount => 0, // staker has no xORCA
        ),
    )
    .unwrap();

    // Define instruction
    let ix: solana_sdk::instruction::Instruction = Stake {
        staker_account: ctx.signer(),
        state_account,
        vault_account,
        staker_orca_ata,
        staker_xorca_ata,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
    }
    .instruction(StakeInstructionArgs {
        stake_amount: 1_000_000, // stake 1 ORCA
    });

    // Execute instruction
    let result = ctx.send(ix);
    assert_program_error!(result, XorcaStakingProgramError::IncorrectOwner);
}

/// Test 2b: Invalid state account, invalid PDA seeds
#[test]
fn test_stake_invalid_state_account_invalid_seeds() {
    let mut ctx = TestContext::new();
    let seeds: &[&[u8]] = &[b"state_1"]; // invalid seeds
    let state_account = Pubkey::find_program_address(seeds, &XORCA_PROGRAM_ID).0;

    // Write state account
    ctx.write_account(
        state_account,
        XORCA_PROGRAM_ID,
        state_data!(
            escrowed_orca_amount => 0, // 0 ORCA
            update_authority => Pubkey::default(),
            cool_down_period_s => 7 * 24 * 60 * 60, // 7 days
        ),
    )
    .unwrap();
    ctx.pad_account(state_account, DEFAULT_ACCOUNT_LEN).unwrap();

    // Write xOrca mint account with valid data
    ctx.write_account(
        XORCA_ID,
        TOKEN_PROGRAM_ID,
        token_mint_data!(
            supply => 0,
            decimals => 9,
            mint_authority_flag => 1,
            mint_authority => XORCA_PROGRAM_ID,
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
            supply => 1_000_000_000_000, // 1,000,000 ORCA
            decimals => 6,
            mint_authority_flag => 1,
            mint_authority => Pubkey::default(),
            is_initialized => true,
            freeze_authority_flag => 0,
            freeze_authority => Pubkey::default(),
        ),
    )
    .unwrap();

    // Write Vault Orca ata
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
            amount => 0, // Vault has no ORCA
        ),
    )
    .unwrap();

    // Write staker Orca ata
    let staker_orca_ata = Pubkey::find_program_address(
        &[
            &ctx.signer().to_bytes(),
            &TOKEN_PROGRAM_ID.to_bytes(),
            &ORCA_ID.to_bytes(),
        ],
        &ATA_PROGRAM_ID,
    )
    .0;
    ctx.write_account(
        staker_orca_ata,
        TOKEN_PROGRAM_ID,
        token_account_data!(
            mint => ORCA_ID,
            owner => ctx.signer(),
            amount => 1_000_000, // owns 1 ORCA
        ),
    )
    .unwrap();

    // Write staker xOrca ata
    let staker_xorca_ata = Pubkey::find_program_address(
        &[
            &ctx.signer().to_bytes(),
            &TOKEN_PROGRAM_ID.to_bytes(),
            &XORCA_ID.to_bytes(),
        ],
        &ATA_PROGRAM_ID,
    )
    .0;
    ctx.write_account(
        staker_xorca_ata,
        TOKEN_PROGRAM_ID,
        token_account_data!(
            mint => XORCA_ID,
            owner => ctx.signer(),
            amount => 0, // staker has no xORCA
        ),
    )
    .unwrap();

    // Define instruction
    let ix: solana_sdk::instruction::Instruction = Stake {
        staker_account: ctx.signer(),
        state_account,
        vault_account,
        staker_orca_ata,
        staker_xorca_ata,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
    }
    .instruction(StakeInstructionArgs {
        stake_amount: 1_000_000, // stake 1 ORCA
    });

    // Execute instruction
    let result = ctx.send(ix);
    assert_program_error!(result, XorcaStakingProgramError::InvalidSeeds);
}

/// Test 3a: Invalid staker orca ata, invalid ata owner
#[test]
fn test_stake_invalid_staker_orca_ata_invalid_ata_owner() {
    let mut ctx = TestContext::new();
    let state_account = find_state_address().unwrap().0;

    // Write state account
    ctx.write_account(
        state_account,
        XORCA_PROGRAM_ID,
        state_data!(
            escrowed_orca_amount => 0, // 0 ORCA
            update_authority => Pubkey::default(),
            cool_down_period_s => 7 * 24 * 60 * 60, // 7 days
        ),
    )
    .unwrap();
    ctx.pad_account(state_account, DEFAULT_ACCOUNT_LEN).unwrap();

    // Write xOrca mint account with valid data
    ctx.write_account(
        XORCA_ID,
        TOKEN_PROGRAM_ID,
        token_mint_data!(
            supply => 0,
            decimals => 9,
            mint_authority_flag => 1,
            mint_authority => XORCA_PROGRAM_ID,
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
            supply => 1_000_000_000_000, // 1,000,000 ORCA
            decimals => 6,
            mint_authority_flag => 1,
            mint_authority => Pubkey::default(),
            is_initialized => true,
            freeze_authority_flag => 0,
            freeze_authority => Pubkey::default(),
        ),
    )
    .unwrap();

    // Write Vault Orca ata
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
            amount => 0, // Vault has no ORCA
        ),
    )
    .unwrap();

    // Write staker Orca ata
    let staker_orca_ata = Pubkey::find_program_address(
        &[
            &Pubkey::default().to_bytes(), // Invalid token account owner seed
            &TOKEN_PROGRAM_ID.to_bytes(),
            &ORCA_ID.to_bytes(),
        ],
        &ATA_PROGRAM_ID,
    )
    .0;
    ctx.write_account(
        staker_orca_ata,
        TOKEN_PROGRAM_ID,
        token_account_data!(
            mint => ORCA_ID,
            owner => Pubkey::default(), // Invalid token account owner
            amount => 1_000_000, // owns 1 ORCA
        ),
    )
    .unwrap();

    // Write staker xOrca ata
    let staker_xorca_ata = Pubkey::find_program_address(
        &[
            &ctx.signer().to_bytes(),
            &TOKEN_PROGRAM_ID.to_bytes(),
            &XORCA_ID.to_bytes(),
        ],
        &ATA_PROGRAM_ID,
    )
    .0;
    ctx.write_account(
        staker_xorca_ata,
        TOKEN_PROGRAM_ID,
        token_account_data!(
            mint => XORCA_ID,
            owner => ctx.signer(),
            amount => 0, // staker has no xORCA
        ),
    )
    .unwrap();

    // Define instruction
    let ix: solana_sdk::instruction::Instruction = Stake {
        staker_account: ctx.signer(),
        state_account,
        vault_account,
        staker_orca_ata,
        staker_xorca_ata,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
    }
    .instruction(StakeInstructionArgs {
        stake_amount: 1_000_000, // stake 1 ORCA
    });

    // Execute instruction
    let result = ctx.send(ix);
    assert_program_error!(result, XorcaStakingProgramError::InvalidAccountData);
}

/// Test 3b: Invalid token ata, invalid ORCA mint
#[test]
fn test_stake_invalid_staker_orca_ata_invalid_mint() {
    let mut ctx = TestContext::new();
    let (state_account, _) = find_state_address().unwrap();

    // Write state account
    ctx.write_account(
        state_account,
        XORCA_PROGRAM_ID,
        state_data!(
            escrowed_orca_amount => 0, // 0 ORCA
            update_authority => Pubkey::default(),
            cool_down_period_s => 7 * 24 * 60 * 60, // 7 days
        ),
    )
    .unwrap();
    ctx.pad_account(state_account, DEFAULT_ACCOUNT_LEN).unwrap();

    // Write xOrca mint account with valid data
    ctx.write_account(
        XORCA_ID,
        TOKEN_PROGRAM_ID,
        token_mint_data!(
            supply => 0,
            decimals => 9,
            mint_authority_flag => 1,
            mint_authority => XORCA_PROGRAM_ID,
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
            supply => 1_000_000_000_000, // 1,000,000 ORCA
            decimals => 6,
            mint_authority_flag => 1,
            mint_authority => Pubkey::default(),
            is_initialized => true,
            freeze_authority_flag => 0,
            freeze_authority => Pubkey::default(),
        ),
    )
    .unwrap();

    // Write Vault Orca ata
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
            amount => 0, // Vault has no ORCA
        ),
    )
    .unwrap();

    // Write staker Orca ata
    let staker_orca_ata = Pubkey::find_program_address(
        &[
            &ctx.signer().to_bytes(),
            &TOKEN_PROGRAM_ID.to_bytes(),
            &XORCA_ID.to_bytes(), // invalid mint seed
        ],
        &ATA_PROGRAM_ID,
    )
    .0;
    ctx.write_account(
        staker_orca_ata,
        TOKEN_PROGRAM_ID,
        token_account_data!(
            mint => XORCA_ID, // invalid mint
            owner => ctx.signer(),
            amount => 1_000_000, // owns 1 ORCA
        ),
    )
    .unwrap();

    // Write staker xOrca ata
    let staker_xorca_ata = Pubkey::find_program_address(
        &[
            &ctx.signer().to_bytes(),
            &TOKEN_PROGRAM_ID.to_bytes(),
            &XORCA_ID.to_bytes(),
        ],
        &ATA_PROGRAM_ID,
    )
    .0;
    ctx.write_account(
        staker_xorca_ata,
        TOKEN_PROGRAM_ID,
        token_account_data!(
            mint => XORCA_ID,
            owner => ctx.signer(),
            amount => 0, // staker has no xORCA
        ),
    )
    .unwrap();

    // Define instruction
    let ix: solana_sdk::instruction::Instruction = Stake {
        staker_account: ctx.signer(),
        state_account,
        vault_account,
        staker_orca_ata,
        staker_xorca_ata,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
    }
    .instruction(StakeInstructionArgs {
        stake_amount: 1_000_000, // stake 1 ORCA
    });

    // Execute instruction
    let result = ctx.send(ix);
    assert_program_error!(result, XorcaStakingProgramError::InvalidAccountData);
}

/// Test 3c: Invalid Orca ata, ata not owned by Token Program
#[test]
fn test_stake_invalid_staker_orca_ata_invalid_owner() {
    let mut ctx = TestContext::new();
    let (state_account, _) = find_state_address().unwrap();

    // Write state account
    ctx.write_account(
        state_account,
        XORCA_PROGRAM_ID,
        state_data!(
            escrowed_orca_amount => 0, // 0 ORCA
            update_authority => Pubkey::default(),
            cool_down_period_s => 7 * 24 * 60 * 60, // 7 days
        ),
    )
    .unwrap();
    ctx.pad_account(state_account, DEFAULT_ACCOUNT_LEN).unwrap();

    // Write xOrca mint account with valid data
    ctx.write_account(
        XORCA_ID,
        TOKEN_PROGRAM_ID,
        token_mint_data!(
            supply => 0,
            decimals => 9,
            mint_authority_flag => 1,
            mint_authority => XORCA_PROGRAM_ID,
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
            supply => 1_000_000_000_000, // 1,000,000 ORCA
            decimals => 6,
            mint_authority_flag => 1,
            mint_authority => Pubkey::default(),
            is_initialized => true,
            freeze_authority_flag => 0,
            freeze_authority => Pubkey::default(),
        ),
    )
    .unwrap();

    // Write Vault Orca ata
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
            amount => 0, // Vault has no ORCA
        ),
    )
    .unwrap();

    // Write staker Orca ata
    let staker_orca_ata = Pubkey::find_program_address(
        &[
            &ctx.signer().to_bytes(),
            &ATA_PROGRAM_ID.to_bytes(), // invalid owner
            &ORCA_ID.to_bytes(),
        ],
        &ATA_PROGRAM_ID,
    )
    .0;
    ctx.write_account(
        staker_orca_ata,
        ATA_PROGRAM_ID, // invalid owner
        token_account_data!(
            mint => ORCA_ID,
            owner => ctx.signer(),
            amount => 1_000_000, // owns 1 ORCA
        ),
    )
    .unwrap();

    // Write staker xOrca ata
    let staker_xorca_ata = Pubkey::find_program_address(
        &[
            &ctx.signer().to_bytes(),
            &TOKEN_PROGRAM_ID.to_bytes(),
            &XORCA_ID.to_bytes(),
        ],
        &ATA_PROGRAM_ID,
    )
    .0;
    ctx.write_account(
        staker_xorca_ata,
        TOKEN_PROGRAM_ID,
        token_account_data!(
            mint => XORCA_ID,
            owner => ctx.signer(),
            amount => 0, // staker has no xORCA
        ),
    )
    .unwrap();

    // Define instruction
    let ix: solana_sdk::instruction::Instruction = Stake {
        staker_account: ctx.signer(),
        state_account,
        vault_account,
        staker_orca_ata,
        staker_xorca_ata,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
    }
    .instruction(StakeInstructionArgs {
        stake_amount: 1_000_000, // stake 1 ORCA
    });

    // Execute instruction
    let result = ctx.send(ix);
    assert_program_error!(result, XorcaStakingProgramError::IncorrectOwner);
}

/// Test 3d: Invalid staker xorca ata, invalid ata owner
#[test]
fn test_stake_invalid_staker_xorca_ata_invalid_ata_owner() {
    let mut ctx = TestContext::new();
    let state_account = find_state_address().unwrap().0;

    // Write state account
    ctx.write_account(
        state_account,
        XORCA_PROGRAM_ID,
        state_data!(
            escrowed_orca_amount => 0, // 0 ORCA
            update_authority => Pubkey::default(),
            cool_down_period_s => 7 * 24 * 60 * 60, // 7 days
        ),
    )
    .unwrap();
    ctx.pad_account(state_account, DEFAULT_ACCOUNT_LEN).unwrap();

    // Write xOrca mint account with valid data
    ctx.write_account(
        XORCA_ID,
        TOKEN_PROGRAM_ID,
        token_mint_data!(
            supply => 0,
            decimals => 9,
            mint_authority_flag => 1,
            mint_authority => XORCA_PROGRAM_ID,
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
            supply => 1_000_000_000_000, // 1,000,000 ORCA
            decimals => 6,
            mint_authority_flag => 1,
            mint_authority => Pubkey::default(),
            is_initialized => true,
            freeze_authority_flag => 0,
            freeze_authority => Pubkey::default(),
        ),
    )
    .unwrap();

    // Write Vault Orca ata
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
            amount => 0, // Vault has no ORCA
        ),
    )
    .unwrap();

    // Write staker Orca ata
    let staker_orca_ata = Pubkey::find_program_address(
        &[
            &ctx.signer().to_bytes(),
            &TOKEN_PROGRAM_ID.to_bytes(),
            &ORCA_ID.to_bytes(),
        ],
        &ATA_PROGRAM_ID,
    )
    .0;
    ctx.write_account(
        staker_orca_ata,
        TOKEN_PROGRAM_ID,
        token_account_data!(
            mint => ORCA_ID,
            owner => ctx.signer(),
            amount => 1_000_000, // owns 1 ORCA
        ),
    )
    .unwrap();

    // Write staker xOrca ata
    let staker_xorca_ata = Pubkey::find_program_address(
        &[
            &Pubkey::default().to_bytes(), // Invalid token account owner seed
            &TOKEN_PROGRAM_ID.to_bytes(),
            &XORCA_ID.to_bytes(),
        ],
        &ATA_PROGRAM_ID,
    )
    .0;
    ctx.write_account(
        staker_xorca_ata,
        TOKEN_PROGRAM_ID,
        token_account_data!(
            mint => XORCA_ID,
            owner => Pubkey::default(), // Invalid token account owner
            amount => 0, // staker has no xORCA
        ),
    )
    .unwrap();

    // Define instruction
    let ix: solana_sdk::instruction::Instruction = Stake {
        staker_account: ctx.signer(),
        state_account,
        vault_account,
        staker_orca_ata,
        staker_xorca_ata,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
    }
    .instruction(StakeInstructionArgs {
        stake_amount: 1_000_000, // stake 1 ORCA
    });

    // Execute instruction
    let result = ctx.send(ix);
    assert_program_error!(result, XorcaStakingProgramError::InvalidAccountData);
}

/// Test 3e: Invalid token ata, invalid xOrca mint
#[test]
fn test_stake_invalid_staker_xorca_ata_invalid_mint() {
    let mut ctx = TestContext::new();
    let (state_account, _) = find_state_address().unwrap();

    // Write state account
    ctx.write_account(
        state_account,
        XORCA_PROGRAM_ID,
        state_data!(
            escrowed_orca_amount => 0, // 0 ORCA
            update_authority => Pubkey::default(),
            cool_down_period_s => 7 * 24 * 60 * 60, // 7 days
        ),
    )
    .unwrap();
    ctx.pad_account(state_account, DEFAULT_ACCOUNT_LEN).unwrap();

    // Write xOrca mint account with valid data
    ctx.write_account(
        XORCA_ID,
        TOKEN_PROGRAM_ID,
        token_mint_data!(
            supply => 0,
            decimals => 9,
            mint_authority_flag => 1,
            mint_authority => XORCA_PROGRAM_ID,
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
            supply => 1_000_000_000_000, // 1,000,000 ORCA
            decimals => 6,
            mint_authority_flag => 1,
            mint_authority => Pubkey::default(),
            is_initialized => true,
            freeze_authority_flag => 0,
            freeze_authority => Pubkey::default(),
        ),
    )
    .unwrap();

    // Write Vault Orca ata
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
            amount => 0, // Vault has no ORCA
        ),
    )
    .unwrap();

    // Write staker Orca ata
    let staker_orca_ata = Pubkey::find_program_address(
        &[
            &ctx.signer().to_bytes(),
            &TOKEN_PROGRAM_ID.to_bytes(),
            &ORCA_ID.to_bytes(),
        ],
        &ATA_PROGRAM_ID,
    )
    .0;
    ctx.write_account(
        staker_orca_ata,
        TOKEN_PROGRAM_ID,
        token_account_data!(
            mint => ORCA_ID,
            owner => ctx.signer(),
            amount => 1_000_000, // owns 1 ORCA
        ),
    )
    .unwrap();

    // Write staker xOrca ata
    let staker_xorca_ata = Pubkey::find_program_address(
        &[
            &ctx.signer().to_bytes(),
            &TOKEN_PROGRAM_ID.to_bytes(),
            &ORCA_ID.to_bytes(), // invalid mint seed
        ],
        &ATA_PROGRAM_ID,
    )
    .0;
    ctx.write_account(
        staker_xorca_ata,
        TOKEN_PROGRAM_ID,
        token_account_data!(
            mint => ORCA_ID, // invalid mint
            owner => ctx.signer(),
            amount => 0, // staker has no xORCA
        ),
    )
    .unwrap();

    // Define instruction
    let ix: solana_sdk::instruction::Instruction = Stake {
        staker_account: ctx.signer(),
        state_account,
        vault_account,
        staker_orca_ata,
        staker_xorca_ata,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
    }
    .instruction(StakeInstructionArgs {
        stake_amount: 1_000_000, // stake 1 ORCA
    });

    // Execute instruction
    let result = ctx.send(ix);
    assert_program_error!(result, XorcaStakingProgramError::InvalidAccountData);
}

/// Test 3e: Invalid xOrca ata, ata not owned by Token Program
#[test]
fn test_stake_invalid_staker_xorca_ata_invalid_owner() {
    let mut ctx = TestContext::new();
    let (state_account, _) = find_state_address().unwrap();

    // Write state account
    ctx.write_account(
        state_account,
        XORCA_PROGRAM_ID,
        state_data!(
            escrowed_orca_amount => 0, // 0 ORCA
            update_authority => Pubkey::default(),
            cool_down_period_s => 7 * 24 * 60 * 60, // 7 days
        ),
    )
    .unwrap();
    ctx.pad_account(state_account, DEFAULT_ACCOUNT_LEN).unwrap();

    // Write xOrca mint account with valid data
    ctx.write_account(
        XORCA_ID,
        TOKEN_PROGRAM_ID,
        token_mint_data!(
            supply => 0,
            decimals => 9,
            mint_authority_flag => 1,
            mint_authority => XORCA_PROGRAM_ID,
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
            supply => 1_000_000_000_000, // 1,000,000 ORCA
            decimals => 6,
            mint_authority_flag => 1,
            mint_authority => Pubkey::default(),
            is_initialized => true,
            freeze_authority_flag => 0,
            freeze_authority => Pubkey::default(),
        ),
    )
    .unwrap();

    // Write Vault Orca ata
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
            amount => 0, // Vault has no ORCA
        ),
    )
    .unwrap();

    // Write staker Orca ata
    let staker_orca_ata = Pubkey::find_program_address(
        &[
            &ctx.signer().to_bytes(),
            &TOKEN_PROGRAM_ID.to_bytes(),
            &ORCA_ID.to_bytes(),
        ],
        &ATA_PROGRAM_ID,
    )
    .0;
    ctx.write_account(
        staker_orca_ata,
        TOKEN_PROGRAM_ID,
        token_account_data!(
            mint => ORCA_ID,
            owner => ctx.signer(),
            amount => 1_000_000, // owns 1 ORCA
        ),
    )
    .unwrap();

    // Write staker xOrca ata
    let staker_xorca_ata = Pubkey::find_program_address(
        &[
            &ctx.signer().to_bytes(),
            &ATA_PROGRAM_ID.to_bytes(), // invalid owner
            &XORCA_ID.to_bytes(),
        ],
        &ATA_PROGRAM_ID,
    )
    .0;
    ctx.write_account(
        staker_xorca_ata,
        ATA_PROGRAM_ID, // invalid owner
        token_account_data!(
            mint => XORCA_ID,
            owner => ctx.signer(),
            amount => 0, // staker has no xORCA
        ),
    )
    .unwrap();

    // Define instruction
    let ix: solana_sdk::instruction::Instruction = Stake {
        staker_account: ctx.signer(),
        state_account,
        vault_account,
        staker_orca_ata,
        staker_xorca_ata,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
    }
    .instruction(StakeInstructionArgs {
        stake_amount: 1_000_000, // stake 1 ORCA
    });

    // Execute instruction
    let result = ctx.send(ix);
    assert_program_error!(result, XorcaStakingProgramError::IncorrectOwner);
}
