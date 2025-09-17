use crate::{
    assert_program_error, TestContext, ATA_PROGRAM_ID, INITIAL_UPDATE_AUTHORITY_ID, ORCA_ID,
    SYSTEM_PROGRAM_ID, TOKEN_PROGRAM_ID, XORCA_ID,
};
use solana_sdk::pubkey::Pubkey;
use xorca::{
    find_orca_vault_address, find_state_address, Initialize, InitializeInstructionArgs, State,
    TokenMint,
};

#[test]
fn initialize_sets_values_with_standard_values_success() {
    let mut ctx = TestContext::new();
    let (state, _) = find_state_address().unwrap();

    // Seed mints
    ctx.write_account(
        XORCA_ID,
        TOKEN_PROGRAM_ID,
        crate::token_mint_data!(
            supply => 0,
            decimals => 6,
            mint_authority_flag => 1,
            mint_authority => state,
            is_initialized => true,
            freeze_authority_flag => 0,
            freeze_authority => Pubkey::default(),
        ),
    )
    .unwrap();
    ctx.write_account(
        ORCA_ID,
        TOKEN_PROGRAM_ID,
        crate::token_mint_data!(
            supply => 0,
            decimals => 6,
            mint_authority_flag => 1,
            mint_authority => Pubkey::default(),
            is_initialized => true,
            freeze_authority_flag => 0,
            freeze_authority => Pubkey::default(),
        ),
    )
    .unwrap();

    // Calculate vault account address
    let (vault_account, _) = find_orca_vault_address(&state, &TOKEN_PROGRAM_ID, &ORCA_ID).unwrap();

    let ix = Initialize {
        payer_account: ctx.signer(),
        state_account: state,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        update_authority_account: INITIAL_UPDATE_AUTHORITY_ID,
        system_program_account: SYSTEM_PROGRAM_ID,
        vault_account,
        token_program_account: TOKEN_PROGRAM_ID,
        associated_token_program_account: ATA_PROGRAM_ID,
    }
    .instruction(InitializeInstructionArgs {
        cool_down_period_s: 100,
    });
    assert!(ctx.send(ix).is_ok());

    let state_account = ctx.get_account::<State>(state).unwrap();
    assert_eq!(state_account.data.cool_down_period_s, 100);
    assert_eq!(
        state_account.data.update_authority,
        INITIAL_UPDATE_AUTHORITY_ID
    );
    assert_eq!(state_account.account.owner, crate::XORCA_PROGRAM_ID);
    let mint_after = ctx.get_account::<TokenMint>(XORCA_ID).unwrap();
    assert_eq!(mint_after.data.mint_authority, state);
    assert_eq!(mint_after.data.supply, 0);
}

// Update authority must match the expected constant
#[test]
fn initialize_fails_with_wrong_update_authority_account() {
    let mut ctx = TestContext::new();
    let (state, _) = find_state_address().unwrap();
    // Seed mints
    ctx.write_account(
        XORCA_ID,
        TOKEN_PROGRAM_ID,
        crate::token_mint_data!(
            supply => 0,
            decimals => 6,
            mint_authority_flag => 1,
            mint_authority => state,
            is_initialized => true,
            freeze_authority_flag => 0,
            freeze_authority => Pubkey::default(),
        ),
    )
    .unwrap();
    ctx.write_account(
        ORCA_ID,
        TOKEN_PROGRAM_ID,
        crate::token_mint_data!(
            supply => 0,
            decimals => 6,
            mint_authority_flag => 1,
            mint_authority => Pubkey::default(),
            is_initialized => true,
            freeze_authority_flag => 0,
            freeze_authority => Pubkey::default(),
        ),
    )
    .unwrap();
    let wrong_update = Pubkey::new_unique();

    // Calculate vault account address
    let (vault_account, _) = find_orca_vault_address(&state, &TOKEN_PROGRAM_ID, &ORCA_ID).unwrap();

    let ix = Initialize {
        payer_account: ctx.signer(),
        state_account: state,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        update_authority_account: wrong_update,
        system_program_account: SYSTEM_PROGRAM_ID,
        vault_account,
        token_program_account: TOKEN_PROGRAM_ID,
        associated_token_program_account: ATA_PROGRAM_ID,
    }
    .instruction(InitializeInstructionArgs {
        cool_down_period_s: 1,
    });
    let res = ctx.send(ix);
    assert_program_error!(
        res,
        xorca::XorcaStakingProgramError::IncorrectAccountAddress
    );
}

// System program must be correct
#[test]
fn initialize_fails_with_wrong_system_program_account() {
    let mut ctx = TestContext::new();
    let (state, _) = find_state_address().unwrap();
    // Seed mints
    ctx.write_account(
        XORCA_ID,
        TOKEN_PROGRAM_ID,
        crate::token_mint_data!(
            supply => 0,
            decimals => 6,
            mint_authority_flag => 1,
            mint_authority => state,
            is_initialized => true,
            freeze_authority_flag => 0,
            freeze_authority => Pubkey::default(),
        ),
    )
    .unwrap();
    ctx.write_account(
        ORCA_ID,
        TOKEN_PROGRAM_ID,
        crate::token_mint_data!(
            supply => 0,
            decimals => 6,
            mint_authority_flag => 1,
            mint_authority => Pubkey::default(),
            is_initialized => true,
            freeze_authority_flag => 0,
            freeze_authority => Pubkey::default(),
        ),
    )
    .unwrap();
    let wrong_system = Pubkey::new_unique();

    // Calculate vault account address
    let (vault_account, _) = find_orca_vault_address(&state, &TOKEN_PROGRAM_ID, &ORCA_ID).unwrap();

    let ix = Initialize {
        payer_account: ctx.signer(),
        state_account: state,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        update_authority_account: INITIAL_UPDATE_AUTHORITY_ID,
        system_program_account: wrong_system,
        vault_account,
        token_program_account: TOKEN_PROGRAM_ID,
        associated_token_program_account: ATA_PROGRAM_ID,
    }
    .instruction(InitializeInstructionArgs {
        cool_down_period_s: 1,
    });
    let res = ctx.send(ix);
    assert_program_error!(
        res,
        xorca::XorcaStakingProgramError::IncorrectAccountAddress
    );
}

// Insufficient lamports: payer has too few lamports to cover any required rents/ops → expect failure
#[test]
fn initialize_fails_with_insufficient_lamports() {
    let mut ctx = TestContext::new();
    // Drain payer lamports
    ctx.set_account(
        ctx.signer(),
        solana_sdk::account::Account {
            lamports: 1000,
            owner: solana_sdk::system_program::ID,
            executable: false,
            rent_epoch: 0,
            data: vec![],
        },
    )
    .unwrap();
    let (state, _) = find_state_address().unwrap();
    // Seed mints minimally
    ctx.write_account(
        XORCA_ID,
        TOKEN_PROGRAM_ID,
        crate::token_mint_data!(
            supply => 0,
            decimals => 6,
            mint_authority_flag => 1,
            mint_authority => state,
            is_initialized => true,
            freeze_authority_flag => 0,
            freeze_authority => Pubkey::default(),
        ),
    )
    .unwrap();
    ctx.write_account(
        ORCA_ID,
        TOKEN_PROGRAM_ID,
        crate::token_mint_data!(
            supply => 0,
            decimals => 6,
            mint_authority_flag => 1,
            mint_authority => Pubkey::default(),
            is_initialized => true,
            freeze_authority_flag => 0,
            freeze_authority => Pubkey::default(),
        ),
    )
    .unwrap();

    // Calculate vault account address
    let (vault_account, _) = find_orca_vault_address(&state, &TOKEN_PROGRAM_ID, &ORCA_ID).unwrap();

    let ix = Initialize {
        payer_account: ctx.signer(),
        state_account: state,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        update_authority_account: INITIAL_UPDATE_AUTHORITY_ID,
        system_program_account: SYSTEM_PROGRAM_ID,
        vault_account,
        token_program_account: TOKEN_PROGRAM_ID,
        associated_token_program_account: ATA_PROGRAM_ID,
    }
    .instruction(InitializeInstructionArgs {
        cool_down_period_s: 1,
    });
    let res = ctx.send(ix);
    assert!(res.is_err(), "Should fail with insufficient lamports");
}

// xORCA mint: frozen should fail (freeze_authority_flag != 0)
#[test]
fn initialize_fails_when_xorca_mint_frozen() {
    let mut ctx = TestContext::new();
    let (state, _) = find_state_address().unwrap();
    ctx.write_account(
        XORCA_ID,
        TOKEN_PROGRAM_ID,
        crate::token_mint_data!(
            supply => 0,
            decimals => 6,
            mint_authority_flag => 1,
            mint_authority => state,
            is_initialized => true,
            freeze_authority_flag => 1,
            freeze_authority => Pubkey::new_unique(),
        ),
    )
    .unwrap();
    ctx.write_account(
        ORCA_ID,
        TOKEN_PROGRAM_ID,
        crate::token_mint_data!(
            supply => 0,
            decimals => 6,
            mint_authority_flag => 1,
            mint_authority => Pubkey::default(),
            is_initialized => true, freeze_authority_flag => 0, freeze_authority => Pubkey::default(),
        )
    ).unwrap();

    // Calculate vault account address
    let (vault_account, _) = find_orca_vault_address(&state, &TOKEN_PROGRAM_ID, &ORCA_ID).unwrap();

    let ix = Initialize {
        payer_account: ctx.signer(),
        state_account: state,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        update_authority_account: INITIAL_UPDATE_AUTHORITY_ID,
        system_program_account: SYSTEM_PROGRAM_ID,
        vault_account,
        token_program_account: TOKEN_PROGRAM_ID,
        associated_token_program_account: ATA_PROGRAM_ID,
    }
    .instruction(InitializeInstructionArgs {
        cool_down_period_s: 1,
    });
    let res = ctx.send(ix);
    assert_program_error!(res, xorca::XorcaStakingProgramError::InvalidAccountData);
}

// xORCA mint: mint_authority_flag = 0 should fail
#[test]
fn initialize_fails_when_xorca_mint_no_authority_flag() {
    let mut ctx = TestContext::new();
    let (state, _) = find_state_address().unwrap();
    ctx.write_account(
        XORCA_ID,
        TOKEN_PROGRAM_ID,
        crate::token_mint_data!(
            supply => 0,
            decimals => 6,
            mint_authority_flag => 0,
            mint_authority => state,
            is_initialized => true,
            freeze_authority_flag => 0,
            freeze_authority => Pubkey::default(),
        ),
    )
    .unwrap();
    ctx.write_account(
        ORCA_ID,
        TOKEN_PROGRAM_ID,
        crate::token_mint_data!(
            supply => 0,
            decimals => 6,
            mint_authority_flag => 1,
            mint_authority => Pubkey::default(),
            is_initialized => true, freeze_authority_flag => 0, freeze_authority => Pubkey::default(),
        )
    ).unwrap();

    let (vault_account, _) = find_orca_vault_address(&state, &TOKEN_PROGRAM_ID, &ORCA_ID).unwrap();

    let ix = Initialize {
        payer_account: ctx.signer(),
        state_account: state,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        update_authority_account: INITIAL_UPDATE_AUTHORITY_ID,
        system_program_account: SYSTEM_PROGRAM_ID,
        vault_account,
        token_program_account: TOKEN_PROGRAM_ID,
        associated_token_program_account: ATA_PROGRAM_ID,
    }
    .instruction(InitializeInstructionArgs {
        cool_down_period_s: 1,
    });
    let res = ctx.send(ix);
    assert_program_error!(res, xorca::XorcaStakingProgramError::InvalidAccountData);
}
// xORCA mint supply must be zero
#[test]
fn initialize_fails_when_xorca_mint_supply_nonzero() {
    let mut ctx = TestContext::new();
    let (state, _) = find_state_address().unwrap();
    ctx.write_account(
        XORCA_ID,
        TOKEN_PROGRAM_ID,
        crate::token_mint_data!(
            supply => 1,
            decimals => 6,
            mint_authority_flag => 1,
            mint_authority => state,
            is_initialized => true,
            freeze_authority_flag => 0,
            freeze_authority => Pubkey::default(),
        ),
    )
    .unwrap();
    ctx.write_account(
        ORCA_ID,
        TOKEN_PROGRAM_ID,
        crate::token_mint_data!(
            supply => 0,
            decimals => 6,
            mint_authority_flag => 1,
            mint_authority => Pubkey::default(),
            is_initialized => true,
            freeze_authority_flag => 0,
            freeze_authority => Pubkey::default(),
        ),
    )
    .unwrap();

    // Calculate vault account address
    let (vault_account, _) = find_orca_vault_address(&state, &TOKEN_PROGRAM_ID, &ORCA_ID).unwrap();

    let ix = Initialize {
        payer_account: ctx.signer(),
        state_account: state,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        update_authority_account: INITIAL_UPDATE_AUTHORITY_ID,
        system_program_account: SYSTEM_PROGRAM_ID,
        vault_account,
        token_program_account: TOKEN_PROGRAM_ID,
        associated_token_program_account: ATA_PROGRAM_ID,
    }
    .instruction(InitializeInstructionArgs {
        cool_down_period_s: 1,
    });
    let res = ctx.send(ix);
    assert_program_error!(res, xorca::XorcaStakingProgramError::InvalidAccountData);
}

// xORCA mint wrong owner
#[test]
fn initialize_fails_when_xorca_mint_wrong_owner() {
    let mut ctx = TestContext::new();
    let (state, _) = find_state_address().unwrap();
    ctx.write_account(
        XORCA_ID,
        SYSTEM_PROGRAM_ID,
        crate::token_mint_data!(
            supply => 0,
            decimals => 6,
            mint_authority_flag => 1,
            mint_authority => state,
            is_initialized => true,
            freeze_authority_flag => 0,
            freeze_authority => Pubkey::default(),
        ),
    )
    .unwrap();
    ctx.write_account(
        ORCA_ID,
        TOKEN_PROGRAM_ID,
        crate::token_mint_data!(
            supply => 0,
            decimals => 6,
            mint_authority_flag => 1,
            mint_authority => Pubkey::default(),
            is_initialized => true,
            freeze_authority_flag => 0,
            freeze_authority => Pubkey::default(),
        ),
    )
    .unwrap();
    let (vault_account, _) = find_orca_vault_address(&state, &TOKEN_PROGRAM_ID, &ORCA_ID).unwrap();

    let ix = Initialize {
        payer_account: ctx.signer(),
        state_account: state,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        update_authority_account: INITIAL_UPDATE_AUTHORITY_ID,
        system_program_account: SYSTEM_PROGRAM_ID,
        vault_account,
        token_program_account: TOKEN_PROGRAM_ID,
        associated_token_program_account: ATA_PROGRAM_ID,
    }
    .instruction(InitializeInstructionArgs {
        cool_down_period_s: 1,
    });
    let res = ctx.send(ix);
    assert_program_error!(res, xorca::XorcaStakingProgramError::IncorrectOwner);
}

// xORCA mint wrong address
#[test]
fn initialize_fails_when_xorca_mint_wrong_address() {
    let mut ctx = TestContext::new();
    let (state, _) = find_state_address().unwrap();
    let wrong_mint = Pubkey::new_unique();
    ctx.write_account(
        wrong_mint,
        TOKEN_PROGRAM_ID,
        crate::token_mint_data!(
            supply => 0,
            decimals => 6,
            mint_authority_flag => 1,
            mint_authority => state,
            is_initialized => true,
            freeze_authority_flag => 0,
            freeze_authority => Pubkey::default(),
        ),
    )
    .unwrap();
    ctx.write_account(
        ORCA_ID,
        TOKEN_PROGRAM_ID,
        crate::token_mint_data!(
            supply => 0,
            decimals => 6,
            mint_authority_flag => 1,
            mint_authority => Pubkey::default(),
            is_initialized => true,
            freeze_authority_flag => 0,
            freeze_authority => Pubkey::default(),
        ),
    )
    .unwrap();
    let (vault_account, _) = find_orca_vault_address(&state, &TOKEN_PROGRAM_ID, &ORCA_ID).unwrap();

    let ix = Initialize {
        payer_account: ctx.signer(),
        state_account: state,
        xorca_mint_account: wrong_mint,
        orca_mint_account: ORCA_ID,
        update_authority_account: INITIAL_UPDATE_AUTHORITY_ID,
        system_program_account: SYSTEM_PROGRAM_ID,
        vault_account,
        token_program_account: TOKEN_PROGRAM_ID,
        associated_token_program_account: ATA_PROGRAM_ID,
    }
    .instruction(InitializeInstructionArgs {
        cool_down_period_s: 1,
    });
    let res = ctx.send(ix);
    assert_program_error!(
        res,
        xorca::XorcaStakingProgramError::IncorrectAccountAddress
    );
}
// State already initialized should fail
#[test]
fn initialize_fails_when_state_already_initialized() {
    let mut ctx = TestContext::new();
    let (state, _) = find_state_address().unwrap();
    // Pre-populate state with any data (non-empty)
    ctx.write_raw_account(state, SYSTEM_PROGRAM_ID, vec![1u8])
        .unwrap();
    // Seed mints
    ctx.write_account(
        XORCA_ID,
        TOKEN_PROGRAM_ID,
        crate::token_mint_data!(
            supply => 0,
            decimals => 6,
            mint_authority_flag => 1,
            mint_authority => state,
            is_initialized => true,
            freeze_authority_flag => 0,
            freeze_authority => Pubkey::default(),
        ),
    )
    .unwrap();
    ctx.write_account(
        ORCA_ID,
        TOKEN_PROGRAM_ID,
        crate::token_mint_data!(
            supply => 0,
            decimals => 6,
            mint_authority_flag => 1,
            mint_authority => Pubkey::default(),
            is_initialized => true,
            freeze_authority_flag => 0,
            freeze_authority => Pubkey::default(),
        ),
    )
    .unwrap();

    let (vault_account, _) = find_orca_vault_address(&state, &TOKEN_PROGRAM_ID, &ORCA_ID).unwrap();

    let ix = Initialize {
        payer_account: ctx.signer(),
        state_account: state,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        update_authority_account: INITIAL_UPDATE_AUTHORITY_ID,
        system_program_account: SYSTEM_PROGRAM_ID,
        vault_account,
        token_program_account: TOKEN_PROGRAM_ID,
        associated_token_program_account: ATA_PROGRAM_ID,
    }
    .instruction(InitializeInstructionArgs {
        cool_down_period_s: 1,
    });
    let res = ctx.send(ix);
    assert_program_error!(
        res,
        xorca::XorcaStakingProgramError::StateAccountAlreadyInitialized
    );
}

// Wrong state owner (not System Program)
#[test]
fn initialize_fails_with_wrong_state_owner() {
    let mut ctx = TestContext::new();
    let (state, _) = find_state_address().unwrap();
    ctx.write_raw_account(state, TOKEN_PROGRAM_ID, vec![])
        .unwrap();
    // Seed mints
    ctx.write_account(
        XORCA_ID,
        TOKEN_PROGRAM_ID,
        crate::token_mint_data!(
            supply => 0,
            decimals => 6,
            mint_authority_flag => 1,
            mint_authority => state,
            is_initialized => true,
            freeze_authority_flag => 0,
            freeze_authority => Pubkey::default(),
        ),
    )
    .unwrap();
    ctx.write_account(
        ORCA_ID,
        TOKEN_PROGRAM_ID,
        crate::token_mint_data!(
            supply => 0,
            decimals => 6,
            mint_authority_flag => 1,
            mint_authority => Pubkey::default(),
            is_initialized => true,
            freeze_authority_flag => 0,
            freeze_authority => Pubkey::default(),
        ),
    )
    .unwrap();
    let (vault_account, _) = find_orca_vault_address(&state, &TOKEN_PROGRAM_ID, &ORCA_ID).unwrap();

    let ix = Initialize {
        payer_account: ctx.signer(),
        state_account: state,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        update_authority_account: INITIAL_UPDATE_AUTHORITY_ID,
        system_program_account: SYSTEM_PROGRAM_ID,
        vault_account,
        token_program_account: TOKEN_PROGRAM_ID,
        associated_token_program_account: ATA_PROGRAM_ID,
    }
    .instruction(InitializeInstructionArgs {
        cool_down_period_s: 1,
    });
    let res = ctx.send(ix);
    assert_program_error!(res, xorca::XorcaStakingProgramError::IncorrectOwner);
}
