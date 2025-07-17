use crate::{
    assert_program_error, assert_program_success, state_data, token_account_data, token_mint_data,
    TestContext, ATA_PROGRAM_ID, ORCA_ID, TOKEN_PROGRAM_ID, XORCA_ID, XORCA_PROGRAM_ID,
};
use solana_sdk::pubkey::Pubkey;
use xorca::{
    find_state_address, Stake, StakeInstructionArgs, State, TokenAccount, TokenMint,
    XorcaStakingProgramError,
};

/// Sets up the basic test context with correct PDAs and initial mint accounts.
fn setup_base_stake_context(ctx: &mut TestContext) -> (Pubkey, Pubkey, Pubkey, Pubkey, Pubkey) {
    let state_account = find_state_address().unwrap().0;
    let staker_signer = ctx.signer();
    ctx.write_account(
        state_account,
        XORCA_PROGRAM_ID,
        state_data!(
            escrowed_orca_amount => 0,
            update_authority => Pubkey::default(),
            cool_down_period_s => 7 * 24 * 60 * 60,
        ),
    )
    .unwrap();
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
            supply => 1_000_000_000_000, // Total ORCA supply example
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
            amount => 0, // Initial vault amount 0
        ),
    )
    .unwrap();
    let staker_orca_ata = Pubkey::find_program_address(
        &[
            &staker_signer.to_bytes(),
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
            owner => staker_signer,
            amount => 0, // Initial staker ORCA amount 0
        ),
    )
    .unwrap();
    let staker_xorca_ata = Pubkey::find_program_address(
        &[
            &staker_signer.to_bytes(),
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
            owner => staker_signer,
            amount => 0, // Initial staker xORCA amount 0
        ),
    )
    .unwrap();
    (
        state_account,
        vault_account,
        staker_orca_ata,
        staker_xorca_ata,
        staker_signer,
    )
}

fn set_balances_for_1_1_exchange(
    ctx: &mut TestContext,
    state_account: Pubkey,
    staker_orca_ata: Pubkey,
    staker_signer: Pubkey,
    orca_stake_amount: u64,
) {
    ctx.write_account(
        state_account,
        XORCA_PROGRAM_ID,
        state_data!(
            escrowed_orca_amount => 0, // No escrowed ORCA initially for 1:1
            update_authority => Pubkey::default(),
            cool_down_period_s => 7 * 24 * 60 * 60,
        ),
    )
    .unwrap();
    ctx.write_account(
        XORCA_ID,
        TOKEN_PROGRAM_ID,
        token_mint_data!(
            supply => 0,// 0 xORCA supply
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
        staker_orca_ata,
        TOKEN_PROGRAM_ID,
        token_account_data!(
            mint => ORCA_ID,
            owner => staker_signer,
            amount => orca_stake_amount,
        ),
    )
    .unwrap();
}

fn set_balances_for_more_than_1_1_exchange(
    ctx: &mut TestContext,
    state_account: Pubkey,
    vault_account: Pubkey,
    staker_orca_ata: Pubkey,
    staker_xorca_ata: Pubkey,
    staker_signer: Pubkey,
    orca_stake_amount: u64,
) {
    ctx.write_account(
        state_account,
        XORCA_PROGRAM_ID,
        state_data!(
            escrowed_orca_amount => 39_232_982_923, // 39,232.982923 ORCA
            update_authority => Pubkey::default(),
            cool_down_period_s => 7 * 24 * 60 * 60,
        ),
    )
    .unwrap();
    ctx.write_account(
        XORCA_ID,
        TOKEN_PROGRAM_ID,
        token_mint_data!(
            supply => 358_384_859_821_223, // 358,384.859821223 xORCA
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
            amount => 923_384_268_587, // 923,384.268587 ORCA
        ),
    )
    .unwrap();
    ctx.write_account(
        staker_orca_ata,
        TOKEN_PROGRAM_ID,
        token_account_data!(
            mint => ORCA_ID,
            owner => staker_signer,
            amount => orca_stake_amount,
        ),
    )
    .unwrap();
    ctx.write_account(
        staker_xorca_ata,
        TOKEN_PROGRAM_ID,
        token_account_data!(
            mint => XORCA_ID,
            owner => staker_signer,
            amount => 12_823_658_283, // 12.823658283 xORCA
        ),
    )
    .unwrap();
}

// --- Invalid Account Configuration Helpers ---
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

fn make_staker_orca_ata_invalid_owner_in_data(ctx: &mut TestContext, staker_orca_ata: Pubkey) {
    ctx.write_account(
        staker_orca_ata,
        TOKEN_PROGRAM_ID,
        token_account_data!(
            mint => ORCA_ID,
            owner => Pubkey::default(), // Invalid owner in account data
            amount => 1_000_000,
        ),
    )
    .unwrap();
}

fn make_staker_orca_ata_invalid_mint_in_data(ctx: &mut TestContext, staker_orca_ata: Pubkey) {
    ctx.write_account(
        staker_orca_ata,
        TOKEN_PROGRAM_ID,
        token_account_data!(
            mint => XORCA_ID, // Invalid mint in account data
            owner => ctx.signer(),
            amount => 1_000_000,
        ),
    )
    .unwrap();
}

fn make_staker_orca_ata_invalid_program_owner(ctx: &mut TestContext, staker_orca_ata: Pubkey) {
    ctx.write_account(
        staker_orca_ata,
        ATA_PROGRAM_ID, // Incorrect program owner for the account itself
        token_account_data!(
            mint => ORCA_ID,
            owner => ctx.signer(),
            amount => 1_000_000,
        ),
    )
    .unwrap();
}

fn make_staker_xorca_ata_invalid_owner_in_data(ctx: &mut TestContext, staker_xorca_ata: Pubkey) {
    ctx.write_account(
        staker_xorca_ata,
        TOKEN_PROGRAM_ID,
        token_account_data!(
            mint => XORCA_ID,
            owner => Pubkey::default(), // Invalid owner in account data
            amount => 0,
        ),
    )
    .unwrap();
}

fn make_staker_xorca_ata_invalid_mint_in_data(ctx: &mut TestContext, staker_xorca_ata: Pubkey) {
    ctx.write_account(
        staker_xorca_ata,
        TOKEN_PROGRAM_ID,
        token_account_data!(
            mint => ORCA_ID, // Invalid mint in account data
            owner => ctx.signer(),
            amount => 0,
        ),
    )
    .unwrap();
}

fn make_staker_xorca_ata_invalid_program_owner(ctx: &mut TestContext, staker_xorca_ata: Pubkey) {
    ctx.write_account(
        staker_xorca_ata,
        ATA_PROGRAM_ID, // Incorrect program owner for the account itself
        token_account_data!(
            mint => XORCA_ID,
            owner => ctx.signer(),
            amount => 0,
        ),
    )
    .unwrap();
}

// --- Test Functions ---

#[test]
fn test_stake_success_1_1_exchange() {
    let mut ctx = TestContext::new();
    let (state_account, vault_account, staker_orca_ata, staker_xorca_ata, staker_signer) =
        setup_base_stake_context(&mut ctx);
    let orca_stake_amount = 1_000_000; // 1 ORCA
    set_balances_for_1_1_exchange(
        &mut ctx,
        state_account,
        staker_orca_ata,
        staker_signer,
        orca_stake_amount,
    );

    let ix = Stake {
        staker_account: staker_signer,
        state_account: state_account,
        vault_account: vault_account,
        staker_orca_ata: staker_orca_ata,
        staker_xorca_ata: staker_xorca_ata,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        token_program_account: TOKEN_PROGRAM_ID,
    }
    .instruction(StakeInstructionArgs { orca_stake_amount });
    let result = ctx.send(ix);
    assert_program_success!(result);
    let vault_account_after = ctx.get_account::<TokenAccount>(vault_account).unwrap();
    let staker_orca_ata_after = ctx.get_account::<TokenAccount>(staker_orca_ata).unwrap();
    let staker_xorca_ata_after = ctx.get_account::<TokenAccount>(staker_xorca_ata).unwrap();
    let xorca_mint_account_after = ctx.get_account::<TokenMint>(XORCA_ID).unwrap();
    let state_account_after = ctx.get_account::<State>(state_account).unwrap();
    assert_eq!(
        vault_account_after.data.amount, 1_000_000,
        "Vault account should have 1 ORCA"
    );
    assert_eq!(
        staker_orca_ata_after.data.amount, 0,
        "Staker Orca ATA should have 0 ORCA"
    );
    assert_eq!(
        staker_xorca_ata_after.data.amount,
        1_000_000_000, // 1 xORCA (9 decimals)
        "Staker xOrca ATA should have 1 xORCA"
    );
    assert_eq!(
        xorca_mint_account_after.data.supply,
        1_000_000_000, // 1 xORCA (9 decimals)
        "xOrca supply should be 1 xORCA"
    );
    assert_eq!(
        state_account_after.data.escrowed_orca_amount, 0,
        "Escrowed Orca amount should be unchanged (0 ORCA)"
    );
}

#[test]
fn test_stake_success_more_than_1_1_exchange() {
    let mut ctx = TestContext::new();
    let (state_account, vault_account, staker_orca_ata, staker_xorca_ata, staker_signer) =
        setup_base_stake_context(&mut ctx);
    let orca_stake_amount = 2_384_964; // 2.384964 ORCA
    set_balances_for_more_than_1_1_exchange(
        &mut ctx,
        state_account,
        vault_account,
        staker_orca_ata,
        staker_xorca_ata,
        staker_signer,
        5_123_538, // Initial staker ORCA amount
    );
    let ix = Stake {
        staker_account: staker_signer,
        state_account: state_account,
        vault_account: vault_account,
        staker_orca_ata: staker_orca_ata,
        staker_xorca_ata: staker_xorca_ata,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        token_program_account: TOKEN_PROGRAM_ID,
    }
    .instruction(StakeInstructionArgs { orca_stake_amount });
    let result = ctx.send(ix);
    assert_program_success!(result);
    let vault_account_after = ctx.get_account::<TokenAccount>(vault_account).unwrap();
    let staker_orca_ata_after = ctx.get_account::<TokenAccount>(staker_orca_ata).unwrap();
    let staker_xorca_ata_after = ctx.get_account::<TokenAccount>(staker_xorca_ata).unwrap();
    let xorca_mint_account_after = ctx.get_account::<TokenMint>(XORCA_ID).unwrap();
    let state_account_after = ctx.get_account::<State>(state_account).unwrap();
    assert_eq!(
        vault_account_after.data.amount,
        923_386_653_551, // 923,384.268587 + 2.384964 ORCA = 923,386.653551
        "Vault account amount incorrect"
    );
    assert_eq!(
        staker_orca_ata_after.data.amount,
        2_738_574, // 5.123538 - 2.384964 ORCA = 2.738574 ORCA
        "Staker Orca ATA amount incorrect"
    );
    assert_eq!(
        staker_xorca_ata_after.data.amount,
        13_790_387_622, // 12.823658283 + (2.384964 ORCA * current rate) = 13.790387622 xORCA
        "Staker xOrca ATA amount incorrect"
    );
    assert_eq!(
        xorca_mint_account_after.data.supply,
        358_385_826_550_562, // 358,384.859821223 + 0.966729339339 xORCA (minted) = 358,385.826550562 xORCA
        "xOrca supply incorrect"
    );
    assert_eq!(
        state_account_after.data.escrowed_orca_amount, 39_232_982_923,
        "Escrowed Orca amount should be unchanged"
    );
}

#[test]
fn test_stake_invalid_state_account_owner() {
    let mut ctx = TestContext::new();
    let (state_account, vault_account, staker_orca_ata, staker_xorca_ata, staker_signer) =
        setup_base_stake_context(&mut ctx);
    let orca_stake_amount = 1_000_000;
    set_balances_for_1_1_exchange(
        &mut ctx,
        state_account,
        staker_orca_ata,
        staker_signer,
        orca_stake_amount,
    );
    make_state_account_invalid_owner(&mut ctx, state_account);
    let ix = Stake {
        staker_account: staker_signer,
        state_account: state_account,
        vault_account: vault_account,
        staker_orca_ata: staker_orca_ata,
        staker_xorca_ata: staker_xorca_ata,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        token_program_account: TOKEN_PROGRAM_ID,
    }
    .instruction(StakeInstructionArgs { orca_stake_amount });
    let result = ctx.send(ix);
    assert_program_error!(result, XorcaStakingProgramError::IncorrectOwner);
}

#[test]
fn test_stake_invalid_state_account_seeds() {
    let mut ctx = TestContext::new();
    let (correct_state_account, vault_account, staker_orca_ata, staker_xorca_ata, staker_signer) =
        setup_base_stake_context(&mut ctx);
    let orca_stake_amount = 1_000_000;
    set_balances_for_1_1_exchange(
        &mut ctx,
        correct_state_account,
        staker_orca_ata,
        staker_signer,
        orca_stake_amount,
    );
    let invalid_state_account = make_state_account_invalid_seeds(&mut ctx);
    let ix = Stake {
        staker_account: staker_signer,
        state_account: invalid_state_account,
        vault_account: vault_account,
        staker_orca_ata: staker_orca_ata,
        staker_xorca_ata: staker_xorca_ata,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        token_program_account: TOKEN_PROGRAM_ID,
    }
    .instruction(StakeInstructionArgs { orca_stake_amount });
    let result = ctx.send(ix);
    assert_program_error!(result, XorcaStakingProgramError::InvalidSeeds);
}

#[test]
fn test_stake_invalid_staker_orca_ata_owner_data() {
    let mut ctx = TestContext::new();
    let (state_account, vault_account, staker_orca_ata, staker_xorca_ata, staker_signer) =
        setup_base_stake_context(&mut ctx);
    let orca_stake_amount = 1_000_000;
    set_balances_for_1_1_exchange(
        &mut ctx,
        state_account,
        staker_orca_ata,
        staker_signer,
        orca_stake_amount,
    );
    make_staker_orca_ata_invalid_owner_in_data(&mut ctx, staker_orca_ata);
    let ix = Stake {
        staker_account: staker_signer,
        state_account: state_account,
        vault_account: vault_account,
        staker_orca_ata: staker_orca_ata,
        staker_xorca_ata: staker_xorca_ata,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        token_program_account: TOKEN_PROGRAM_ID,
    }
    .instruction(StakeInstructionArgs { orca_stake_amount });
    let result = ctx.send(ix);
    assert_program_error!(result, XorcaStakingProgramError::InvalidAccountData);
}

#[test]
fn test_stake_invalid_staker_orca_ata_mint_data() {
    let mut ctx = TestContext::new();
    let (state_account, vault_account, staker_orca_ata, staker_xorca_ata, staker_signer) =
        setup_base_stake_context(&mut ctx);
    let orca_stake_amount = 1_000_000;
    set_balances_for_1_1_exchange(
        &mut ctx,
        state_account,
        staker_orca_ata,
        staker_signer,
        orca_stake_amount,
    );
    make_staker_orca_ata_invalid_mint_in_data(&mut ctx, staker_orca_ata);
    let ix = Stake {
        staker_account: staker_signer,
        state_account: state_account,
        vault_account: vault_account,
        staker_orca_ata: staker_orca_ata,
        staker_xorca_ata: staker_xorca_ata,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        token_program_account: TOKEN_PROGRAM_ID,
    }
    .instruction(StakeInstructionArgs { orca_stake_amount });
    let result = ctx.send(ix);
    assert_program_error!(result, XorcaStakingProgramError::InvalidAccountData);
}

#[test]
fn test_stake_invalid_staker_orca_ata_program_owner() {
    let mut ctx = TestContext::new();
    let (state_account, vault_account, staker_orca_ata, staker_xorca_ata, staker_signer) =
        setup_base_stake_context(&mut ctx);
    let orca_stake_amount = 1_000_000;
    set_balances_for_1_1_exchange(
        &mut ctx,
        state_account,
        staker_orca_ata,
        staker_signer,
        orca_stake_amount,
    );
    make_staker_orca_ata_invalid_program_owner(&mut ctx, staker_orca_ata);
    let ix = Stake {
        staker_account: staker_signer,
        state_account: state_account,
        vault_account: vault_account,
        staker_orca_ata: staker_orca_ata,
        staker_xorca_ata: staker_xorca_ata,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        token_program_account: TOKEN_PROGRAM_ID,
    }
    .instruction(StakeInstructionArgs { orca_stake_amount });
    let result = ctx.send(ix);
    assert_program_error!(result, XorcaStakingProgramError::IncorrectOwner);
}

#[test]
fn test_stake_invalid_staker_xorca_ata_owner_data() {
    let mut ctx = TestContext::new();
    let (state_account, vault_account, staker_orca_ata, staker_xorca_ata, staker_signer) =
        setup_base_stake_context(&mut ctx);

    let orca_stake_amount = 1_000_000;
    set_balances_for_1_1_exchange(
        &mut ctx,
        state_account,
        staker_orca_ata,
        staker_signer,
        orca_stake_amount,
    );
    make_staker_xorca_ata_invalid_owner_in_data(&mut ctx, staker_xorca_ata);
    let ix = Stake {
        staker_account: staker_signer,
        state_account: state_account,
        vault_account: vault_account,
        staker_orca_ata: staker_orca_ata,
        staker_xorca_ata: staker_xorca_ata,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        token_program_account: TOKEN_PROGRAM_ID,
    }
    .instruction(StakeInstructionArgs { orca_stake_amount });
    let result = ctx.send(ix);
    assert_program_error!(result, XorcaStakingProgramError::InvalidAccountData);
}

#[test]
fn test_stake_invalid_staker_xorca_ata_mint_data() {
    let mut ctx = TestContext::new();
    let (state_account, vault_account, staker_orca_ata, staker_xorca_ata, staker_signer) =
        setup_base_stake_context(&mut ctx);
    let orca_stake_amount = 1_000_000;
    set_balances_for_1_1_exchange(
        &mut ctx,
        state_account,
        staker_orca_ata,
        staker_signer,
        orca_stake_amount,
    );
    make_staker_xorca_ata_invalid_mint_in_data(&mut ctx, staker_xorca_ata);
    let ix = Stake {
        staker_account: staker_signer,
        state_account: state_account,
        vault_account: vault_account,
        staker_orca_ata: staker_orca_ata,
        staker_xorca_ata: staker_xorca_ata,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        token_program_account: TOKEN_PROGRAM_ID,
    }
    .instruction(StakeInstructionArgs { orca_stake_amount });
    let result = ctx.send(ix);
    assert_program_error!(result, XorcaStakingProgramError::InvalidAccountData);
}

#[test]
fn test_stake_invalid_staker_xorca_ata_program_owner() {
    let mut ctx = TestContext::new();
    let (state_account, vault_account, staker_orca_ata, staker_xorca_ata, staker_signer) =
        setup_base_stake_context(&mut ctx);
    let orca_stake_amount = 1_000_000;
    set_balances_for_1_1_exchange(
        &mut ctx,
        state_account,
        staker_orca_ata,
        staker_signer,
        orca_stake_amount,
    );
    make_staker_xorca_ata_invalid_program_owner(&mut ctx, staker_xorca_ata);
    let ix = Stake {
        staker_account: staker_signer,
        state_account: state_account,
        vault_account: vault_account,
        staker_orca_ata: staker_orca_ata,
        staker_xorca_ata: staker_xorca_ata,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        token_program_account: TOKEN_PROGRAM_ID,
    }
    .instruction(StakeInstructionArgs { orca_stake_amount });
    let result = ctx.send(ix);
    assert_program_error!(result, XorcaStakingProgramError::IncorrectOwner);
}
