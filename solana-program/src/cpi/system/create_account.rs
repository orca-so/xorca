use pinocchio::{
    account_info::AccountInfo,
    instruction::{AccountMeta, Instruction, Signer},
    program::invoke_signed,
    pubkey::Pubkey,
    ProgramResult,
};

const DISCRIMINATOR: [u8; 4] = [0, 0, 0, 0];

pub struct CreateAccount<'a> {
    pub program: &'a AccountInfo,
    pub from: &'a AccountInfo,
    pub to: &'a AccountInfo,
    pub lamports: u64,
    pub space: u64,
    pub owner: &'a Pubkey,
}

impl CreateAccount<'_> {
    #[inline(always)]
    pub fn invoke(&self) -> ProgramResult {
        self.invoke_signed(&[])
    }

    pub fn invoke_signed(&self, signers: &[Signer]) -> ProgramResult {
        // account metadata
        let account_metas: [AccountMeta; 2] = [
            AccountMeta::writable_signer(self.from.key()),
            AccountMeta::writable_signer(self.to.key()),
        ];

        // instruction data
        // -  4 bytes discriminator
        // -  8 bytes lamports
        // -  8 bytes account space
        // - 32 bytes owner pubkey
        let mut instruction_data = [0; 52];
        instruction_data[0..4].copy_from_slice(&DISCRIMINATOR);
        instruction_data[4..12].copy_from_slice(&self.lamports.to_le_bytes());
        instruction_data[12..20].copy_from_slice(&self.space.to_le_bytes());
        instruction_data[20..52].copy_from_slice(self.owner.as_ref());

        let instruction = Instruction {
            program_id: &self.program.key(),
            accounts: &account_metas,
            data: &instruction_data,
        };

        invoke_signed(&instruction, &[self.from, self.to], signers)
    }
}
