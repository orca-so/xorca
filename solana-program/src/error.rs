use thiserror_no_std::Error;

#[derive(Error, Clone, Debug, Eq, PartialEq)]
pub enum ErrorCode {
    #[error("Unknown instruction discriminator")]
    UnknownInstructionDiscriminator = 6000, // 0x1770

    #[error("Incorrect program id")]
    IncorrectProgramId = 6001, // 0x1771

    #[error("Invalid account role")]
    InvalidAccountRole = 6002, // 0x1772

    #[error("Not enough account keys")]
    NotEnoughAccountKeys = 6003, // 0x1773

    #[error("Incorrect owner")]
    IncorrectOwner = 6004, // 0x1774

    #[error("Invalid seeds")]
    InvalidSeeds = 6005, // 0x1775

    #[error("Invalid account address")]
    IncorrectAccountAddress = 6006, // 0x1776

    #[error("Invalid account data")]
    InvalidAccountData = 6007, // 0x1777

    #[error("Arithmetic error")]
    ArithmeticError = 6008, // 0x1778
}

impl From<ErrorCode> for pinocchio::program_error::ProgramError {
    fn from(e: ErrorCode) -> Self {
        pinocchio::program_error::ProgramError::Custom(e as u32)
    }
}
