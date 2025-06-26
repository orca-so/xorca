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
macro_rules! xorca_state_data {
    ($($name:ident => $value:expr),* $(,)?) => {{
        let mut data = crate::zeroed_type!(xorca::XorcaState);
        data.discriminator = xorca::AccountDiscriminator::XorcaState;
        $(
            data.$name = $value;
        )*
        data
    }};
}
