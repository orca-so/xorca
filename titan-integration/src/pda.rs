use titan_integration_template::trading_venue::error::TradingVenueError;

// ----------------------------------
// STATE
// ----------------------------------

#[derive(Debug)]
pub struct State {
    pub escrowed_orca_amount: u64,
}

impl State {
    pub fn load(account_data: &[u8]) -> PdaDataResult<Self> {
        let mut offset = 8; // skip discriminator, padding1, bump and vault_bump
        let escrowed_orca_amount = read_u64(account_data, &mut offset)?;
        Ok(Self {
            escrowed_orca_amount,
        })
    }
}

// ----------------------------------
// VAULT
// ----------------------------------

#[derive(Debug)]
pub struct Vault {
    pub vault_orca_amount: u64,
}

impl Vault {
    pub fn load(account_data: &[u8]) -> PdaDataResult<Self> {
        let mut offset = 64; // skip token mint and owner
        let vault_orca_amount = read_u64(account_data, &mut offset)?;
        Ok(Self { vault_orca_amount })
    }
}

// ----------------------------------
// XORCA MINT
// ----------------------------------

#[derive(Debug)]
pub struct XOrcaMint {
    pub xorca_supply: u64,
}

impl XOrcaMint {
    pub fn load(account_data: &[u8]) -> PdaDataResult<Self> {
        let mut offset = 36; // skip mint authority
        let xorca_supply = read_u64(account_data, &mut offset)?;
        Ok(Self { xorca_supply })
    }
}

// ----------------------------------
// TYPES
// ----------------------------------

pub type PdaDataResult<T> = std::result::Result<T, TradingVenueError>;

// ----------------------------------
// UTILS
// ----------------------------------

const U64_SIZE: usize = 8;

fn read_u64(data: &[u8], offset: &mut usize) -> PdaDataResult<u64> {
    let end = *offset + U64_SIZE;
    let bytes = data
        .get(*offset..end)
        .ok_or(TradingVenueError::FromAccountError(
            "account data too short".into(),
        ))?;
    *offset = end;
    Ok(u64::from_le_bytes(
        bytes
            .try_into()
            .expect("slice length checked to be 8 bytes"),
    ))
}
