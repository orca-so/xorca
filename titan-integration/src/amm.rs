use async_trait::async_trait;
use solana_account::Account;
use solana_instruction::Instruction;
use solana_pubkey::Pubkey;
use spl_associated_token_account::get_associated_token_address;
use titan_integration_template::{
    account_caching::AccountsCache,
    trading_venue::{
        error::TradingVenueError, protocol::PoolProtocol, token_info::TokenInfo,
        AddressLookupTableTrait, FromAccount, QuoteRequest, QuoteResult, TradingVenue,
    },
};

use crate::{
    constants::{
        ORCA_TOKEN_INFO, STATE_KEY, TOKEN_PROGRAM_ID, VAULT_KEY, XORCA_STAKING_PROGRAM_ID,
        XORCA_TOKEN_INFO,
    },
    math::convert_orca_to_xorca,
    pda::{State, Vault, XOrcaMint},
    stake::{Stake, StakeInstructionArgs},
};
pub struct XOrcaVenue {
    state_key: Pubkey,
    vault_key: Pubkey,
    escrowed_orca_amount: u64,
    vault_orca_amount: Option<u64>,
    xorca_supply: Option<u64>,
    token_infos: Vec<TokenInfo>,
}

impl FromAccount for XOrcaVenue {
    fn from_account(pubkey: &Pubkey, account: &Account) -> Result<Self, TradingVenueError>
    where
        Self: Sized,
    {
        if !pubkey.eq(&STATE_KEY) {
            return Err(TradingVenueError::FromAccountError(pubkey.into()));
        }
        let state = State::load(&account.data)?;
        Ok(Self {
            state_key: *pubkey,
            vault_key: VAULT_KEY,
            escrowed_orca_amount: state.escrowed_orca_amount,
            vault_orca_amount: None,
            xorca_supply: None,
            token_infos: vec![ORCA_TOKEN_INFO, XORCA_TOKEN_INFO],
        })
    }
}

#[async_trait]
impl TradingVenue for XOrcaVenue {
    fn initialized(&self) -> bool {
        true
    }

    fn program_id(&self) -> Pubkey {
        XORCA_STAKING_PROGRAM_ID
    }

    fn program_dependencies(&self) -> Vec<Pubkey> {
        vec![XORCA_STAKING_PROGRAM_ID]
    }

    fn market_id(&self) -> Pubkey {
        XORCA_STAKING_PROGRAM_ID
    }

    fn get_token_info(&self) -> &[TokenInfo] {
        &self.token_infos
    }

    fn protocol(&self) -> PoolProtocol {
        PoolProtocol::XOrca
    }

    fn get_required_pubkeys_for_update(&self) -> Result<Vec<Pubkey>, TradingVenueError> {
        Ok(vec![
            self.state_key,
            self.vault_key,
            self.token_infos[1].pubkey,
        ])
    }

    async fn update_state(&mut self, cache: &dyn AccountsCache) -> Result<(), TradingVenueError> {
        let accounts: Vec<Option<Account>> = cache
            .get_accounts(&[self.state_key, self.vault_key, self.token_infos[1].pubkey])
            .await?;
        let [state_account, vault_account, xorca_mint_account]: [Option<Account>; 3] = accounts
            .try_into()
            .map_err(|_| TradingVenueError::FailedToFetchMultipleAccountData)?;
        let state_account = state_account
            .ok_or_else(|| TradingVenueError::NoAccountFound(self.state_key.into()))?;
        let vault_account = vault_account
            .ok_or_else(|| TradingVenueError::NoAccountFound(self.vault_key.into()))?;
        let xorca_mint_account = xorca_mint_account
            .ok_or_else(|| TradingVenueError::NoAccountFound(self.token_infos[1].pubkey.into()))?;
        let state = State::load(&state_account.data)?;
        let vault = Vault::load(&vault_account.data)?;
        let xorca_mint = XOrcaMint::load(&xorca_mint_account.data)?;
        self.escrowed_orca_amount = state.escrowed_orca_amount;
        self.vault_orca_amount = Some(vault.vault_orca_amount);
        self.xorca_supply = Some(xorca_mint.xorca_supply);
        Ok(())
    }

    fn quote(&self, request: QuoteRequest) -> Result<QuoteResult, TradingVenueError> {
        if !(request.input_mint.eq(&self.token_infos[0].pubkey)) {
            return Err(TradingVenueError::InvalidMint(request.input_mint.into()));
        }
        if !(request.output_mint.eq(&self.token_infos[1].pubkey)) {
            return Err(TradingVenueError::InvalidMint(request.output_mint.into()));
        }
        let (vault_orca_amount, xorca_supply) = match (self.vault_orca_amount, self.xorca_supply) {
            (Some(vault), Some(supply)) => (vault, supply),
            _ => {
                return Err(TradingVenueError::NotInitialized(
                    "State needs to be updated".into(),
                ));
            }
        };
        // If the request amount will cause the vault ORCA amount to overflow, set the amount
        // to the max value that will not overflow the finalized vault ORCA amount
        // and set the not_enough_liquidity flag to true.
        let mut amount = request.amount;
        let mut not_enough_liquidity = false;
        if request.amount > u64::MAX - vault_orca_amount {
            amount = u64::MAX - vault_orca_amount;
            not_enough_liquidity = true;
        }
        let non_escrowed_orca_amount = vault_orca_amount
            .checked_sub(self.escrowed_orca_amount)
            .ok_or(TradingVenueError::CheckedMathError(
                "Vault ORCA amount minus escrowed ORCA amount is less than 0".into(),
            ))?;
        let output_amount = convert_orca_to_xorca(amount, non_escrowed_orca_amount, xorca_supply)?;
        Ok(QuoteResult {
            input_mint: request.input_mint,
            output_mint: request.output_mint,
            amount,
            expected_output: output_amount,
            not_enough_liquidity,
        })
    }

    fn generate_swap_instruction(
        &self,
        request: QuoteRequest,
        user: Pubkey,
    ) -> Result<Instruction, TradingVenueError> {
        let user_input_ata = get_associated_token_address(&user, &request.input_mint);
        let user_output_ata = get_associated_token_address(&user, &request.output_mint);
        let stake = Stake {
            staker_account: user,
            vault_account: self.vault_key,
            staker_orca_ata: user_input_ata,
            staker_xorca_ata: user_output_ata,
            xorca_mint_account: self.token_infos[1].pubkey,
            state_account: self.state_key,
            orca_mint_account: self.token_infos[0].pubkey,
            token_program_account: TOKEN_PROGRAM_ID,
        };
        let ix = stake.instruction(StakeInstructionArgs {
            orca_stake_amount: request.amount,
        });
        Ok(ix)
    }
}

#[async_trait]
impl AddressLookupTableTrait for XOrcaVenue {
    async fn get_lookup_table_keys(
        &self,
        _accounts_cache: Option<&dyn AccountsCache>,
    ) -> Result<Vec<Pubkey>, TradingVenueError> {
        Ok(vec![
            self.vault_key,
            self.state_key,
            self.token_infos[0].pubkey,
            self.token_infos[1].pubkey,
            TOKEN_PROGRAM_ID,
            XORCA_STAKING_PROGRAM_ID,
        ])
    }
}
