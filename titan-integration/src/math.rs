use titan_integration_template::trading_venue::error::TradingVenueError;

/// Convert an ORCA amount to xORCA using the same virtual-offset defense as the
/// on-chain program. Returns the amount of xORCA to mint (integer division floors).
pub fn convert_orca_to_xorca(
    orca_amount_to_convert: u64,
    non_escrowed_orca_amount: u64,
    xorca_supply: u64,
) -> Result<u64, TradingVenueError> {
    if orca_amount_to_convert == 0 {
        return Ok(0);
    }
    if xorca_supply == 0 || non_escrowed_orca_amount == 0 {
        return Ok(orca_amount_to_convert);
    }

    let (non_escrowed_with_virtual_offset, xorca_supply_with_virtual_offset) =
        apply_virtual_offsets(non_escrowed_orca_amount, xorca_supply)?;

    let out_xorca = (orca_amount_to_convert as u128)
        .checked_mul(xorca_supply_with_virtual_offset)
        .ok_or(TradingVenueError::CheckedMathError(
            "xOrca Arithmetic Error".into(),
        ))?
        .checked_div(non_escrowed_with_virtual_offset)
        .ok_or(TradingVenueError::CheckedMathError(
            "xOrca Arithmetic Error".into(),
        ))?;

    out_xorca.try_into().map_err(|_| {
        TradingVenueError::DataConversionError("Could not convert xOrca amount to u64".into())
    })
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
) -> Result<(u128, u128), TradingVenueError> {
    let xorca_supply_with_virtual_offset = (xorca_supply as u128)
        .checked_add(VIRTUAL_XORCA_SUPPLY)
        .ok_or(TradingVenueError::CheckedMathError(
            "xOrca Virtual Offsets Arithmetic Error".into(),
        ))?;
    let non_escrowed_with_virtual_offset = (non_escrowed_orca_amount as u128)
        .checked_add(VIRTUAL_NON_ESCROWED_ORCA_AMOUNT)
        .ok_or(TradingVenueError::CheckedMathError(
            "xOrca Virtual Offsets Arithmetic Error".into(),
        ))?;
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
    fn orca_to_xorca_zero_input() {
        let out = convert_orca_to_xorca(0, 123_000_000_000, 443_000_000_000).unwrap();
        assert_eq!(out, 0);
    }
}
