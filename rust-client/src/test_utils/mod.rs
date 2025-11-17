#![cfg(all(test, feature = "fetch"))]

use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use serde::Deserialize;
use serde_json::{json, Value};
use solana_client::client_error::{ClientErrorKind, Result as RpcResult};
use solana_client::rpc_client::{RpcClient, RpcClientConfig};
use solana_client::rpc_request::RpcRequest;
use solana_client::rpc_sender::{RpcSender, RpcTransportStats};
use solana_program::program_option::COption;
use solana_program_pack::Pack;
use solana_pubkey::Pubkey;
use solana_sdk::commitment_config::CommitmentConfig;
use spl_token_interface::state::{Account as TokenAccount, AccountState, Mint};
use std::{collections::HashMap, path::Path, str::FromStr, u8};

use crate::{
    find_orca_vault_address, find_pending_withdraw_pda, find_state_address,
    utils::{ORCA_MINT_ADDRESS, TOKEN_PROGRAM_ADDRESS, XORCA_MINT_ADDRESS},
    PendingWithdraw, State, XORCA_STAKING_PROGRAM_ID,
};

#[derive(Debug, Deserialize)]
pub struct FixturesFile {
    pub staker: String,
    pub pending_indices_present: Vec<u8>,
    pub state: StateFixture,
    pub vault: VaultFixture,
    pub xorca_mint: MintFixture,
}

#[derive(Debug, Deserialize)]
pub struct StateFixture {
    pub cool_down_period_s: i64,
    pub escrowed_orca_amount: u64,
    pub update_authority: String,
}

#[derive(Debug, Deserialize)]
pub struct VaultFixture {
    pub owner: String,
    pub amount: u64,
}

#[derive(Debug, Deserialize)]
pub struct MintFixture {
    pub supply: u64,
    pub decimals: u8,
}

#[derive(Clone)]
pub struct AccountBytes {
    pub owner: Pubkey,
    pub lamports: u64,
    pub data: Vec<u8>,
}

#[derive(Clone)]
pub struct Fixtures {
    accounts: HashMap<Pubkey, AccountBytes>,
}

impl Fixtures {
    pub fn load_from_file(path: &Path) -> anyhow::Result<Self> {
        let json: FixturesFile = serde_json::from_slice(&std::fs::read(path)?)?;

        let staker = Pubkey::from_str(&json.staker)?;
        let update_authority = Pubkey::from_str(&json.state.update_authority)?;
        let (state_pda, _) = find_state_address()?;
        let token_program = Pubkey::from_str(TOKEN_PROGRAM_ADDRESS)?;
        let orca_mint = Pubkey::from_str(ORCA_MINT_ADDRESS)?;
        let (vault_pda, _) = find_orca_vault_address(&state_pda, &token_program, &orca_mint)?;
        let xorca_mint = Pubkey::from_str(XORCA_MINT_ADDRESS)?;

        let mut accounts: HashMap<Pubkey, AccountBytes> = HashMap::new();

        // State account
        let state_bytes = {
            let state = State {
                discriminator: crate::AccountDiscriminator::State,
                padding1: [0u8; 5],
                bump: 1,
                vault_bump: 1,
                escrowed_orca_amount: json.state.escrowed_orca_amount,
                cool_down_period_s: json.state.cool_down_period_s,
                update_authority,
                padding2: [0u8; 1992],
            };
            borsh::to_vec(&state)?
        };
        accounts.insert(
            state_pda,
            AccountBytes {
                owner: XORCA_STAKING_PROGRAM_ID,
                lamports: 1_000_000,
                data: state_bytes,
            },
        );

        // Vault token account
        let token_account_bytes = {
            let token_account = TokenAccount {
                mint: orca_mint,
                owner: Pubkey::from_str(&json.vault.owner)?,
                amount: json.vault.amount,
                delegate: COption::None,
                state: AccountState::Initialized,
                is_native: COption::None,
                delegated_amount: 0,
                close_authority: COption::None,
            };
            let mut buf = vec![0u8; TokenAccount::LEN];
            TokenAccount::pack(token_account, &mut buf)?;
            buf
        };
        accounts.insert(
            vault_pda,
            AccountBytes {
                owner: token_program,
                lamports: 1_000_000,
                data: token_account_bytes,
            },
        );

        // xORCA mint
        let mint_bytes = {
            let mint = Mint {
                mint_authority: COption::None,
                supply: json.xorca_mint.supply,
                decimals: json.xorca_mint.decimals,
                is_initialized: true,
                freeze_authority: COption::None,
            };
            let mut buf = vec![0u8; Mint::LEN];
            Mint::pack(mint, &mut buf)?;
            buf
        };
        accounts.insert(
            xorca_mint,
            AccountBytes {
                owner: token_program,
                lamports: 1_000_000,
                data: mint_bytes,
            },
        );

        // Pending withdraw PDAs
        for idx in 0u8..u8::MAX {
            if !json.pending_indices_present.contains(&idx) {
                continue;
            }
            let (addr, _) = find_pending_withdraw_pda(&staker, &idx)?;
            let pending = PendingWithdraw {
                discriminator: crate::AccountDiscriminator::PendingWithdraw,
                padding1: [0u8; 5],
                bump: 1,
                withdraw_index: idx,
                unstaker: staker,
                withdrawable_orca_amount: 1_000 + (idx as u64),
                withdrawable_timestamp: 123_456 + (idx as i64),
                padding2: [0u8; 968],
            };
            let bytes = borsh::to_vec(&pending)?;
            accounts.insert(
                addr,
                AccountBytes {
                    owner: XORCA_STAKING_PROGRAM_ID,
                    lamports: 1_000_000,
                    data: bytes,
                },
            );
        }

        Ok(Self { accounts })
    }

    fn ui_account_for(&self, address: &Pubkey) -> Value {
        if let Some(acc) = self.accounts.get(address) {
            let data_b64 = BASE64.encode(&acc.data);
            json!({
                "executable": false,
                "lamports": acc.lamports,
                "owner": acc.owner.to_string(),
                "rentEpoch": 0,
                "data": [data_b64, "base64"],
                "space": acc.data.len()
            })
        } else {
            Value::Null
        }
    }
}

pub struct MockSender {
    fixtures: Fixtures,
}

impl MockSender {
    pub fn new(fixtures: Fixtures) -> Self {
        Self { fixtures }
    }
}

#[async_trait]
impl RpcSender for MockSender {
    async fn send(&self, request: RpcRequest, params: Value) -> RpcResult<Value> {
        match request {
            RpcRequest::GetAccountInfo => {
                let address_str = params
                    .get(0)
                    .and_then(Value::as_str)
                    .ok_or_else(|| ClientErrorKind::Custom("missing address".into()))?;
                let address = Pubkey::from_str(address_str)
                    .map_err(|e| ClientErrorKind::Custom(e.to_string()))?;
                let value = self.fixtures.ui_account_for(&address);
                Ok(json!({ "context": { "slot": 123 }, "value": value }))
            }
            RpcRequest::GetMultipleAccounts => {
                let addrs = params
                    .get(0)
                    .and_then(Value::as_array)
                    .ok_or_else(|| ClientErrorKind::Custom("missing addresses".into()))?;
                let values: Vec<Value> = addrs
                    .iter()
                    .map(|v| {
                        v.as_str()
                            .and_then(|s| Pubkey::from_str(s).ok())
                            .map(|k| self.fixtures.ui_account_for(&k))
                            .unwrap_or(Value::Null)
                    })
                    .collect();
                Ok(json!({ "context": { "slot": 123 }, "value": values }))
            }
            _ => Err(ClientErrorKind::Custom(format!("Unmocked method: {:?}", request)).into()),
        }
    }

    fn get_transport_stats(&self) -> RpcTransportStats {
        RpcTransportStats::default()
    }

    fn url(&self) -> String {
        "mock://".to_string()
    }
}

pub fn make_mocked_client_from_fixtures(path: &Path) -> anyhow::Result<RpcClient> {
    let fixtures = Fixtures::load_from_file(path)?;
    let sender = MockSender::new(fixtures);
    let config = RpcClientConfig::with_commitment(CommitmentConfig::confirmed());
    Ok(RpcClient::new_sender(sender, config))
}
