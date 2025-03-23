use anchor_lang::prelude::borsh::{BorshDeserialize, BorshSerialize};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::solana_program::system_program;
use anchor_lang::solana_program::sysvar::slot_hashes;
use crate::instruction::ConsumeRandomness;
use anchor_lang::solana_program::hash::hash;
use anchor_lang::solana_program::program::invoke;

declare_id!("AL32mNVFdhxHXztaWuNWvwoiPYCHofWmVRNH49pMCafD");

#[program]
pub mod use_randomness {
    use super::*;

    pub fn request_randomness(ctx: Context<RequestRandomnessCtx>, client_seed: u8) -> Result<()> {
        msg!(
            "Generating a random number: (from program: {:?})",
            ctx.program_id
        );
        let ix = create_request_randomness_ix(
            ctx.accounts.payer.key(),
            ctx.accounts.oracle_queue.key(),
            ID,
            ConsumeRandomness::DISCRIMINATOR,
            None,
            hash(&[client_seed]).to_bytes(),
            None,
        );
        invoke(
            &ix,
            &[
                ctx.accounts.payer.to_account_info(),
                ctx.accounts.oracle_queue.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
                ctx.accounts.slot_hashes.to_account_info(),
            ],
        )?;
        Ok(())
    }

    pub fn consume_randomness(ctx: Context<ConsumeRandomnessCtx>, randomness: [u8; 32]) -> Result<()> {
        // If the PDA identity is a signer, this means the VRF program is the caller
        msg!("VRF identity: {:?}", ctx.accounts.vrf_program_identity.key());
        msg!("VRF identity is signer: {:?}", ctx.accounts.vrf_program_identity.is_signer);
        // We can safely consume the randomness
        msg!(
            "Consuming random number: {:?}",
            random_u32(&randomness)
        );
        Ok(())
    }
}

#[derive(Accounts)]
pub struct RequestRandomnessCtx<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    /// CHECK: Oracle queue
    #[account(mut, address = DEFAULT_QUEUE)]
    pub oracle_queue: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    /// CHECK: Slot hashes sysvar
    #[account(address = slot_hashes::ID)]
    pub slot_hashes: AccountInfo<'info>,
    pub vrf_program: Program<'info, VrfProgram>,
}

#[derive(Accounts)]
pub struct ConsumeRandomnessCtx<'info> {
    #[account(address = VRF_PROGRAM_IDENTITY)]
    pub vrf_program_identity: Signer<'info>,
}

/// SDK methods
pub fn create_request_randomness_ix(
    payer: Pubkey,
    oracle_queue: Pubkey,
    callback_program_id: Pubkey,
    callback_discriminator: &[u8],
    accounts_metas: Option<Vec<SerializableAccountMeta>>,
    caller_seed: [u8; 32],
    callback_args: Option<Vec<u8>>,
) -> Instruction {
    Instruction {
        program_id: VRF_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(payer, true),
            AccountMeta::new(oracle_queue, false),
            AccountMeta::new_readonly(system_program::ID, false),
            AccountMeta::new_readonly(slot_hashes::ID, false),
        ],
        data: RequestRandomness {
            caller_seed,
            callback_program_id,
            callback_discriminator: callback_discriminator.to_vec(),
            callback_accounts_metas: accounts_metas.unwrap_or(vec![]),
            callback_args: callback_args.unwrap_or(vec![]),
        }.to_bytes(),
    }
}

#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Default)]
pub struct RequestRandomness {
    pub caller_seed: [u8; 32],
    pub callback_program_id: Pubkey,
    pub callback_discriminator: Vec<u8>,
    pub callback_accounts_metas: Vec<SerializableAccountMeta>,
    pub callback_args: Vec<u8>,
}

impl RequestRandomness {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![3, 0, 0, 0, 0, 0, 0, 0];
        self.serialize(&mut bytes).unwrap();
        bytes
    }
}

pub const DEFAULT_QUEUE: Pubkey =  pubkey!("4tFFjWnz1qZDJEskJXjxdMzdv71v16ukAPiRqiAbXJ3L");
pub const VRF_PROGRAM_ID: Pubkey = pubkey!("VrffXU38S8MzqTtTYQG3M8GNwheKH8n77HVEZUdakH8");
pub const VRF_PROGRAM_IDENTITY: Pubkey = pubkey!("AwF6egvgtC2RdkfUEcCCtjHP2iWhCzFBMi1a6bjv9Hkp");

pub struct VrfProgram;

impl Id for VrfProgram {
    fn id() -> Pubkey {
        VRF_PROGRAM_ID
    }
}

#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Default, Clone)]
pub struct SerializableAccountMeta {
    pub pubkey: Pubkey,
    pub is_signer: bool,
    pub is_writable: bool,
}

pub fn random_u32(bytes: &[u8; 32]) -> u32 {
    u32::from_le_bytes([
        bytes[0],
        bytes[3],
        bytes[7],
        bytes[12],
    ])
}