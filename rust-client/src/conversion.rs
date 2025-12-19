//! Off-chain conversion helpers that mirror the on-chain staking math. These
//! allow clients to predict xORCA/ORCA amounts using the exact same logic and
//! virtual offsets as the program.

use thiserror::Error;

// ----------------------------------
// CONVERSION FUNCTIONS
// ----------------------------------

/// Convert an ORCA amount to xORCA using the same virtual-offset defense as the
/// on-chain program. Returns the amount of xORCA to mint (integer division floors).
pub fn convert_orca_to_xorca(
    orca_amount_to_convert: u64,
    non_escrowed_orca_amount: u64,
    xorca_supply: u64,
) -> Result<u64, ConversionError> {
    if xorca_supply == 0 || non_escrowed_orca_amount == 0 {
        return Ok(orca_amount_to_convert);
    }

    let (non_escrowed_with_virtual_offset, xorca_supply_with_virtual_offset) =
        apply_virtual_offsets(non_escrowed_orca_amount, xorca_supply)?;

    let out_xorca = (orca_amount_to_convert as u128)
        .checked_mul(xorca_supply_with_virtual_offset)
        .ok_or(ConversionError::Arithmetic)?
        .checked_div(non_escrowed_with_virtual_offset)
        .ok_or(ConversionError::Arithmetic)?;

    out_xorca
        .try_into()
        .map_err(|_| ConversionError::Arithmetic)
}

/// Convert an xORCA amount to ORCA using the same virtual-offset defense as the
/// on-chain program. Returns the amount of ORCA to withdraw (integer division floors).
pub fn convert_xorca_to_orca(
    xorca_amount_to_convert: u64,
    non_escrowed_orca_amount: u64,
    xorca_supply: u64,
) -> Result<u64, ConversionError> {
    if xorca_supply == 0 || non_escrowed_orca_amount == 0 {
        return Err(ConversionError::Arithmetic);
    }

    let (non_escrowed_with_virtual_offset, xorca_supply_with_virtual_offset) =
        apply_virtual_offsets(non_escrowed_orca_amount, xorca_supply)?;

    let out_orca = (xorca_amount_to_convert as u128)
        .checked_mul(non_escrowed_with_virtual_offset)
        .ok_or(ConversionError::Arithmetic)?
        .checked_div(xorca_supply_with_virtual_offset)
        .ok_or(ConversionError::Arithmetic)?;

    out_orca.try_into().map_err(|_| ConversionError::Arithmetic)
}

// ----------------------------------
// ERROR
// ----------------------------------

#[derive(Debug, Error, PartialEq, Eq)]
/// Errors that occur during conversion (overflow or invalid inputs).
pub enum ConversionError {
    #[error("arithmetic overflow or invalid inputs")]
    Arithmetic,
}

// ----------------------------------
// VIRTUAL OFFSETS
// ----------------------------------

/// Virtual supply offset applied to xORCA to defend against vault inflation.
pub const VIRTUAL_XORCA_SUPPLY: u128 = 100;
/// Virtual ORCA offset applied to non-escrowed ORCA to defend against vault inflation.
pub const VIRTUAL_NON_ESCROWED_ORCA_AMOUNT: u128 = 100;

fn apply_virtual_offsets(
    non_escrowed_orca_amount: u64,
    xorca_supply: u64,
) -> Result<(u128, u128), ConversionError> {
    let xorca_supply_with_virtual_offset = (xorca_supply as u128)
        .checked_add(VIRTUAL_XORCA_SUPPLY)
        .ok_or(ConversionError::Arithmetic)?;
    let non_escrowed_with_virtual_offset = (non_escrowed_orca_amount as u128)
        .checked_add(VIRTUAL_NON_ESCROWED_ORCA_AMOUNT)
        .ok_or(ConversionError::Arithmetic)?;
    Ok((
        non_escrowed_with_virtual_offset,
        xorca_supply_with_virtual_offset,
    ))
}

// ----------------------------------
// TESTS
// ----------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orca_to_xorca_zero_supply() {
        assert_eq!(
            convert_orca_to_xorca(10, 0, 0).unwrap(),
            10,
            "zero supply or non-escrowed should convert 1-1"
        );
    }

    #[test]
    fn orca_to_xorca_nominal() {
        // Large values (9 decimal tokens): 100 ORCA, 400 non-escrowed, 200 xORCA supply
        let out = convert_orca_to_xorca(100_000_000_000, 400_000_000_000, 200_000_000_000).unwrap();
        // With virtual offsets: supply=200_000_000_100, non-escrowed=400_000_000_100 =>
        // 100_000_000_000 * 200_000_000_100 / 400_000_000_100 = 50_000_000_012 (floored)
        assert_eq!(out, 50_000_000_012);
    }

    #[test]
    fn xorca_to_orca_zero_supply_errs() {
        assert_eq!(
            convert_xorca_to_orca(10, 0, 0).unwrap_err(),
            ConversionError::Arithmetic
        );
    }

    #[test]
    fn xorca_to_orca_nominal() {
        // Large values (9 decimal tokens): xORCA 50, non-escrowed ORCA 500, xORCA supply 250
        let out = convert_xorca_to_orca(50_000_000_000, 500_000_000_000, 250_000_000_000).unwrap();
        // With virtual offsets: supply=250_000_000_100, non-escrowed=500_000_000_100 =>
        // 50_000_000_000 * 500_000_000_100 / 250_000_000_100 = 99_999_999_980 (floored)
        assert_eq!(out, 99_999_999_980);
    }

    #[test]
    fn detects_overflow_on_large_product() {
        // Force a multiply overflow in u128 (u64::MAX * u64::MAX > u128::MAX)
        let err = convert_orca_to_xorca(u64::MAX, u64::MAX, u64::MAX).unwrap_err();
        assert_eq!(err, ConversionError::Arithmetic);
    }
}
