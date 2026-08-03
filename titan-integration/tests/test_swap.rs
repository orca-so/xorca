use solana_client::nonblocking::rpc_client::RpcClient;
use solana_pubkey::Pubkey;
use solana_sdk::{
    compute_budget::ComputeBudgetInstruction,
    instruction::Instruction,
    message::{VersionedMessage, v0::Message},
    signature::{Keypair, Signer},
    transaction::VersionedTransaction,
};
use std::error::Error;
use titan_integration_template::{
    account_caching::{AccountsCache, rpc_cache::RpcClientCache},
    trading_venue::{FromAccount, QuoteRequest, SwapType, TradingVenue},
};
use xorca_titan_integration::{
    amm::XOrcaVenue,
    constants::{ORCA_TOKEN_INFO, STATE_KEY, XORCA_TOKEN_INFO},
};

mod common;
use common::{rpc_client, venue_account};

#[ignore]
#[tokio::test]
async fn test() -> Result<(), Box<dyn Error>> {
    let rpc: RpcClient = rpc_client();
    let cache = RpcClientCache::new(rpc_client());
    let venue_account = venue_account(&cache).await;
    let mut trade_venue = XOrcaVenue::from_account(&STATE_KEY, &venue_account)?;
    let required_pubkeys = trade_venue.get_required_pubkeys_for_update()?;
    let _prefetched_accounts = cache.get_accounts(&required_pubkeys).await?;

    trade_venue.update_state(&cache).await?;
    let quote_req = QuoteRequest {
        input_mint: ORCA_TOKEN_INFO.pubkey,
        output_mint: XORCA_TOKEN_INFO.pubkey,
        amount: 100,
        swap_type: SwapType::ExactIn,
    };
    let quote = trade_venue.quote(quote_req.clone())?;

    println!("{quote:?}");

    let keypair = load_keypair()?;
    let ix = trade_venue.generate_swap_instruction(quote_req, keypair.pubkey())?;
    let message = build_message(&rpc, &keypair.pubkey(), ix).await?;
    let transaction = VersionedTransaction::try_new(message, &[keypair])?;
    let res = rpc.send_transaction(&transaction).await?;

    println!("{res}");

    Ok(())
}

fn load_keypair() -> Result<Keypair, Box<dyn Error>> {
    let signer_bytes_str = std::env::var("SIGNER_JSON_BYTES")
        .map_err(|_| "SIGNER_JSON_BYTES env var must be set to a JSON array of 64 bytes")?;
    let keypair_bytes: [u8; 64] = serde_json::from_str::<Vec<u8>>(&signer_bytes_str)?
        .try_into()
        .map_err(|_| "SIGNER_JSON_BYTES must decode to 64 bytes")?;
    Ok(Keypair::from_bytes(&keypair_bytes)?)
}

async fn build_message(
    rpc: &RpcClient,
    payer: &Pubkey,
    trade_ix: Instruction,
) -> Result<VersionedMessage, Box<dyn Error>> {
    let mut ixs = Vec::new();
    ixs.extend([
        ComputeBudgetInstruction::set_compute_unit_limit(1_400_000),
        ComputeBudgetInstruction::set_compute_unit_price(0),
    ]);
    ixs.push(trade_ix);
    let block_hash = rpc.get_latest_blockhash().await?;
    let msg = Message::try_compile(payer, &ixs, &[], block_hash)?;
    Ok(VersionedMessage::V0(msg))
}
