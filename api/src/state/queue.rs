use steel::*;
use borsh::{BorshDeserialize, BorshSerialize};

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct QueueAccount {
    pub oracle: Vec<Oracle>,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct Oracle {
    pub identity: Pubkey,
    pub oracle_publickey: Pubkey,
    pub registration_slot: u64,
}