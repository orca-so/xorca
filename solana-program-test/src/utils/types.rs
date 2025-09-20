#[macro_export]
macro_rules! zeroed_type {
    ($t:ty) => {{
        use borsh::BorshDeserialize;
        let bytes = [0u8; core::mem::size_of::<$t>()];
        let data = <$t>::deserialize(&mut bytes.as_ref()).unwrap();
        data
    }};
}

#[macro_export]
macro_rules! state_data {
    ($($name:ident => $value:expr),* $(,)?) => {{
        let mut data = crate::zeroed_type!(xorca::State);
        data.discriminator = xorca::AccountDiscriminator::State;
        $(
            data.$name = $value;
        )*
        data
    }};
}

#[macro_export]
macro_rules! token_mint_data {
    ($($name:ident => $value:expr),* $(,)?) => {{
        let mut data = crate::zeroed_type!(xorca::TokenMint);
        data.is_initialized = true;
        data.decimals = 6;
        $(
            data.$name = $value;
        )*
        data
    }};
}

#[macro_export]
macro_rules! token2022_mint_data {
    ($($name:ident => $value:expr),* $(,)?) => {{
        let mut data = crate::zeroed_type!(xorca::Token2022Mint);
        data.is_initialized = true;
        data.decimals = 6;
        $(
            data.$name = $value;
        )*
        data
    }};
}
#[macro_export]
macro_rules! token_account_data {
    ($($name:ident => $value:expr),* $(,)?) => {{
        let mut data = crate::zeroed_type!(xorca::TokenAccount);
        data.state = xorca::TokenAccountState::Initialized;
        $(
            data.$name = $value;
        )*
        data
    }};
}

#[macro_export]
macro_rules! token2022_account_data {
    ($($name:ident => $value:expr),* $(,)?) => {{
        let mut data = crate::zeroed_type!(xorca::Token2022Account);
        data.state = xorca::Token2022AccountState::Initialized;
        $(
            data.$name = $value;
        )*
        data
    }};
}

#[macro_export]
macro_rules! pending_withdraw_data {
    ($($name:ident => $value:expr),* $(,)?) => {{
        let mut data = crate::zeroed_type!(xorca::PendingWithdraw);
        data.discriminator = xorca::AccountDiscriminator::PendingWithdraw;
        $(
            data.$name = $value;
        )*
        data
    }};
}
