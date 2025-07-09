use crate::{
    assert_program_error, assert_program_success, token_mint_data, xorca_state_data, TestContext,
    ORCA_ID, SYSTEM_PROGRAM_ID, TOKEN_PROGRAM_ID, XORCA_ID,
};
use rstest::rstest;
use solana_sdk::pubkey::Pubkey;
use xorca::{
    find_xorca_state_address, AccountDiscriminator, Initialize, InitializeInstructionArgs,
    XorcaStakingProgramError, XorcaState,
};

#[rstest]
fn test_initialize(
    #[values(
        "Success",
        "XorcaStateExists",
        "InvalidLSTMintAuthority",
        "InvalidUpdateAuthority"
    )]
    case: &str,
) {
    let mut ctx = TestContext::new();
    let (xorca_state_account, _) = find_xorca_state_address(ORCA_ID).unwrap();
    let cool_down_period_s: u64 = 100;

    let lst_mint_authority = if case == "InvalidLSTMintAuthority" {
        Pubkey::default()
    } else {
        xorca_state_account
    };

    let update_authority = if case == "InvalidUpdateAuthority" {
        TOKEN_PROGRAM_ID
    } else {
        Pubkey::default()
    };

    if case == "XorcaStateExists" {
        ctx.write_account(xorca_state_account, xorca::ID, xorca_state_data!())
            .unwrap();
    }

    // Write data to accounts
    ctx.write_account(
        XORCA_ID,
        TOKEN_PROGRAM_ID,
        token_mint_data!(
            supply => 0,
            decimals => 9,
            mint_authority_flag => 1,
            mint_authority => lst_mint_authority,
        ),
    )
    .unwrap();
    ctx.write_account(ORCA_ID, TOKEN_PROGRAM_ID, token_mint_data!())
        .unwrap();

    // Define instruction
    let ix: solana_sdk::instruction::Instruction = Initialize {
        payer_account: ctx.signer(),
        xorca_state_account,
        xorca_mint_account: XORCA_ID,
        orca_mint_account: ORCA_ID,
        update_authority_account: update_authority,
        system_program_account: SYSTEM_PROGRAM_ID,
        token_program_account: TOKEN_PROGRAM_ID,
    }
    .instruction(InitializeInstructionArgs { cool_down_period_s });

    // Evaluate instruction
    let result = ctx.send(ix);
    match case {
        "Success" => {
            assert_program_success!(result);
            let xorca_state_account_after =
                ctx.get_account::<XorcaState>(xorca_state_account).unwrap();
            assert_eq!(
                xorca_state_account_after.data.discriminator,
                AccountDiscriminator::XorcaState
            );
            assert_eq!(xorca_state_account_after.data.xorca_mint, XORCA_ID);
            assert_eq!(
                xorca_state_account_after
                    .data
                    .cool_down_period_s
                    .to_le_bytes(),
                cool_down_period_s.to_be_bytes()
            );
            assert_eq!(
                xorca_state_account_after.data.update_authority,
                Pubkey::default()
            );
            assert_eq!(xorca_state_account_after.data.escrowed_orca_amount, 0);
        }
        "XorcaStateExists" => {
            assert_program_error!(result, XorcaStakingProgramError::IncorrectOwner);
        }
        "InvalidLSTMintAuthority" => {
            assert_program_error!(result, XorcaStakingProgramError::IncorrectAccountAddress);
        }
        "InvalidUpdateAuthority" => {
            assert_program_error!(result, XorcaStakingProgramError::IncorrectAccountAddress);
        }
        _ => panic!("Unknown case: {}", case),
    }
}
