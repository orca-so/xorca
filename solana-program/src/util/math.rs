use crate::error::ErrorCode;
use pinocchio::program_error::ProgramError;

pub fn convert_orca_to_xorca(
    orca_amount_to_convert: u64,
    non_escrowed_orca_amount: u64,
    xorca_supply: u64,
) -> Result<u64, ProgramError> {
    if (xorca_supply == 0) || (non_escrowed_orca_amount == 0) {
        return Ok(orca_amount_to_convert); // Exchange rate is 1:1
    }

    // Perform calculations using u128 to prevent overflow for intermediate products.
    // Convert all relevant u64 inputs to u128 for the calculation.
    let xorca_supply_u128 = xorca_supply as u128;
    let orca_amount_to_convert_u128 = orca_amount_to_convert as u128;
    let non_escrowed_orca_amount_u128 = non_escrowed_orca_amount as u128;

    let out_xorca_amount_u128 = orca_amount_to_convert_u128
        .checked_mul(xorca_supply_u128)
        .ok_or(ErrorCode::ArithmeticError)?
        .checked_div(non_escrowed_orca_amount_u128)
        .ok_or(ErrorCode::ArithmeticError)?;

    // Cast the final u128 result back to u64.
    let out_xorca_amount: u64 = out_xorca_amount_u128
        .try_into()
        .map_err(|_| ErrorCode::ArithmeticError)?; // Return an error if the value is too large for u64

    Ok(out_xorca_amount)
}

pub fn convert_xorca_to_orca(
    xorca_amount_to_convert: u64,
    non_escrowed_orca_amount: u64,
    xorca_supply: u64,
) -> Result<u64, ProgramError> {
    if (xorca_supply == 0) || (non_escrowed_orca_amount == 0) {
        // Should be unreachable
        return Err(ErrorCode::ArithmeticError.into());
    }

    // Perform calculations using u128 to prevent overflow for intermediate products.
    // Convert all relevant u64 inputs to u128 for the calculation.
    let non_escrowed_orca_amount_u128 = non_escrowed_orca_amount as u128;
    let xorca_amount_to_convert_u128 = xorca_amount_to_convert as u128;
    let xorca_supply_u128 = xorca_supply as u128;

    let out_orca_amount_u128 = xorca_amount_to_convert_u128
        .checked_mul(non_escrowed_orca_amount_u128)
        .ok_or(ErrorCode::ArithmeticError)?
        .checked_div(xorca_supply_u128)
        .ok_or(ErrorCode::ArithmeticError)?;

    // Cast the final u128 result back to u64.
    let out_orca_amount: u64 = out_orca_amount_u128
        .try_into()
        .map_err(|_| ErrorCode::ArithmeticError)?; // Return an error if the value is too large for u64

    Ok(out_orca_amount)
}
