use crate::{
    assert_program_success, state_data, token_account_data, token_mint_data, TestContext,
    ATA_PROGRAM_ID, ORCA_ID, TOKEN_PROGRAM_ID, XORCA_ID, XORCA_PROGRAM_ID,
};
use solana_sdk::pubkey::Pubkey;
use xorca::{find_state_address, Stake, StakeInstructionArgs, TokenAccount, DEFAULT_ACCOUNT_LEN};

/// Test 1a: Stake token for xOrca with valid parameters
/// - stake token is transferred from staker to vault
/// - xORCA is minted to the staker
/// - exchange rate is updated
/// - no escrowed ORCA
/// - 1:1 exchange rate
#[test]
fn test_stake_success() {
    let mut ctx = TestContext::new();
    let (state_account, _) = find_state_address().unwrap();

    // Write state account
    ctx.write_account(
        state_account,
        XORCA_PROGRAM_ID,
        state_data!(
            escrowed_orca_amount => 0,
            update_authority => Pubkey::default(),
            cool_down_period_s => 7 * 24 * 60 * 60,
        ),
    )
    .unwrap();
    ctx.pad_account(state_account, DEFAULT_ACCOUNT_LEN).unwrap();

    // Write xOrca mint account with valid data
    ctx.write_account(
        XORCA_ID,
        TOKEN_PROGRAM_ID,
        token_mint_data!(
            supply => 0,
            decimals => 9,
            mint_authority_flag => 1,
            mint_authority => XORCA_PROGRAM_ID,
            is_initialized => true,
            freeze_authority_flag => 0,
            freeze_authority => Pubkey::default(),
        ),
    )
    .unwrap();

    // Write Orca mint account
    ctx.write_account(
        ORCA_ID,
        TOKEN_PROGRAM_ID,
        token_mint_data!(
            supply => 1000000000,
            decimals => 6,
            mint_authority_flag => 1,
            mint_authority => Pubkey::default(),
            is_initialized => true,
            freeze_authority_flag => 0,
            freeze_authority => Pubkey::default(),
        ),
    )
    .unwrap();

    // Write Vault Orca ata
    let vault_account = Pubkey::find_program_address(
        &[
            &state_account.to_bytes(),
            &TOKEN_PROGRAM_ID.to_bytes(),
            &ORCA_ID.to_bytes(),
        ],
        &ATA_PROGRAM_ID,
    )
    .0;
    ctx.write_account(
        vault_account,
        TOKEN_PROGRAM_ID,
        token_account_data!(
            mint => ORCA_ID,
            owner => state_account,
            amount => 0, // Vault has no ORCA
        ),
    )
    .unwrap();

    // Write staker Orca ata
    let staker_orca_ata = Pubkey::find_program_address(
        &[
            &ctx.signer().to_bytes(),
            &TOKEN_PROGRAM_ID.to_bytes(),
            &ORCA_ID.to_bytes(),
        ],
        &ATA_PROGRAM_ID,
    )
    .0;
    ctx.write_account(
        staker_orca_ata,
        TOKEN_PROGRAM_ID,
        token_account_data!(
            mint => ORCA_ID,
            owner => ctx.signer(),
            amount => 1_000_000, // owns 1 ORCA
        ),
    )
    .unwrap();

    // Write staker xOrca ata
    let staker_xorca_ata = Pubkey::find_program_address(
        &[
            &ctx.signer().to_bytes(),
            &TOKEN_PROGRAM_ID.to_bytes(),
            &XORCA_ID.to_bytes(),
        ],
        &ATA_PROGRAM_ID,
    )
    .0;
    ctx.write_account(
        staker_xorca_ata,
        TOKEN_PROGRAM_ID,
        token_account_data!(
            mint => XORCA_ID,
            owner => ctx.signer(),
            amount => 0, // staker has no xORCA
        ),
    )
    .unwrap();

    // Define instruction
    let ix: solana_sdk::instruction::Instruction = Stake {
        staker_account: ctx.signer(),
        state_account,
        vault_account,
        staker_orca_ata,
        staker_xorca_ata,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
    }
    .instruction(StakeInstructionArgs {
        stake_amount: 1_000_000,
    });

    // Execute instruction
    let result = ctx.send(ix);
    assert_program_success!(result);

    // Check accounts
    let vault_account_after = ctx.get_account::<TokenAccount>(vault_account).unwrap();
    println!(
        "Decoded Vault Account from client: {:?}",
        vault_account_after.data
    );

    let staker_orca_ata_after = ctx.get_account::<TokenAccount>(staker_orca_ata).unwrap();
    println!(
        "Decoded Staker Orca ATA from client: {:?}",
        staker_orca_ata_after.data
    );

    let staker_xorca_ata_after = ctx.get_account::<TokenAccount>(staker_xorca_ata).unwrap();
    println!(
        "Decoded Staker xOrca ATA from client: {:?}",
        staker_xorca_ata_after.data
    );

    assert_eq!(
        vault_account_after.data.amount, 1_000_000,
        "Vault account should have 1 ORCA"
    );
    assert_eq!(
        staker_orca_ata_after.data.amount, 0,
        "Staker Orca ATA should have 0 ORCA"
    );
    assert_eq!(
        staker_xorca_ata_after.data.amount, 1_000_000_000,
        "Staker xOrca ATA should have 1 xORCA"
    );
}
