use crate::TestContext;
use borsh::BorshDeserialize;
use litesvm::types::TransactionResult;
use solana_sdk::pubkey::Pubkey;
use xorca::PendingWithdraw;
use xorca::{AccountDiscriminator, Event, State, TokenAccount, TokenMint};

pub struct ExpectedTokenAccount<'a> {
    pub owner: &'a Pubkey,
    pub mint: &'a Pubkey,
    pub amount: u64,
    pub label: &'a str,
}

pub struct ExpectedMint<'a> {
    pub decimals: u8,
    pub supply: u64,
    pub mint_authority: &'a Pubkey,
    pub label: &'a str,
}

pub struct ExpectedState {
    pub escrowed_orca_amount: u64,
    pub cool_down_period_s: i64,
}

pub fn assert_token_account(ctx: &TestContext, address: Pubkey, expected: ExpectedTokenAccount) {
    let acc = ctx.get_account::<TokenAccount>(address).unwrap();
    assert_eq!(
        acc.data.owner, *expected.owner,
        "{}: token owner",
        expected.label
    );
    assert_eq!(
        acc.data.mint, *expected.mint,
        "{}: token mint",
        expected.label
    );
    assert_eq!(
        acc.data.amount, expected.amount,
        "{}: token amount",
        expected.label
    );
}

pub fn assert_mint(ctx: &TestContext, address: Pubkey, expected: ExpectedMint) {
    let mint = ctx.get_account::<TokenMint>(address).unwrap();
    assert_eq!(
        mint.data.decimals, expected.decimals,
        "{}: mint decimals",
        expected.label
    );
    assert_eq!(
        mint.data.supply, expected.supply,
        "{}: mint supply",
        expected.label
    );
    assert_eq!(
        mint.data.mint_authority, *expected.mint_authority,
        "{}: mint authority",
        expected.label
    );
}

pub fn assert_state(ctx: &TestContext, address: Pubkey, expected: ExpectedState) {
    let state_account = ctx.get_account::<State>(address).unwrap();
    assert_eq!(
        state_account.data.escrowed_orca_amount, expected.escrowed_orca_amount,
        "state: escrowed ORCA amount"
    );
    assert_eq!(
        state_account.data.cool_down_period_s, expected.cool_down_period_s,
        "state: cooldown period seconds"
    );
}

pub fn assert_account_closed(ctx: &TestContext, address: Pubkey, label: &str) {
    match ctx.get_raw_account(address) {
        Err(_) => return, // fully removed is acceptable
        Ok(acc) => {
            let discriminator = acc.data.first().copied().unwrap_or(255);
            let is_closed =
                discriminator == AccountDiscriminator::Closed as u8 || acc.lamports == 0;
            assert!(is_closed, "{}: account should be closed", label);
        }
    }
}

// === Event decoding helpers ===

fn try_decode_base64(s: &str) -> Option<Vec<u8>> {
    use base64::{engine::general_purpose, Engine as _};
    general_purpose::STANDARD.decode(s.trim()).ok()
}

fn try_decode_hex(s: &str) -> Option<Vec<u8>> {
    let cleaned: String = s.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    if cleaned.len() % 2 != 0 || cleaned.is_empty() {
        return None;
    }
    let mut out = Vec::with_capacity(cleaned.len() / 2);
    let bytes = cleaned.as_bytes();
    for i in (0..bytes.len()).step_by(2) {
        let hi = (bytes[i] as char).to_digit(16)? as u8;
        let lo = (bytes[i + 1] as char).to_digit(16)? as u8;
        out.push((hi << 4) | lo);
    }
    Some(out)
}

fn collect_program_data_payloads(result: &TransactionResult) -> Vec<Vec<u8>> {
    let logs: &Vec<String> = match result {
        Ok(meta) => &meta.logs,
        Err(e) => &e.meta.logs,
    };
    let mut payloads = Vec::new();
    for line in logs {
        if let Some(pos) = line.find("Program data:") {
            let payload = line[(pos + "Program data:".len())..].trim();
            if let Some(bytes) = try_decode_base64(payload).or_else(|| try_decode_hex(payload)) {
                payloads.push(bytes);
            }
        }
    }
    payloads
}

pub fn decode_events_from_result(result: &TransactionResult) -> Vec<Event> {
    collect_program_data_payloads(result)
        .into_iter()
        .filter_map(|bytes| Event::deserialize(&mut bytes.as_slice()).ok())
        .collect()
}

// === Human-readable stake assertions ===

pub struct StakeSnapshot {
    pub escrow_before: u64,
    pub vault_before: u64,
    pub user_orca_before: u64,
    pub user_xorca_before: u64,
    pub xorca_supply_before: u64,
}

pub fn take_stake_snapshot(
    ctx: &TestContext,
    state: Pubkey,
    vault: Pubkey,
    user_orca: Pubkey,
    user_xorca: Pubkey,
    xorca_mint: Pubkey,
) -> StakeSnapshot {
    let escrow_before = ctx
        .get_account::<State>(state)
        .unwrap()
        .data
        .escrowed_orca_amount;
    let vault_before = ctx.get_account::<TokenAccount>(vault).unwrap().data.amount;
    let user_orca_before = ctx
        .get_account::<TokenAccount>(user_orca)
        .unwrap()
        .data
        .amount;
    let user_xorca_before = ctx
        .get_account::<TokenAccount>(user_xorca)
        .unwrap()
        .data
        .amount;
    let xorca_supply_before = ctx
        .get_account::<TokenMint>(xorca_mint)
        .unwrap()
        .data
        .supply;
    StakeSnapshot {
        escrow_before,
        vault_before,
        user_orca_before,
        user_xorca_before,
        xorca_supply_before,
    }
}

pub fn assert_stake_effects(
    ctx: &TestContext,
    state: Pubkey,
    vault: Pubkey,
    user_orca: Pubkey,
    user_xorca: Pubkey,
    xorca_mint: Pubkey,
    snap: &StakeSnapshot,
    orca_stake_amount: u64,
    xorca_minted_amount: u64,
    expected_cool_down_period_s: i64,
    label: &str,
) {
    // Escrow and cooldown period remain consistent
    let state_after = ctx.get_account::<State>(state).unwrap();
    assert_eq!(
        state_after.data.escrowed_orca_amount, snap.escrow_before,
        "{}: escrow unchanged after stake",
        label
    );
    assert_eq!(
        state_after.data.cool_down_period_s, expected_cool_down_period_s,
        "{}: cooldown period unchanged",
        label
    );

    // Vault should increase by the staked ORCA
    let vault_after = ctx.get_account::<TokenAccount>(vault).unwrap().data.amount;
    assert_eq!(
        vault_after,
        snap.vault_before.saturating_add(orca_stake_amount),
        "{}: vault after stake",
        label
    );

    // User ORCA should decrease by the staked amount
    let user_orca_after = ctx
        .get_account::<TokenAccount>(user_orca)
        .unwrap()
        .data
        .amount;
    assert_eq!(
        user_orca_after,
        snap.user_orca_before.saturating_sub(orca_stake_amount),
        "{}: user ORCA after stake",
        label
    );

    // User xORCA should increase by the minted amount
    let user_xorca_after = ctx
        .get_account::<TokenAccount>(user_xorca)
        .unwrap()
        .data
        .amount;
    assert_eq!(
        user_xorca_after,
        snap.user_xorca_before.saturating_add(xorca_minted_amount),
        "{}: user xORCA after stake",
        label
    );

    // xORCA mint supply should increase by minted amount and decimals/authority remain correct
    let xorca_supply_after = ctx
        .get_account::<TokenMint>(xorca_mint)
        .unwrap()
        .data
        .supply;
    assert_eq!(
        xorca_supply_after,
        snap.xorca_supply_before.saturating_add(xorca_minted_amount),
        "{}: xORCA supply after stake",
        label
    );
    let xorca_mint_after = ctx.get_account::<TokenMint>(xorca_mint).unwrap();
    assert_eq!(
        xorca_mint_after.data.decimals, 6,
        "{}: xORCA mint decimals",
        label
    );
    assert_eq!(
        xorca_mint_after.data.mint_authority, state,
        "{}: xORCA mint authority",
        label
    );
}

// === Human-readable withdraw assertions ===

pub struct WithdrawSnapshot {
    pub escrow_before: u64,
    pub vault_before: u64,
    pub user_orca_before: u64,
    pub user_xorca_before: u64,
    pub xorca_supply_before: u64,
}

pub fn take_withdraw_snapshot(
    ctx: &TestContext,
    state: Pubkey,
    vault: Pubkey,
    user_orca: Pubkey,
    user_xorca: Pubkey,
    xorca_mint: Pubkey,
) -> WithdrawSnapshot {
    let escrow_before = ctx
        .get_account::<State>(state)
        .unwrap()
        .data
        .escrowed_orca_amount;
    let vault_before = ctx.get_account::<TokenAccount>(vault).unwrap().data.amount;
    let user_orca_before = ctx
        .get_account::<TokenAccount>(user_orca)
        .unwrap()
        .data
        .amount;
    let user_xorca_before = ctx
        .get_account::<TokenAccount>(user_xorca)
        .unwrap()
        .data
        .amount;
    let xorca_supply_before = ctx
        .get_account::<TokenMint>(xorca_mint)
        .unwrap()
        .data
        .supply;
    WithdrawSnapshot {
        escrow_before,
        vault_before,
        user_orca_before,
        user_xorca_before,
        xorca_supply_before,
    }
}

pub fn assert_withdraw_effects(
    ctx: &TestContext,
    state: Pubkey,
    vault: Pubkey,
    user_orca: Pubkey,
    user_xorca: Pubkey,
    xorca_mint: Pubkey,
    snap: &WithdrawSnapshot,
    pending_orca_amount: u64,
    xorca_unstake_amount: u64,
    label: &str,
) {
    // State escrow decreased by pending amount
    let escrow_after = ctx
        .get_account::<State>(state)
        .unwrap()
        .data
        .escrowed_orca_amount;
    assert_eq!(
        escrow_after,
        snap.escrow_before.saturating_sub(pending_orca_amount),
        "{}: state escrow after",
        label
    );

    // Vault decreased by pending amount
    let vault_after = ctx.get_account::<TokenAccount>(vault).unwrap().data.amount;
    assert_eq!(
        vault_after,
        snap.vault_before.saturating_sub(pending_orca_amount),
        "{}: vault after",
        label
    );

    // User ORCA increased by pending amount
    let user_orca_after = ctx
        .get_account::<TokenAccount>(user_orca)
        .unwrap()
        .data
        .amount;
    assert_eq!(
        user_orca_after,
        snap.user_orca_before.saturating_add(pending_orca_amount),
        "{}: user ORCA after",
        label
    );

    // xORCA supply should be unchanged by withdraw (xORCA was already burned during unstake)
    let xorca_supply_after = ctx
        .get_account::<TokenMint>(xorca_mint)
        .unwrap()
        .data
        .supply;
    assert_eq!(
        xorca_supply_after, snap.xorca_supply_before,
        "{}: xORCA supply after",
        label
    );

    // User xORCA should be unchanged by withdraw
    let user_xorca_after = ctx
        .get_account::<TokenAccount>(user_xorca)
        .unwrap()
        .data
        .amount;
    assert_eq!(
        user_xorca_after, snap.user_xorca_before,
        "{}: user xORCA after",
        label
    );

    // Additionally verify that the pending ORCA was computed from the xORCA burn at unstake time
    // using the program's exchange formula with pre-unstake values.
    // At withdraw-snapshot time, vault_before is unchanged since unstake; escrow_before already includes
    // the newly created pending amount; and xorca_supply_before is AFTER the burn. So reconstruct the
    // pre-unstake values:
    let non_escrowed_pre_unstake_u128 = (snap.vault_before as u128)
        .saturating_sub((snap.escrow_before as u128).saturating_sub(pending_orca_amount as u128));
    let supply_pre_unstake_u128 =
        (snap.xorca_supply_before as u128).saturating_add(xorca_unstake_amount as u128);
    if supply_pre_unstake_u128 > 0 {
        let expected_pending_u128 = (xorca_unstake_amount as u128)
            .saturating_mul(non_escrowed_pre_unstake_u128)
            / supply_pre_unstake_u128;
        let expected_pending = expected_pending_u128 as u64;
        assert_eq!(
            expected_pending, pending_orca_amount,
            "{}: pending amount matches xORCA burn and exchange rate",
            label
        );
    }
}

/// Basic withdraw assertions that do not recompute the pending amount from exchange rate.
/// Use this when vault balance may have changed between unstake and withdraw (e.g., yield deposit),
/// making the reconstruction from a withdraw-time snapshot invalid.
pub fn assert_withdraw_effects_basic(
    ctx: &TestContext,
    state: Pubkey,
    vault: Pubkey,
    user_orca: Pubkey,
    user_xorca: Pubkey,
    xorca_mint: Pubkey,
    snap: &WithdrawSnapshot,
    pending_orca_amount: u64,
    label: &str,
) {
    // State escrow decreased by pending amount
    let escrow_after = ctx
        .get_account::<State>(state)
        .unwrap()
        .data
        .escrowed_orca_amount;
    assert_eq!(
        escrow_after,
        snap.escrow_before.saturating_sub(pending_orca_amount),
        "{}: state escrow after",
        label
    );

    // Vault decreased by pending amount from whatever it was at snapshot time
    let vault_after = ctx.get_account::<TokenAccount>(vault).unwrap().data.amount;
    assert_eq!(
        vault_after,
        snap.vault_before.saturating_sub(pending_orca_amount),
        "{}: vault after",
        label
    );

    // User ORCA increased by pending amount
    let user_orca_after = ctx
        .get_account::<TokenAccount>(user_orca)
        .unwrap()
        .data
        .amount;
    assert_eq!(
        user_orca_after,
        snap.user_orca_before.saturating_add(pending_orca_amount),
        "{}: user ORCA after",
        label
    );

    // xORCA supply unchanged by withdraw
    let xorca_supply_after = ctx
        .get_account::<TokenMint>(xorca_mint)
        .unwrap()
        .data
        .supply;
    assert_eq!(
        xorca_supply_after, snap.xorca_supply_before,
        "{}: xORCA supply after",
        label
    );

    // User xORCA unchanged by withdraw
    let user_xorca_after = ctx
        .get_account::<TokenAccount>(user_xorca)
        .unwrap()
        .data
        .amount;
    assert_eq!(
        user_xorca_after, snap.user_xorca_before,
        "{}: user xORCA after",
        label
    );
}

// === Human-readable unstake assertions ===

pub fn assert_pending_withdraw(
    ctx: &TestContext,
    pending_withdraw_account: Pubkey,
    expected_unstaker: Pubkey,
    expected_withdrawable: u64,
    min_withdrawable_timestamp: i64,
    expected_withdraw_index: u8,
    label: &str,
) {
    let acc = ctx
        .get_account::<PendingWithdraw>(pending_withdraw_account)
        .unwrap();
    assert_eq!(
        acc.data.unstaker, expected_unstaker,
        "{}: pending unstaker",
        label
    );
    assert_eq!(
        acc.data.withdrawable_orca_amount, expected_withdrawable,
        "{}: pending withdrawable ORCA",
        label
    );
    assert!(
        acc.data.withdrawable_timestamp >= min_withdrawable_timestamp,
        "{}: pending timestamp >= now",
        label
    );
    assert_eq!(
        acc.data.withdraw_index, expected_withdraw_index,
        "{}: pending withdraw index",
        label
    );
}

pub fn assert_unstake_effects(
    ctx: &TestContext,
    state: Pubkey,
    vault: Pubkey,
    user_orca: Pubkey,
    user_xorca: Pubkey,
    xorca_mint: Pubkey,
    snap: &WithdrawSnapshot,
    pending_orca_amount: u64,
    xorca_unstake_amount: u64,
    label: &str,
) {
    // State escrow increased by pending amount
    let escrow_after = ctx
        .get_account::<State>(state)
        .unwrap()
        .data
        .escrowed_orca_amount;
    assert_eq!(
        escrow_after,
        snap.escrow_before.saturating_add(pending_orca_amount),
        "{}: state escrow after unstake",
        label
    );

    // Vault unchanged on unstake
    let vault_after = ctx.get_account::<TokenAccount>(vault).unwrap().data.amount;
    assert_eq!(
        vault_after, snap.vault_before,
        "{}: vault unchanged after unstake",
        label
    );

    // User ORCA unchanged on unstake
    let user_orca_after = ctx
        .get_account::<TokenAccount>(user_orca)
        .unwrap()
        .data
        .amount;
    assert_eq!(
        user_orca_after, snap.user_orca_before,
        "{}: user ORCA unchanged after unstake",
        label
    );

    // xORCA supply decreased by burn amount
    let xorca_supply_after = ctx
        .get_account::<TokenMint>(xorca_mint)
        .unwrap()
        .data
        .supply;
    assert_eq!(
        xorca_supply_after,
        snap.xorca_supply_before
            .saturating_sub(xorca_unstake_amount),
        "{}: xORCA supply after",
        label
    );

    // User xORCA decreased by burn amount
    let user_xorca_after = ctx
        .get_account::<TokenAccount>(user_xorca)
        .unwrap()
        .data
        .amount;
    assert_eq!(
        user_xorca_after,
        snap.user_xorca_before.saturating_sub(xorca_unstake_amount),
        "{}: user xORCA after",
        label
    );
}
