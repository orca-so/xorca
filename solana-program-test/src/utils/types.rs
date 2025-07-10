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
        data.decimals = 9;
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
