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
macro_rules! staking_pool_data {
    ($($name:ident => $value:expr),* $(,)?) => {{
        let mut data = crate::zeroed_type!(xorca::StakingPool);
        data.discriminator = xorca::AccountDiscriminator::StakingPool;
        $(
            data.$name = $value;
        )*
        data
    }};
}
