use crate::utils::fixture::Env;
use crate::{ORCA_ID, SYSTEM_PROGRAM_ID, TOKEN_PROGRAM_ID, XORCA_ID};
use litesvm::types::TransactionResult;
use solana_sdk::{clock::Clock, pubkey::Pubkey};
use xorca::{
    find_pending_withdraw_pda, Stake, StakeInstructionArgs, Unstake, UnstakeInstructionArgs,
    Withdraw, WithdrawInstructionArgs,
};

pub fn unstake_and_advance(
    env: &mut Env,
    withdraw_index: u8,
    xorca_unstake_amount: u64,
    advance_secs: i64,
) -> Pubkey {
    let pending_withdraw_account = find_pending_withdraw_pda(&env.staker, &withdraw_index)
        .unwrap()
        .0;
    let ix_unstake = Unstake {
        unstaker_account: env.staker,
        state_account: env.state,
        vault_account: env.vault,
        pending_withdraw_account,
        unstaker_xorca_ata: env.staker_xorca_ata,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        system_program_account: SYSTEM_PROGRAM_ID,
        token_program_account: TOKEN_PROGRAM_ID,
    }
    .instruction(UnstakeInstructionArgs {
        xorca_unstake_amount,
        withdraw_index,
    });
    let _ = env.ctx.send(ix_unstake);
    advance_clock_env(env, advance_secs);
    pending_withdraw_account
}

pub fn do_withdraw(
    env: &mut Env,
    pending_withdraw_account: Pubkey,
    withdraw_index: u8,
) -> TransactionResult {
    let ix = Withdraw {
        unstaker_account: env.staker,
        state_account: env.state,
        vault_account: env.vault,
        pending_withdraw_account: pending_withdraw_account,
        unstaker_orca_ata: env.staker_orca_ata,
        orca_mint_account: ORCA_ID,
        system_program_account: SYSTEM_PROGRAM_ID,
        token_program_account: TOKEN_PROGRAM_ID,
    }
    .instruction(WithdrawInstructionArgs { withdraw_index });
    env.ctx.send(ix)
}

pub fn do_unstake(
    env: &mut Env,
    withdraw_index: u8,
    xorca_unstake_amount: u64,
) -> TransactionResult {
    let pending_withdraw_account = find_pending_withdraw_pda(&env.staker, &withdraw_index)
        .unwrap()
        .0;
    let ix_unstake = Unstake {
        unstaker_account: env.staker,
        state_account: env.state,
        vault_account: env.vault,
        pending_withdraw_account: pending_withdraw_account,
        unstaker_xorca_ata: env.staker_xorca_ata,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        system_program_account: SYSTEM_PROGRAM_ID,
        token_program_account: TOKEN_PROGRAM_ID,
    }
    .instruction(UnstakeInstructionArgs {
        xorca_unstake_amount,
        withdraw_index,
    });
    env.ctx.send(ix_unstake)
}

pub fn advance_clock_env(env: &mut Env, advance_secs: i64) {
    if advance_secs == 0 {
        return;
    }
    let mut clock = env.ctx.svm.get_sysvar::<Clock>();
    clock.unix_timestamp += advance_secs;
    env.ctx.svm.set_sysvar::<Clock>(&clock);
}

pub fn stake_orca(env: &mut Env, orca_amount: u64, label: &str) {
    let ix = Stake {
        staker_account: env.staker,
        state_account: env.state,
        vault_account: env.vault,
        staker_orca_ata: env.staker_orca_ata,
        staker_xorca_ata: env.staker_xorca_ata,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        token_program_account: TOKEN_PROGRAM_ID,
    }
    .instruction(StakeInstructionArgs {
        orca_stake_amount: orca_amount,
    });
    assert!(env.ctx.send(ix).is_ok(), "{}: stake should succeed", label);
}

pub fn deposit_yield_into_vault(env: &mut Env, orca_amount: u64, label: &str) {
    let before = env
        .ctx
        .get_account::<xorca::TokenAccount>(env.vault)
        .unwrap()
        .data
        .amount;
    env.ctx
        .write_account(
            env.vault,
            TOKEN_PROGRAM_ID,
            crate::token_account_data!(mint => ORCA_ID, owner => env.state, amount => before.saturating_add(orca_amount)),
        )
        .unwrap();
    let after = env
        .ctx
        .get_account::<xorca::TokenAccount>(env.vault)
        .unwrap()
        .data
        .amount;
    assert_eq!(
        after,
        before.saturating_add(orca_amount),
        "{}: vault ORCA increased by deposit",
        label
    );
}
