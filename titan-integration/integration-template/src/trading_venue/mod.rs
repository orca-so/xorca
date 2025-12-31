//! Core traits and data structures used by Titan-compatible trading venues.
//!
//! A "trading venue" is any automated market maker (AMM), orderbook, or
//! proprietary liquidity engine that wishes to integrate with Titan’s quoting
//! and routing framework.
//!
//! Implementers are responsible for correctly handling state updates,
//! account deserialization, quoting semantics, and swap instruction
//! generation. Titan aggregates venues based on these traits after some
//! proprietary modifications to the logic provided by integrating partners.

pub mod bounds;
pub mod error;
pub mod protocol;
pub mod token_info;

use assert_no_alloc::assert_no_alloc;
use async_trait::async_trait;
use solana_account::Account;
use solana_instruction::Instruction;
use solana_pubkey::Pubkey;

use crate::{
    account_caching::AccountsCache,
    trading_venue::{
        bounds::find_boundaries, error::TradingVenueError, protocol::PoolProtocol,
        token_info::TokenInfo,
    },
};

/// Describes which type of swap the user is performing.
///
/// * `ExactIn`  — The user specifies exactly how many input atoms they want
///                to spend, and the venue returns a quote for the resulting
///                output amount.
/// * `ExactOut` — The user specifies exactly how many output atoms they want
///                to receive, and the venue determines how many input atoms
///                are required.
///
/// **Warning:** Titan currently only supports `ExactIn`. Implementers *must*
/// support `ExactIn`, and may optionally support `ExactOut` for future use.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum SwapType {
    ExactIn,
    ExactOut,
}

/// Request structure passed to venue `quote()` and `generate_swap_instruction()`.
///
/// All amounts are denominated in integer atom units (not scaled to UI decimals).
#[derive(Debug, Clone)]
pub struct QuoteRequest {
    /// Mint of the token the user is providing.
    pub input_mint: Pubkey,

    /// Mint of the token the user expects to receive.
    pub output_mint: Pubkey,

    /// Amount of *input* or *output* atoms, depending on `swap_type`.
    pub amount: u64,

    /// Swap mode: `ExactIn` or `ExactOut`.
    ///
    /// Titan currently calls venues only with `ExactIn`, but venues should
    /// not panic if `ExactOut` is provided.
    pub swap_type: SwapType,
}

/// A result returned from a venue’s `quote()` implementation.
///
/// This describes how much of the input would be consumed and how much
/// output would be produced, based on current pool state.
#[derive(Debug, Clone)]
pub struct QuoteResult {
    /// Mint of the token the user provided.
    pub input_mint: Pubkey,

    /// Mint of the token the user receives.
    pub output_mint: Pubkey,

    /// Actual amount of input atoms that would be consumed.
    pub amount: u64,

    /// Expected number of output atoms produced by the venue.
    pub expected_output: u64,

    /// Indicates whether the pool has insufficient liquidity to consume the full input.
    ///
    /// For example, if a pool only has enough liquidity for half of the provided
    /// input, this flag should be set to `true` and `amount = request.amount / 2`.
    pub not_enough_liquidity: bool,
}

/// A convenience trait for converting on-chain accounts into structured pool/venue state.
///
/// Implementers are responsible for performing any deserialization necessary
/// to reconstruct on-chain pool state for their venue.
pub trait FromAccount {
    /// Parse an on-chain Solana account into the venue’s internal state structure.
    ///
    /// `pubkey` is the address of the account; `account` is its data.
    fn from_account(pubkey: &Pubkey, account: &Account) -> Result<Self, TradingVenueError>
    where
        Self: Sized;
}

/// Trait allowing a venue to declare which address-lookup table (ALT) keys
/// it requires for transaction construction.
///
/// Implementers should return all additional keys (besides swap accounts)
/// that must be included in the ALT in order to successfully compress swaps.
#[async_trait]
pub trait AddressLookupTableTrait {
    /// Return a list of pubkeys that should be inserted into an address lookup table.
    async fn get_lookup_table_keys(
        &self,
        accounts_cache: Option<&dyn AccountsCache>,
    ) -> Result<Vec<Pubkey>, TradingVenueError>;
}

/// Primary trait describing an AMM or trading venue integrated with Titan.
///
/// Any AMM, orderbook, or custom liquidity engine must implement this trait
/// to be usable by Titan’s routing system.
#[async_trait]
pub trait TradingVenue {
    /// Whether the venue is fully initialized.
    ///
    /// This allows Titan to skip venues that failed initialization or
    /// are missing required on-chain accounts.
    fn initialized(&self) -> bool;

    /// The main program ID for the venue.
    fn program_id(&self) -> Pubkey;

    /// All additional program IDs this venue depends on (e.g. SPL Token program).
    fn program_dependencies(&self) -> Vec<Pubkey>;

    /// Unique identifier for the market/pool instance.
    fn market_id(&self) -> Pubkey;

    /// Return the mint pubkeys for all tokens traded in this venue.
    ///
    /// The default implementation pulls these from the venue's `TokenInfo`.
    fn tradable_mints(&self) -> Result<Vec<Pubkey>, TradingVenueError> {
        Ok(self.get_token_info().iter().map(|x| x.pubkey).collect())
    }

    /// Return the decimals for each tradable token.
    fn decimals(&self) -> Result<Vec<i32>, TradingVenueError> {
        Ok(self.get_token_info().iter().map(|x| x.decimals).collect())
    }

    /// Return fixed token metadata for this venue (mint + decimals).
    fn get_token_info(&self) -> &[TokenInfo];

    /// Fetch a single token by index.
    ///
    /// Returns an error if the index is out of bounds.
    fn get_token(&self, i: usize) -> Result<&TokenInfo, TradingVenueError> {
        self.get_token_info()
            .get(i)
            .ok_or(TradingVenueError::TokenInfoIndexError(i))
    }

    /// Identify which protocol type this venue is (e.g. Raydium, Orca, Phoenix).
    fn protocol(&self) -> PoolProtocol;

    /// A human-readable label describing the venue’s protocol.
    fn label(&self) -> String {
        self.protocol().into()
    }

    /// Returns the minimal set of pubkeys required to update venue state.
    ///
    /// Titan will prefetch these accounts before calling `update_state()`.
    fn get_required_pubkeys_for_update(&self) -> Result<Vec<Pubkey>, TradingVenueError>;

    /// Update the venue's internal state from the provided account cache.
    ///
    /// This is where implementers deserialize pool accounts, tick arrays,
    /// orderbooks, or other relevant on-chain state.
    async fn update_state(&mut self, cache: &dyn AccountsCache) -> Result<(), TradingVenueError>;

    /// Compute a quote for the given swap parameters.
    ///
    /// **Implementer requirement:** the venue **must** handle zero input amounts
    /// without panicking or returning an error. Titan sometimes requests zero-input
    /// quotes.
    fn quote(&self, request: QuoteRequest) -> Result<QuoteResult, TradingVenueError>;

    /// Construct the transaction instruction needed to execute a swap.
    ///
    /// This should use the amounts from the original `QuoteRequest`,
    /// not the `QuoteResult`. Venues should not modify swap semantics here;
    /// only build the appropriate on-chain instruction.
    fn generate_swap_instruction(
        &self,
        request: QuoteRequest,
        user: Pubkey,
    ) -> Result<Instruction, TradingVenueError>;

    /// Compute lower/upper admissible boundaries for valid input amounts
    /// using binary search over the venue's `quote()` function.
    ///
    /// This is used by Titan when determining safe routing ranges or when
    /// generating fallback limits.
    ///
    /// `tkn_in_ind` and `tkn_out_ind` refer to token indices in `get_token_info()`.
    fn bounds(&self, tkn_in_ind: u8, tkn_out_ind: u8) -> Result<(u64, u64), TradingVenueError> {
        let input_mint = self.get_token(tkn_in_ind as usize)?.pubkey;
        let output_mint = self.get_token(tkn_out_ind as usize)?.pubkey;

        // Closure for boundary-finding—performs `ExactIn` quotes at various x.
        let f = |x: u64| {
            self.quote(QuoteRequest {
                amount: x,
                swap_type: SwapType::ExactIn,
                input_mint,
                output_mint,
            })
        };

        find_boundaries(&f)
    }
}
