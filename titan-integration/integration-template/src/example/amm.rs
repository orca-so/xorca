use arrayref::array_ref;
use safe_transmute::{transmute_one_pedantic, transmute_to_bytes};
use solana_account::{Account, ReadableAccount};
use solana_instruction::Instruction;
use solana_program::program_pack::Pack;
use solana_pubkey::Pubkey;
use std::convert::TryFrom;

use crate::{
    example::raydium::{
        self,
        math::{CheckedCeilDiv, SwapDirection, U128},
        processor::{self, AUTHORITY_AMM},
        state::{Loadable, TEN_THOUSAND},
    },
    trading_venue::error::TradingVenueError,
};

#[derive(Clone, Copy, Debug)]
pub struct AmmKeys {
    pub amm_pool: Pubkey,
    pub amm_coin_mint: Pubkey,
    pub amm_pc_mint: Pubkey,
    pub amm_authority: Pubkey,
    pub amm_target: Pubkey,
    pub amm_coin_vault: Pubkey,
    pub amm_pc_vault: Pubkey,
    pub amm_lp_mint: Pubkey,
    pub nonce: u8,
}

const TEN_THOUSAND_U128: u128 = 10_000;

#[derive(Clone, Copy, Debug)]
pub struct CalculateResult {
    pub pool_pc_vault_amount: u64,
    pub pool_coin_vault_amount: u64,
    pub pool_lp_amount: u64,
    pub swap_fee_numerator: u64,
    pub swap_fee_denominator: u64,
}

pub fn load_amm_keys(
    amm_program: &Pubkey,
    amm_key: &Pubkey,
    amm_pool_account: &Account,
) -> Result<AmmKeys, TradingVenueError> {
    let account_data = amm_pool_account.data()[..].to_vec();
    let amm = raydium::state::AmmInfo::load_from_bytes(&account_data).map_err(|_| {
        TradingVenueError::DeserializationFailed("Failed to load AmmInfo from bytes".into())
    })?;
    let data = AmmKeys {
        amm_pool: *amm_key,
        amm_target: amm.target_orders,
        amm_coin_vault: amm.coin_vault,
        amm_pc_vault: amm.pc_vault,
        amm_lp_mint: amm.lp_mint,
        amm_coin_mint: amm.coin_vault_mint,
        amm_pc_mint: amm.pc_vault_mint,
        amm_authority: processor::authority_id(amm_program, AUTHORITY_AMM, amm.nonce as u8)
            .map_err(|_| {
                TradingVenueError::AmmMethodError("Failed to determine authority ID".into())
            })?,
        nonce: amm.nonce as u8,
    };
    Ok(data)
}

pub fn calculate_pool_vault_amounts_from_accounts(
    rsps: &[Option<Account>],
) -> Result<CalculateResult, TradingVenueError> {
    let accounts = array_ref![rsps, 0, 3];
    let [amm_account, amm_pc_vault_account, amm_coin_vault_account] = accounts;
    let amm: raydium::state::AmmInfo =
        transmute_one_pedantic::<raydium::state::AmmInfo>(transmute_to_bytes(
            &amm_account
                .as_ref()
                .ok_or_else(|| TradingVenueError::MissingState("AMM account missing".into()))?
                .clone()
                .data,
        ))
        .map_err(|_| {
            TradingVenueError::DeserializationFailed("Failed to deserialize AMM Info".into())
        })?;
    let amm_pc_vault = spl_token::state::Account::unpack(
        &amm_pc_vault_account
            .as_ref()
            .ok_or_else(|| TradingVenueError::MissingState("AMM PC vault account missing".into()))?
            .clone()
            .data,
    )
    .map_err(|_| {
        TradingVenueError::DeserializationFailed("Failed to deserialize token account".into())
    })?;
    let amm_coin_vault = spl_token::state::Account::unpack(
        &amm_coin_vault_account
            .as_ref()
            .ok_or_else(|| {
                TradingVenueError::MissingState("AMM Coin vault account missing".into())
            })?
            .clone()
            .data,
    )
    .map_err(|_| {
        TradingVenueError::DeserializationFailed("Failed to deserialize AMM coin vault".into())
    })?;
    let (amm_pool_pc_vault_amount, amm_pool_coin_vault_amount) =
        if raydium::state::AmmStatus::from_u64(amm.status).orderbook_permission() {
            return Err(TradingVenueError::AmmMethodError(
                "Pool has orderbook permission, not able to swap with v2 instruction".into(),
            ));
        } else {
            let (amm_pool_pc_vault_amount, amm_pool_coin_vault_amount) =
                raydium::math::Calculator::calc_total_without_take_pnl_no_orderbook(
                    amm_pc_vault.amount,
                    amm_coin_vault.amount,
                    &amm,
                )
                .map_err(|_| {
                    TradingVenueError::AmmMethodError(
                        "Failed to calculate total without take PNL with no orderbook".into(),
                    )
                })?;
            (amm_pool_pc_vault_amount, amm_pool_coin_vault_amount)
        };

    Ok(CalculateResult {
        pool_pc_vault_amount: amm_pool_pc_vault_amount,
        pool_coin_vault_amount: amm_pool_coin_vault_amount,
        pool_lp_amount: amm.lp_amount,
        swap_fee_numerator: amm.fees.swap_fee_numerator,
        swap_fee_denominator: amm.fees.swap_fee_denominator,
    })
}

fn max_amount_with_slippage(input_amount: u64, slippage_bps: u64) -> u64 {
    let input_expanded = u128::from(input_amount);
    let mult = u128::from(
        TEN_THOUSAND
            .checked_add(slippage_bps)
            .expect("Provided slippage bps + 10_000 overflows u64"),
    );
    // Should be impossible to multiply two u64 values and overflow a u128.
    let dividend = input_expanded.checked_mul(mult).unwrap();
    // Can use wrapping_div as we know the divisor isn't 0.
    let quotient = dividend.wrapping_div(TEN_THOUSAND_U128);
    // TODO: Return result in case of failure
    u64::try_from(quotient).expect("increasing input amount by slippage overflowed u64")
}

fn min_amount_with_slippage(input_amount: u64, slippage_bps: u64) -> u64 {
    let input_expanded = u128::from(input_amount);
    let mult = u128::from(TEN_THOUSAND.saturating_sub(slippage_bps));
    // Should be impossible to multiply two u64 values and overflow a u128.
    let dividend = input_expanded.checked_mul(mult).unwrap();
    // mult <= TEN_THOUSAND, so result can never be greater than input amount, which fit in a u64,
    // so should be safe to unwrap here.
    // Able to use wrapping_div as we know the divisor isn't 0.
    u64::try_from(dividend.wrapping_div(TEN_THOUSAND_U128)).unwrap()
}

pub fn swap_with_slippage(
    pc_amount: u64,
    coin_amount: u64,
    pc_vault_amount: u64,
    coin_vault_amount: u64,
    swap_fee_numerator: u64,
    swap_fee_denominator: u64,
    swap_direction: SwapDirection,
    amount_specified: u64,
    swap_base_in: bool,
    slippage_bps: u64,
) -> Result<u64, TradingVenueError> {
    if swap_direction == SwapDirection::PC2Coin
        && (pc_amount).checked_add(amount_specified).is_none()
    {
        return Err(TradingVenueError::MathError(
            "Amount exceeds possible threshold".into(),
        ));
    } else if swap_direction == SwapDirection::Coin2PC
        && (coin_amount).checked_add(amount_specified).is_none()
    {
        return Err(TradingVenueError::MathError(
            "Amount exceeds possible threshold".into(),
        ));
    }

    let other_amount_threshold = swap_exact_amount(
        pc_vault_amount,
        coin_vault_amount,
        swap_fee_numerator,
        swap_fee_denominator,
        swap_direction,
        amount_specified,
        swap_base_in,
    )?;
    let other_amount_threshold = if swap_base_in {
        // min out
        min_amount_with_slippage(other_amount_threshold, slippage_bps)
    } else {
        // max in
        max_amount_with_slippage(other_amount_threshold, slippage_bps)
    };

    if (swap_direction == SwapDirection::Coin2PC && other_amount_threshold >= pc_vault_amount)
        || (swap_direction == SwapDirection::PC2Coin && other_amount_threshold >= coin_vault_amount)
    {
        return Err(TradingVenueError::MathError(
            "Output amounds exceed tolerance".into(),
        ));
    }

    Ok(other_amount_threshold)
}

pub fn swap_exact_amount(
    pc_vault_amount: u64,
    coin_vault_amount: u64,
    swap_fee_numerator: u64,
    swap_fee_denominator: u64,
    swap_direction: raydium::math::SwapDirection,
    amount_specified: u64,
    swap_base_in: bool,
) -> Result<u64, TradingVenueError> {
    let other_amount_threshold = if swap_base_in {
        let swap_fee = U128::from(amount_specified)
            .checked_mul(swap_fee_numerator.into())
            .ok_or(TradingVenueError::MathError(
                "swap fee checked math error".into(),
            ))?
            .checked_ceil_div(swap_fee_denominator.into())
            .ok_or(TradingVenueError::MathError(
                "swap fee checked math error".into(),
            ))?
            .0;

        if swap_fee == U128::from(0) {
            return Err(TradingVenueError::MathError("Invalid fee amount".into()));
        }

        let swap_in_after_deduct_fee = U128::from(amount_specified).checked_sub(swap_fee).ok_or(
            TradingVenueError::MathError("swap_in_after_deduct_fee checked math error".into()),
        )?;
        raydium::math::Calculator::swap_token_amount_base_in(
            swap_in_after_deduct_fee,
            pc_vault_amount.into(),
            coin_vault_amount.into(),
            swap_direction,
        )?
        .as_u64()
    } else {
        let swap_in_before_add_fee = raydium::math::Calculator::swap_token_amount_base_out(
            amount_specified.into(),
            pc_vault_amount.into(),
            coin_vault_amount.into(),
            swap_direction,
        );
        let swap_in_after_add_fee = swap_in_before_add_fee
            .checked_mul(swap_fee_denominator.into())
            .ok_or(TradingVenueError::MathError(
                "swap_in_after_add_fee checked math error".into(),
            ))?
            .checked_ceil_div(
                (swap_fee_denominator.checked_sub(swap_fee_numerator).ok_or(
                    TradingVenueError::MathError("swap_in_after_add_fee checked math error".into()),
                )?)
                .into(),
            )
            .ok_or(TradingVenueError::MathError(
                "swap_in_after_add_fee checked math error".into(),
            ))?
            .0
            .as_u64();

        swap_in_after_add_fee
    };

    Ok(other_amount_threshold)
}

pub fn swap_v2(
    amm_program: &Pubkey,
    amm_keys: &AmmKeys,
    user_owner: &Pubkey,
    user_source: &Pubkey,
    user_destination: &Pubkey,
    amount_specified: u64,
    other_amount_threshold: u64,
    swap_base_in: bool,
) -> Result<Instruction, TradingVenueError> {
    let swap_instruction = if swap_base_in {
        raydium::instruction::swap_base_in_v2(
            amm_program,
            &amm_keys.amm_pool,
            &amm_keys.amm_authority,
            &amm_keys.amm_coin_vault,
            &amm_keys.amm_pc_vault,
            user_source,
            user_destination,
            user_owner,
            amount_specified,
            other_amount_threshold,
        )
        .map_err(|_| TradingVenueError::AmmMethodError("Failed to perform v2 swap".into()))?
    } else {
        unimplemented!()
    };

    Ok(swap_instruction)
}
