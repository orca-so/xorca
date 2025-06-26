use pinocchio::{
    program_error::ProgramError,
    sysvars::{clock::Clock, Sysvar},
};

pub fn get_current_unix_timestamp() -> Result<i64, ProgramError> {
    let current_clock = Clock::get()?;
    Ok(current_clock.unix_timestamp)
}
