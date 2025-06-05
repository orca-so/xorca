#[macro_export]
macro_rules! assert_program_success {
    ($result:expr) => {
        if !$result.is_ok() {
            panic!("Expected a program success, got {:?}", $result.unwrap_err());
        }
    };
}

#[macro_export]
macro_rules! assert_program_error {
    ($result:expr, $error:expr) => {
        if $result.is_ok() {
            panic!("Expected a program error, got {:?}", $result.unwrap());
        }
        let err = $result.unwrap_err().err;
        if let solana_sdk::transaction::TransactionError::InstructionError(_, err) = err {
            assert_eq!(
                err,
                solana_sdk::instruction::InstructionError::from($error as u64)
            );
        } else {
            panic!("Expected a program error, got {:?}", err);
        }
    };
}
