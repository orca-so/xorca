use crate::{
    assert_program_error, assert_program_success, state_data, token_account_data, token_mint_data,
    TestContext, ATA_PROGRAM_ID, ORCA_ID, TOKEN_PROGRAM_ID, XORCA_ID, XORCA_PROGRAM_ID,
};
use rstest::rstest;
use solana_sdk::pubkey::Pubkey;
use xorca::{
    find_state_address, Stake, StakeInstructionArgs, State, TokenAccount, TokenMint,
    XorcaStakingProgramError, DEFAULT_ACCOUNT_LEN,
};

/// Helper function to set up the test context and accounts based on the test case.
/// This function handles the various valid and invalid account configurations.
fn setup_stake_test_context(ctx: &mut TestContext, case: &str) -> Stake {
    let (correct_state_pda, _) = find_state_address().unwrap();
    let staker_signer = ctx.signer();

    // Determine the state_account Pubkey based on the case
    let state_account = if case == "InvalidStateAccountSeeds" {
        // For this specific invalid case, we use a non-PDA address for the state account
        // but still write a valid-looking state data to it.
        Pubkey::find_program_address(&[b"invalid_state_seed"], &XORCA_PROGRAM_ID).0
    } else {
        correct_state_pda
    };

    // Initial values for accounts (can be overridden by specific cases)
    let mut initial_state_escrowed_orca = 0;
    let mut initial_xorca_supply = 0;
    let mut initial_vault_orca_amount = 0;
    let mut initial_staker_orca_amount = 1_000_000; // 1 ORCA
    let mut initial_staker_xorca_amount = 0;

    // Adjust initial values for specific success case
    if case == "SuccessMoreThan1_1" {
        initial_state_escrowed_orca = 39_232_982_923;
        initial_xorca_supply = 358_384_859_821_223;
        initial_vault_orca_amount = 923_384_268_587;
        initial_staker_orca_amount = 5_123_538;
        initial_staker_xorca_amount = 12_823_658_283;
    }

    // Write state account
    let state_account_owner = if case == "InvalidStateAccountOwner" {
        TOKEN_PROGRAM_ID // Incorrect owner for this case
    } else {
        XORCA_PROGRAM_ID
    };
    ctx.write_account(
        state_account,
        state_account_owner,
        state_data!(
            escrowed_orca_amount => initial_state_escrowed_orca,
            update_authority => Pubkey::default(),
            cool_down_period_s => 7 * 24 * 60 * 60,
        ),
    )
    .unwrap();
    ctx.pad_account(state_account, DEFAULT_ACCOUNT_LEN).unwrap();

    // Write xOrca mint account
    ctx.write_account(
        XORCA_ID,
        TOKEN_PROGRAM_ID,
        token_mint_data!(
            supply => initial_xorca_supply,
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

    // Vault Orca ATA (always derived from correct state PDA)
    let vault_account = Pubkey::find_program_address(
        &[
            &correct_state_pda.to_bytes(),
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
            owner => correct_state_pda,
            amount => initial_vault_orca_amount,
        ),
    )
    .unwrap();

    // Staker Orca ATA
    let staker_orca_ata_pda = Pubkey::find_program_address(
        &[
            &staker_signer.to_bytes(),
            &TOKEN_PROGRAM_ID.to_bytes(),
            &ORCA_ID.to_bytes(),
        ],
        &ATA_PROGRAM_ID,
    )
    .0;
    let staker_orca_ata_owner_in_data = if case == "InvalidStakerOrcaAtaOwnerData" {
        Pubkey::default() // Invalid owner in account data
    } else {
        staker_signer
    };
    let staker_orca_ata_mint_in_data = if case == "InvalidStakerOrcaAtaMintData" {
        XORCA_ID // Invalid mint in account data
    } else {
        ORCA_ID
    };
    let staker_orca_ata_program_owner = if case == "InvalidStakerOrcaAtaProgramOwner" {
        ATA_PROGRAM_ID // Incorrect program owner for the account itself
    } else {
        TOKEN_PROGRAM_ID
    };
    ctx.write_account(
        staker_orca_ata_pda,
        staker_orca_ata_program_owner,
        token_account_data!(
            mint => staker_orca_ata_mint_in_data,
            owner => staker_orca_ata_owner_in_data,
            amount => initial_staker_orca_amount,
        ),
    )
    .unwrap();

    // Staker xOrca ATA
    let staker_xorca_ata_pda = Pubkey::find_program_address(
        &[
            &staker_signer.to_bytes(),
            &TOKEN_PROGRAM_ID.to_bytes(),
            &XORCA_ID.to_bytes(),
        ],
        &ATA_PROGRAM_ID,
    )
    .0;
    let staker_xorca_ata_owner_in_data = if case == "InvalidStakerXorcaAtaOwnerData" {
        Pubkey::default() // Invalid owner in account data
    } else {
        staker_signer
    };
    let staker_xorca_ata_mint_in_data = if case == "InvalidStakerXorcaAtaMintData" {
        ORCA_ID // Invalid mint in account data
    } else {
        XORCA_ID
    };
    let staker_xorca_ata_program_owner = if case == "InvalidStakerXorcaAtaProgramOwner" {
        ATA_PROGRAM_ID // Incorrect program owner for the account itself
    } else {
        TOKEN_PROGRAM_ID
    };
    ctx.write_account(
        staker_xorca_ata_pda,
        staker_xorca_ata_program_owner,
        token_account_data!(
            mint => staker_xorca_ata_mint_in_data,
            owner => staker_xorca_ata_owner_in_data,
            amount => initial_staker_xorca_amount,
        ),
    )
    .unwrap();

    Stake {
        state_account,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        vault_account,
        staker_orca_ata: staker_orca_ata_pda,
        staker_xorca_ata: staker_xorca_ata_pda,
        staker_account: staker_signer,
    }
}

#[rstest]
#[case("Success1_1")]
#[case("SuccessMoreThan1_1")]
#[case("InvalidStateAccountOwner")]
#[case("InvalidStateAccountSeeds")]
#[case("InvalidStakerOrcaAtaOwnerData")]
#[case("InvalidStakerOrcaAtaMintData")]
#[case("InvalidStakerOrcaAtaProgramOwner")]
#[case("InvalidStakerXorcaAtaOwnerData")]
#[case("InvalidStakerXorcaAtaMintData")]
#[case("InvalidStakerXorcaAtaProgramOwner")]
fn test_stake_instruction(#[case] case: &str) {
    let mut ctx = TestContext::new();
    let accounts = setup_stake_test_context(&mut ctx, case);

    // Determine stake amount based on success case
    let stake_amount = if case == "SuccessMoreThan1_1" {
        2_384_964 // stake 2.384964 ORCA
    } else {
        1_000_000 // stake 1 ORCA
    };

    // Define instruction
    let ix: solana_sdk::instruction::Instruction = Stake {
        staker_account: accounts.staker_account,
        state_account: accounts.state_account,
        vault_account: accounts.vault_account,
        staker_orca_ata: accounts.staker_orca_ata,
        staker_xorca_ata: accounts.staker_xorca_ata,
        xorca_mint_account: accounts.xorca_mint_account,
        orca_mint_account: accounts.orca_mint_account,
    }
    .instruction(StakeInstructionArgs { stake_amount });

    // Execute instruction
    let result = ctx.send(ix);

    // Assertions based on the test case
    match case {
        "Success1_1" => {
            assert_program_success!(result);
            let vault_account_after = ctx
                .get_account::<TokenAccount>(accounts.vault_account)
                .unwrap();
            let staker_orca_ata_after = ctx
                .get_account::<TokenAccount>(accounts.staker_orca_ata)
                .unwrap();
            let staker_xorca_ata_after = ctx
                .get_account::<TokenAccount>(accounts.staker_xorca_ata)
                .unwrap();
            let xorca_mint_account_after = ctx
                .get_account::<TokenMint>(accounts.xorca_mint_account)
                .unwrap();
            let state_account_after = ctx.get_account::<State>(accounts.state_account).unwrap();

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
        "SuccessMoreThan1_1" => {
            assert_program_success!(result);
            let vault_account_after = ctx
                .get_account::<TokenAccount>(accounts.vault_account)
                .unwrap();
            let staker_orca_ata_after = ctx
                .get_account::<TokenAccount>(accounts.staker_orca_ata)
                .unwrap();
            let staker_xorca_ata_after = ctx
                .get_account::<TokenAccount>(accounts.staker_xorca_ata)
                .unwrap();
            let xorca_mint_account_after = ctx
                .get_account::<TokenMint>(accounts.xorca_mint_account)
                .unwrap();
            let state_account_after = ctx.get_account::<State>(accounts.state_account).unwrap();

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
        "InvalidStateAccountOwner" => {
            assert_program_error!(result, XorcaStakingProgramError::IncorrectOwner);
        }
        "InvalidStateAccountSeeds" => {
            assert_program_error!(result, XorcaStakingProgramError::InvalidSeeds);
        }
        "InvalidStakerOrcaAtaOwnerData"
        | "InvalidStakerOrcaAtaMintData"
        | "InvalidStakerXorcaAtaOwnerData"
        | "InvalidStakerXorcaAtaMintData" => {
            assert_program_error!(result, XorcaStakingProgramError::InvalidAccountData);
        }
        "InvalidStakerOrcaAtaProgramOwner" | "InvalidStakerXorcaAtaProgramOwner" => {
            assert_program_error!(result, XorcaStakingProgramError::IncorrectOwner);
        }
        _ => panic!("Unknown case: {}", case),
    }
}
