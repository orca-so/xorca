use crate::{
    state_data, token_account_data, token_mint_data, TestContext, ATA_PROGRAM_ID,
    INITIAL_UPDATE_AUTHORITY_ID, ORCA_ID, SYSTEM_PROGRAM_ID, TOKEN_PROGRAM_ID, XORCA_ID,
    XORCA_PROGRAM_ID,
};
use core::error::Error;
use rstest::rstest;
use solana_sdk::{clock::Clock, pubkey::Pubkey};
use xorca::{
    find_pending_withdraw_pda, find_state_address, Initialize, InitializeInstructionArgs, Set,
    SetInstructionArgs, Stake, StakeInstructionArgs, State, StateUpdateInstruction, TokenAccount,
    Unstake, UnstakeInstructionArgs, Withdraw, WithdrawInstructionArgs,
};

const ORCA_XORCA_RATIO: u64 = 1_000;

fn get_state_account() -> Result<Pubkey, Box<dyn Error>> {
    let state_account = find_state_address().unwrap().0;
    Ok(state_account)
}

fn get_ata(owner: Pubkey, mint: Pubkey) -> Result<Pubkey, Box<dyn Error>> {
    let ata = Pubkey::find_program_address(
        &[
            &owner.to_bytes(),
            &TOKEN_PROGRAM_ID.to_bytes(),
            &mint.to_bytes(),
        ],
        &ATA_PROGRAM_ID,
    )
    .0;
    Ok(ata)
}

fn initialize_state_account(
    ctx: &mut TestContext,
    cool_down_period_s: i64,
) -> Result<Pubkey, Box<dyn Error>> {
    let state_account = find_state_address().unwrap().0;
    let ix = Initialize {
        payer_account: ctx.signer(),
        state_account,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        update_authority_account: INITIAL_UPDATE_AUTHORITY_ID,
        system_program_account: SYSTEM_PROGRAM_ID,
    }
    .instruction(InitializeInstructionArgs { cool_down_period_s });
    ctx.send(ix).map_err(|e| format!("{:?}", e))?;
    Ok(state_account)
}

fn initialize_token_mints(ctx: &mut TestContext) {
    let state_account = get_state_account().unwrap();
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
            supply => 1_000_000_000_000, // Total 1,000,000 ORCA supply example
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

fn initialize_atas(ctx: &mut TestContext, initial_staker_orca_amount: u64) {
    let state_account = get_state_account().unwrap();
    let vault_account = get_ata(state_account, ORCA_ID).unwrap();
    let staker_orca_ata = get_ata(ctx.signer(), ORCA_ID).unwrap();
    let staker_xorca_ata = get_ata(ctx.signer(), XORCA_ID).unwrap();
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
    ctx.write_account(
        staker_orca_ata,
        TOKEN_PROGRAM_ID,
        token_account_data!(
            mint => ORCA_ID,
            owner => ctx.signer(),
            amount => initial_staker_orca_amount,
        ),
    )
    .unwrap();
    ctx.write_account(
        staker_xorca_ata,
        TOKEN_PROGRAM_ID,
        token_account_data!(
            mint => XORCA_ID,
            owner => ctx.signer(),
            amount => 0, // Initial staker XORCA amount 0
        ),
    )
    .unwrap();
}

fn move_clock_forward(ctx: &mut TestContext, time_to_add: i64) -> Result<(), Box<dyn Error>> {
    let mut clock: Clock = ctx.svm.get_sysvar::<Clock>();
    clock.unix_timestamp += time_to_add;
    ctx.svm.set_sysvar::<Clock>(&clock);
    Ok(())
}

fn deposit_orca_to_vault(ctx: &mut TestContext, deposit_amount: u64) -> Result<(), Box<dyn Error>> {
    let state_account = get_state_account().unwrap();
    let vault_account = get_ata(state_account, ORCA_ID).unwrap();
    let vault_data = ctx.get_account::<TokenAccount>(vault_account).unwrap();
    ctx.write_account(
        vault_account,
        TOKEN_PROGRAM_ID,
        token_account_data!(
            mint => ORCA_ID,
            owner => state_account,
            amount => vault_data.data.amount + deposit_amount,
        ),
    )
    .unwrap();
    Ok(())
}

fn stake_orca(ctx: &mut TestContext, orca_stake_amount: u64) -> Result<(), Box<dyn Error>> {
    let state_account = find_state_address().unwrap().0;
    let vault_account = get_ata(state_account, ORCA_ID)?;
    let staker_orca_ata = get_ata(ctx.signer(), ORCA_ID)?;
    let staker_xorca_ata = get_ata(ctx.signer(), XORCA_ID)?;
    let ix = Stake {
        staker_account: ctx.signer(),
        state_account: state_account,
        vault_account: vault_account,
        staker_orca_ata: staker_orca_ata,
        staker_xorca_ata: staker_xorca_ata,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        token_program_account: TOKEN_PROGRAM_ID,
    }
    .instruction(StakeInstructionArgs { orca_stake_amount });
    ctx.send(ix).map_err(|e| format!("{:?}", e))?;
    Ok(())
}

fn unstake_xorca(
    ctx: &mut TestContext,
    xorca_unstake_amount: u64,
    withdraw_index: u8,
) -> Result<(), Box<dyn Error>> {
    let state_account = find_state_address().unwrap().0;
    let vault_account = get_ata(state_account, ORCA_ID)?;
    let unstaker_xorca_ata = get_ata(ctx.signer(), XORCA_ID)?;
    let pending_withdraw_account = find_pending_withdraw_pda(&ctx.signer(), &withdraw_index)?.0;
    let ix = Unstake {
        unstaker_account: ctx.signer(),
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
        xorca_unstake_amount,
        withdraw_index,
    });
    ctx.send(ix).map_err(|e| format!("{:?}", e))?;
    Ok(())
}

fn withdraw_orca(ctx: &mut TestContext, withdraw_index: u8) -> Result<(), Box<dyn Error>> {
    let state_account = find_state_address().unwrap().0;
    let vault_account = get_ata(state_account, ORCA_ID)?;
    let unstaker_orca_ata = get_ata(ctx.signer(), ORCA_ID)?;
    let pending_withdraw_account = find_pending_withdraw_pda(&ctx.signer(), &withdraw_index)?.0;
    let ix = Withdraw {
        unstaker_account: ctx.signer(),
        state_account,
        vault_account,
        pending_withdraw_account,
        unstaker_orca_ata,
        orca_mint_account: ORCA_ID,
        system_program_account: SYSTEM_PROGRAM_ID,
        token_program_account: TOKEN_PROGRAM_ID,
    }
    .instruction(WithdrawInstructionArgs { withdraw_index });
    ctx.send(ix).map_err(|e| format!("{:?}", e))?;
    Ok(())
}

fn update_cool_down_period(
    ctx: &mut TestContext,
    new_cool_down_period_s: i64,
) -> Result<(), Box<dyn Error>> {
    let state_account = find_state_address().unwrap().0;
    let state = ctx.get_account::<State>(state_account).unwrap();
    // Update authority by writing to account
    ctx.write_account(
        state_account,
        XORCA_PROGRAM_ID,
        state_data!(
            escrowed_orca_amount => state.data.escrowed_orca_amount,
            update_authority => ctx.signer(),
            cool_down_period_s => state.data.cool_down_period_s
        ),
    )
    .unwrap();
    let ix = Set {
        update_authority_account: ctx.signer(),
        state_account: state_account,
    }
    .instruction(SetInstructionArgs {
        instruction_data: StateUpdateInstruction::UpdateCoolDownPeriod {
            new_cool_down_period_s: new_cool_down_period_s,
        },
    });
    ctx.send(ix).map_err(|e| format!("{:?}", e))?;
    Ok(())
}

#[rstest]
#[case(100_000_000, 60 * 60 * 24 * 7, 0)] // 100 ORCA, 7 days, index 0
#[case(1_000_000, 60 * 60 * 24, 0)] // 1 ORCA, 1 day, index 0
#[case(10_000_000_000, 60 * 60 * 24 * 30, 0)] // 10k ORCA, 30 days, index 0
#[case(100_000_000, 60 * 60 * 24 * 7, 1)] // 100 ORCA, 7 days, index 1
#[case(1_000_000_000, 60 * 60 * 24 * 14, 2)] // 1k ORCA, 14 days, index 2
#[case(1, 60, 0)] // 1 lamport ORCA, 1 minute, index 0
#[case(u64::MAX / 1000, 60 * 60 * 24 * 365, 255)] // Large amount, 1 year, max index
pub fn test_e2e_stake_unstake_withdraw_1_1(
    #[case] orca_stake_amount: u64,
    #[case] cool_down_period_s: i64,
    #[case] withdraw_index: u8,
) {
    let mut ctx = TestContext::new();
    initialize_atas(&mut ctx, orca_stake_amount);
    initialize_token_mints(&mut ctx);
    initialize_state_account(&mut ctx, cool_down_period_s).unwrap();
    stake_orca(&mut ctx, orca_stake_amount).unwrap();
    unstake_xorca(
        &mut ctx,
        orca_stake_amount * ORCA_XORCA_RATIO,
        withdraw_index,
    )
    .unwrap();
    move_clock_forward(&mut ctx, cool_down_period_s).unwrap();
    withdraw_orca(&mut ctx, withdraw_index).unwrap();
}

#[rstest]
#[case(100_000_000, 60 * 60 * 24 * 7, 0, 100_000_000, 100_000_000, 60 * 60 * 24 * 30, 1)]
#[case(1_000_000, 60 * 60 * 24, 2, 500_000, 2_000_000, 60 * 60 * 24 * 14, 3)]
#[case(10_000_000_000, 60 * 60 * 24 * 30, 4, 5_000_000_000, 1_000_000_000, 60 * 60 * 24 * 60, 5)]
pub fn test_e2e_stake_unstake_withdraw_more_than_1_1(
    #[case] orca_stake_amount_1: u64,
    #[case] cool_down_period_s_1: i64,
    #[case] withdraw_index_1: u8,
    #[case] orca_vault_deposit_amount: u64,
    #[case] orca_stake_amount_2: u64,
    #[case] cool_down_period_s_2: i64,
    #[case] withdraw_index_2: u8,
) {
    // Enforce the constraint
    assert_ne!(
        withdraw_index_1, withdraw_index_2,
        "withdraw_index_1 and withdraw_index_2 must be different"
    );

    let mut ctx = TestContext::new();
    // Initialize accounts
    initialize_atas(&mut ctx, orca_stake_amount_1 + orca_stake_amount_2);
    initialize_token_mints(&mut ctx);
    initialize_state_account(&mut ctx, cool_down_period_s_1).unwrap();
    // 1_1 stake unstake withdraw
    stake_orca(&mut ctx, orca_stake_amount_1).unwrap();
    unstake_xorca(
        &mut ctx,
        orca_stake_amount_1 * ORCA_XORCA_RATIO,
        withdraw_index_1,
    )
    .unwrap();
    // Intermediate operations
    deposit_orca_to_vault(&mut ctx, orca_vault_deposit_amount).unwrap();
    update_cool_down_period(&mut ctx, cool_down_period_s_2).unwrap();
    // more than 1_1 stake unstake withdraw
    stake_orca(&mut ctx, orca_stake_amount_2).unwrap();
    unstake_xorca(
        &mut ctx,
        orca_stake_amount_2 * ORCA_XORCA_RATIO,
        withdraw_index_2,
    )
    .unwrap();
    // Withdraw
    move_clock_forward(&mut ctx, cool_down_period_s_1).unwrap();
    withdraw_orca(&mut ctx, withdraw_index_1).unwrap();
    move_clock_forward(&mut ctx, cool_down_period_s_2).unwrap();
    withdraw_orca(&mut ctx, withdraw_index_2).unwrap();
}
