use xorca_staking_program::util::math::{convert_orca_to_xorca, convert_xorca_to_orca};

// Test cases for convert_orca_to_xorca function
#[test]
fn test_convert_orca_to_xorca_zero_supply() {
    // When xorca_supply is 0, should return 1:1 exchange rate
    let result = convert_orca_to_xorca(1000, 5000, 0).unwrap();
    assert_eq!(result, 1000 * 1000); // 1000 ORCA * 1000 scaling factor = 1,000,000 xORCA
}

#[test]
fn test_convert_orca_to_xorca_zero_non_escrowed() {
    // When non_escrowed_orca_amount is 0, should return 1:1 exchange rate
    let result = convert_orca_to_xorca(1000, 0, 5000).unwrap();
    assert_eq!(result, 1000 * 1000); // 1000 ORCA * 1000 scaling factor = 1,000,000 xORCA
}

#[test]
fn test_convert_orca_to_xorca_normal_case() {
    // Normal case: 1000 ORCA with 5000 non-escrowed ORCA and 10000 xORCA supply
    // Expected: (1000 * 10000) / 5000 = 2000 xORCA
    let result = convert_orca_to_xorca(1000, 5000, 10000).unwrap();
    assert_eq!(result, 2000);
}

#[test]
fn test_convert_orca_to_xorca_large_numbers() {
    // Test with larger numbers to ensure no overflow
    let result = convert_orca_to_xorca(1_000_000, 5_000_000, 10_000_000).unwrap();
    assert_eq!(result, 2_000_000);
}

#[test]
fn test_convert_orca_to_xorca_precision() {
    // Test precision with decimal-like amounts
    // 100 ORCA with 1000 non-escrowed ORCA and 2000 xORCA supply
    // Expected: (100 * 2000) / 1000 = 200 xORCA
    let result = convert_orca_to_xorca(100, 1000, 2000).unwrap();
    assert_eq!(result, 200);
}

#[test]
fn test_convert_orca_to_xorca_overflow_protection() {
    // Test that large numbers don't cause overflow
    let large_orca = 1_000_000_000; // 1 billion ORCA
    let large_non_escrowed = 2_000_000_000; // 2 billion non-escrowed ORCA
    let large_supply = 5_000_000_000; // 5 billion xORCA supply
    let result = convert_orca_to_xorca(large_orca, large_non_escrowed, large_supply).unwrap();
    // Expected: (1_000_000_000 * 5_000_000_000) / 2_000_000_000 = 2_500_000_000
    assert_eq!(result, 2_500_000_000);
}

// Test cases for convert_xorca_to_orca function
#[test]
fn test_convert_xorca_to_orca_zero_supply() {
    // When xorca_supply is 0, should return error
    let result = convert_xorca_to_orca(1000, 5000, 0);
    assert!(result.is_err());
}

#[test]
fn test_convert_xorca_to_orca_zero_non_escrowed() {
    // When non_escrowed_orca_amount is 0, should return error
    let result = convert_xorca_to_orca(1000, 0, 5000);
    assert!(result.is_err());
}

#[test]
fn test_convert_xorca_to_orca_normal_case() {
    // Normal case: 2000 xORCA with 5000 non-escrowed ORCA and 10000 xORCA supply
    // Expected: (2000 * 5000) / 10000 = 1000 ORCA
    let result = convert_xorca_to_orca(2000, 5000, 10000).unwrap();
    assert_eq!(result, 1000);
}

#[test]
fn test_convert_xorca_to_orca_large_numbers() {
    // Test with larger numbers to ensure no overflow
    let result = convert_xorca_to_orca(2_000_000, 5_000_000, 10_000_000).unwrap();
    assert_eq!(result, 1_000_000);
}

#[test]
fn test_convert_xorca_to_orca_precision() {
    // Test precision with decimal-like amounts
    // 200 xORCA with 1000 non-escrowed ORCA and 2000 xORCA supply
    // Expected: (200 * 1000) / 2000 = 100 ORCA
    let result = convert_xorca_to_orca(200, 1000, 2000).unwrap();
    assert_eq!(result, 100);
}

#[test]
fn test_convert_xorca_to_orca_overflow_protection() {
    // Test that large numbers don't cause overflow
    let large_xorca = 2_500_000_000; // 2.5 billion xORCA
    let large_non_escrowed = 2_000_000_000; // 2 billion non-escrowed ORCA
    let large_supply = 5_000_000_000; // 5 billion xORCA supply
    let result = convert_xorca_to_orca(large_xorca, large_non_escrowed, large_supply).unwrap();
    // Expected: (2_500_000_000 * 2_000_000_000) / 5_000_000_000 = 1_000_000_000
    assert_eq!(result, 1_000_000_000);
}

// Test round-trip conversions
#[test]
fn test_round_trip_conversion() {
    let original_orca = 1000;
    let non_escrowed_orca = 5000;
    let xorca_supply = 10000;
    let xorca_amount =
        convert_orca_to_xorca(original_orca, non_escrowed_orca, xorca_supply).unwrap();
    let back_to_orca =
        convert_xorca_to_orca(xorca_amount, non_escrowed_orca, xorca_supply).unwrap();
    assert_eq!(back_to_orca, original_orca);
}

#[test]
fn test_round_trip_conversion_large_numbers() {
    let original_orca = 1_000_000_000;
    let non_escrowed_orca = 2_000_000_000;
    let xorca_supply = 5_000_000_000;
    let xorca_amount =
        convert_orca_to_xorca(original_orca, non_escrowed_orca, xorca_supply).unwrap();
    let back_to_orca =
        convert_xorca_to_orca(xorca_amount, non_escrowed_orca, xorca_supply).unwrap();
    assert_eq!(back_to_orca, original_orca);
}

// Test edge cases
#[test]
fn test_convert_orca_to_xorca_minimal_amounts() {
    // Test with minimal amounts
    let result = convert_orca_to_xorca(1, 1, 1).unwrap();
    assert_eq!(result, 1);
}

#[test]
fn test_convert_xorca_to_orca_minimal_amounts() {
    // Test with minimal amounts
    let result = convert_xorca_to_orca(1, 1, 1).unwrap();
    assert_eq!(result, 1);
}

#[test]
fn test_convert_orca_to_xorca_max_u64() {
    // Test with maximum u64 values to ensure no overflow since we use u128 for intermediate calculations
    let max_u64 = u64::MAX;
    let result = convert_orca_to_xorca(max_u64, max_u64, max_u64);
    assert!(
        result.is_ok(),
        "Expected Ok for max u64 input, but got error: {:?}",
        result.err()
    );
}

#[test]
fn test_convert_xorca_to_orca_max_u64() {
    // Test with maximum u64 values to ensure no overflow since we use u128 for intermediate calculations
    let max_u64 = u64::MAX;
    let result = convert_xorca_to_orca(max_u64, max_u64, max_u64);
    assert!(
        result.is_ok(),
        "Expected Ok for max u64 input, but got error: {:?}",
        result.err()
    );
}
