use crate::error::ErrorCode;
use pinocchio::program_error::ProgramError;

pub fn convert_stake_token_to_lst(
    stake_token_amount_to_convert: u64,
    non_escrowed_stake_token_amount: u64,
    lst_supply: u64,
) -> Result<u64, ProgramError> {
    if lst_supply == 0 {
        return Err(ErrorCode::ArithmeticError)?;
    }
    let out_lst_amount = lst_supply
        .checked_mul(stake_token_amount_to_convert)
        .ok_or(ErrorCode::ArithmeticError)?
        .checked_div(non_escrowed_stake_token_amount)
        .ok_or(ErrorCode::ArithmeticError)?;
    Ok(out_lst_amount)
}

pub fn convert_lst_to_stake_token(
    lst_amount_to_convert: u64,
    non_escrowed_stake_token_amount: u64,
    lst_supply: u64,
) -> Result<u64, ProgramError> {
    if non_escrowed_stake_token_amount == 0 {
        return Err(ErrorCode::ArithmeticError)?;
    }
    let out_stake_token_amount = non_escrowed_stake_token_amount
        .checked_mul(lst_amount_to_convert)
        .ok_or(ErrorCode::ArithmeticError)?
        .checked_div(lst_supply)
        .ok_or(ErrorCode::ArithmeticError)?;
    Ok(out_stake_token_amount)
}
