use crate::{
    state_data, token_account_data, token_mint_data, TestContext, ATA_PROGRAM_ID, ORCA_ID,
    TOKEN_PROGRAM_ID, XORCA_ID, XORCA_PROGRAM_ID,
};
use solana_sdk::pubkey::Pubkey;
use xorca::find_state_address;
// (reserved) test math helpers could be added here if needed

/// Describe initial pool state in terms of supply, vault ORCA, escrowed ORCA, and cooldown.
pub struct PoolSetup {
    pub xorca_supply: u64,
    pub vault_orca: u64,
    pub escrowed_orca: u64,
    pub cool_down_period_s: i64,
}

impl Default for PoolSetup {
    fn default() -> Self {
        Self {
            xorca_supply: 0,
            vault_orca: 0,
            escrowed_orca: 0,
            cool_down_period_s: 7 * 24 * 60 * 60,
        }
    }
}

/// Per-user starting balances.
pub struct UserSetup {
    pub staker_orca: u64,
    pub staker_xorca: u64,
}

impl Default for UserSetup {
    fn default() -> Self {
        Self {
            staker_orca: 0,
            staker_xorca: 0,
        }
    }
}

/// Construct accounts and provide handy handles for tests.
pub struct Env {
    pub ctx: TestContext,
    pub state: Pubkey,
    pub vault: Pubkey,
    pub staker: Pubkey,
    pub staker_orca_ata: Pubkey,
    pub staker_xorca_ata: Pubkey,
}

impl Env {
    pub fn new(mut ctx: TestContext, pool: &PoolSetup, user: &UserSetup) -> Self {
        let state = find_state_address().unwrap().0;
        let staker = ctx.signer();
        let vault = Pubkey::find_program_address(
            &[
                &state.to_bytes(),
                &TOKEN_PROGRAM_ID.to_bytes(),
                &ORCA_ID.to_bytes(),
            ],
            &ATA_PROGRAM_ID,
        )
        .0;
        let staker_orca_ata = Pubkey::find_program_address(
            &[
                &staker.to_bytes(),
                &TOKEN_PROGRAM_ID.to_bytes(),
                &ORCA_ID.to_bytes(),
            ],
            &ATA_PROGRAM_ID,
        )
        .0;
        let staker_xorca_ata = Pubkey::find_program_address(
            &[
                &staker.to_bytes(),
                &TOKEN_PROGRAM_ID.to_bytes(),
                &XORCA_ID.to_bytes(),
            ],
            &ATA_PROGRAM_ID,
        )
        .0;

        // Write state
        ctx.write_account(
            state,
            XORCA_PROGRAM_ID,
            state_data!(
                escrowed_orca_amount => pool.escrowed_orca,
                update_authority => Pubkey::default(),
                cool_down_period_s => pool.cool_down_period_s,
            ),
        )
        .unwrap();

        // xORCA mint (9 decimals)
        ctx.write_account(
            XORCA_ID,
            TOKEN_PROGRAM_ID,
            token_mint_data!(
                supply => pool.xorca_supply,
                decimals => 9,
                mint_authority_flag => 1,
                mint_authority => state,
                is_initialized => true,
                freeze_authority_flag => 0,
                freeze_authority => Pubkey::default(),
            ),
        )
        .unwrap();

        // ORCA mint (large supply to avoid test overflow)
        ctx.write_account(
            ORCA_ID,
            TOKEN_PROGRAM_ID,
            token_mint_data!(
                supply => 1_000_000_000_000_000u64,
                decimals => 6,
                mint_authority_flag => 1,
                mint_authority => Pubkey::default(),
                is_initialized => true,
                freeze_authority_flag => 0,
                freeze_authority => Pubkey::default(),
            ),
        )
        .unwrap();

        // Use explicit vault amount defined by tests
        let vault_amount = pool.vault_orca as u128;

        // Vault token account
        ctx.write_account(
            vault,
            TOKEN_PROGRAM_ID,
            token_account_data!(mint => ORCA_ID, owner => state, amount => vault_amount as u64),
        )
        .unwrap();

        // User token accounts
        ctx.write_account(
            staker_orca_ata,
            TOKEN_PROGRAM_ID,
            token_account_data!(mint => ORCA_ID, owner => staker, amount => user.staker_orca),
        )
        .unwrap();
        ctx.write_account(
            staker_xorca_ata,
            TOKEN_PROGRAM_ID,
            token_account_data!(mint => XORCA_ID, owner => staker, amount => user.staker_xorca),
        )
        .unwrap();

        Self {
            ctx,
            state,
            vault,
            staker,
            staker_orca_ata,
            staker_xorca_ata,
        }
    }
}

// Note: Any exchange-rate based setup should be expressed via concrete `xorca_supply`,
// `vault_orca`, and `escrowed_orca` values.
