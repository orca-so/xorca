use litesvm::LiteSVM;
use rand::Rng;
use solana_account::Account;
use solana_account::WritableAccount;
use solana_compute_budget::compute_budget::ComputeBudget;
use solana_program_pack::Pack;
use solana_pubkey::Pubkey;
use solana_sdk::native_token::LAMPORTS_PER_SOL;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;
use solana_sysvar::clock::{self, Clock};
use spl_associated_token_account::get_associated_token_address_with_program_id;
use spl_token::state::{Account as TokenAccount, AccountState};
use std::path::PathBuf;
use titan_integration_template::account_caching::AccountsCache;
use titan_integration_template::trading_venue::error::TradingVenueError;
use titan_integration_template::trading_venue::QuoteRequest;
use titan_integration_template::trading_venue::TradingVenue;
use xorca_titan_integration::constants::XORCA_STAKING_PROGRAM_ID;

pub struct SimulationContext {
    pub litesvm: LiteSVM,
    pub keypair: Keypair,
}

/// Creates a new LiteSVM instance configured with a funded signer.
pub fn setup_litesvm() -> SimulationContext {
    let mut litesvm = LiteSVM::new().with_compute_budget(ComputeBudget {
        compute_unit_limit: 1_400_000,
        ..Default::default()
    });

    // Load xORCA Staking Program binary
    let program_path = xorca_program_path();
    litesvm
        .add_program_from_file(XORCA_STAKING_PROGRAM_ID, program_path)
        .unwrap();

    // Create a funded user wallet.
    let keypair = Keypair::new();
    let account = Account {
        lamports: 10_000 * LAMPORTS_PER_SOL,
        data: vec![],
        owner: solana_sdk::system_program::id(),
        executable: false,
        rent_epoch: 0,
    };
    litesvm
        .set_account(keypair.pubkey(), account.into())
        .unwrap();

    SimulationContext { litesvm, keypair }
}

fn xorca_program_path() -> String {
    if let Ok(p) = std::env::var("XORCA_PROGRAM_SO") {
        return p;
    }
    let path: PathBuf =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../target/deploy/xorca_staking_program.so");
    path.to_string_lossy().into_owned()
}

/// Simulate a swap using LiteSVM and return the output amount of token B.
/// This should give the true on-chain output for that swap.
pub async fn sim_quote_request(
    venue: &dyn TradingVenue,
    cache: &dyn AccountsCache,
    request: QuoteRequest,
    litesvm: &mut LiteSVM,
    keypair: &Keypair,
) -> u64 {
    let tradable_mints = venue.get_token_info();

    // Identify which token is A and which is B (depending on swap direction)
    let idx_0 = tradable_mints
        .iter()
        .position(|x| x.pubkey == request.input_mint)
        .unwrap();
    let idx_1 = (idx_0 + 1) % 2;

    let (token_a, token_a_program) = (
        tradable_mints[idx_0].pubkey,
        tradable_mints[idx_0].get_token_program(),
    );
    let (token_b, token_b_program) = (
        tradable_mints[idx_1].pubkey,
        tradable_mints[idx_1].get_token_program(),
    );

    let token_account_a =
        get_associated_token_address_with_program_id(&keypair.pubkey(), &token_a, &token_a_program);
    let token_account_b =
        get_associated_token_address_with_program_id(&keypair.pubkey(), &token_b, &token_b_program);

    //
    // Create synthetic token accounts inside the simulator
    //
    let account_a = build_token_account(
        token_a,
        keypair.pubkey(),
        u64::MAX, // ensure "infinite" input
    );
    let account_b = build_token_account(token_b, keypair.pubkey(), 0);

    // Load accounts into LiteSVM
    litesvm.set_account(token_account_a, account_a).unwrap();
    litesvm.set_account(token_account_b, account_b).unwrap();

    //
    // Build the swap instruction
    //
    let ix = venue
        .generate_swap_instruction(request, keypair.pubkey())
        .unwrap();

    // Load all instruction accounts into SVM (except executable ones already present)
    let pks: Vec<Pubkey> = ix.accounts.iter().map(|acc| acc.pubkey).collect();
    let accounts_to_load = cache
        .get_accounts(&pks)
        .await
        .map_err(|e| format!("failed to load accounts for sim: {e}"))
        .unwrap();
    for (account, key) in accounts_to_load.iter().zip(pks) {
        if let Some(acc) = account {
            if acc.executable {
                continue;
            }
            litesvm.set_account(key, acc.clone()).unwrap();
        }
    }

    //
    // Execute swap inside the SIM
    //
    let blockhash = litesvm.latest_blockhash();
    let tx =
        Transaction::new_signed_with_payer(&[ix], Some(&keypair.pubkey()), &[keypair], blockhash);

    litesvm.send_transaction(tx).unwrap();

    //
    // Read output account and extract the final token amount
    //
    let account_b = litesvm.get_account(&token_account_b).unwrap();
    let post_b =
        TokenAccount::unpack_from_slice(&account_b.data).expect("Failed to unpack token B account");
    post_b.amount
}

pub async fn sync_litesvm_clock(litesvm: &mut LiteSVM, cache: &dyn AccountsCache) {
    let latest_clock = cache.get_account(&clock::ID).await.unwrap();
    let latest_clock: Clock = latest_clock
        .as_ref()
        .ok_or(TradingVenueError::NoAccountFound(clock::ID.into()))
        .unwrap()
        .deserialize_data()
        .unwrap();
    litesvm.set_sysvar::<Clock>(&latest_clock);
}

fn build_token_account(mint: Pubkey, owner: Pubkey, amount: u64) -> Account {
    let mut account = Account::new(LAMPORTS_PER_SOL, TokenAccount::LEN, &spl_token::ID);
    let mut account_data = TokenAccount::default();
    account_data.mint = mint;
    account_data.owner = owner;
    account_data.state = AccountState::Initialized;
    account_data.amount = amount;
    account_data.pack_into_slice(account.data_as_mut_slice());
    account
}

/// Returns a log-uniformly sampled u64 in `[lo, hi]`.
pub fn sample_log_uniform_u64(lo: u64, hi: u64) -> u64 {
    assert!(lo >= 1, "log-uniform sampling requires lo >= 1");
    assert!(lo <= hi);
    let lo_f = lo as f64;
    let hi_f = hi as f64;
    let log_lo = lo_f.ln();
    let log_hi = hi_f.ln();
    let r: f64 = rand::rng().random();
    let log_val = log_lo + r * (log_hi - log_lo);
    (log_val.exp() as u64).clamp(lo, hi)
}
