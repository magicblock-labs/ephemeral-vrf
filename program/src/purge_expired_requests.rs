use ephemeral_vrf_api::prelude::*;
use steel::*;

/// Remove all requests in the queue whose age (current_slot - item.slot)
/// exceeds the TTL.
///
/// Accounts:
/// 0. `[]` oracle_info               – The oracle public key used in the queue PDA seeds
/// 1. `[writable]` oracle_queue_info – The oracle queue account (PDA)
///
/// Requirements:
/// - No signer needed (permissionless), anyone can call.
/// - oracle_queue_info must match seeds [QUEUE, oracle_info.key, [index]].
pub fn process_purge_expired_requests(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
    let args = PurgeExpiredRequests::try_from_bytes(data)?;

    // Accounts
    let [oracle_info, oracle_queue_info] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    // Validate queue PDA seeds and ownership / writability
    oracle_queue_info
        .is_writable()?
        .has_owner(&ephemeral_vrf_api::ID)?
        .has_seeds(
            &[QUEUE, oracle_info.key.to_bytes().as_ref(), &[args.index]],
            &ephemeral_vrf_api::ID,
        )?;

    let current_slot = Clock::get()?.slot;

    // Load queue
    let queue = oracle_queue_info.as_account_mut::<Queue>(&ephemeral_vrf_api::ID)?;

    // Scan and remove expired items.
    let mut total_cost: u64 = 0;
    for i in 0..MAX_QUEUE_ITEMS {
        if queue.used_bitmap.0[i] == 1 {
            let item = queue.items.0[i];
            let age = current_slot.saturating_sub(item.slot);
            if age > QUEUE_TTL_SLOTS {
                let cost = if item.priority_request == 1 {
                    VRF_HIGH_PRIORITY_LAMPORTS_COST
                } else {
                    VRF_LAMPORTS_COST
                };
                total_cost = total_cost.saturating_add(cost);
                let _ = queue.remove_item(i);
            }
        }
    }

    // Send the fees to the oracle.
    // The oracle also accrue fees on malformed/expired requests to
    // 1) incentivize queue cleaning and
    // 2) disincentivize creation of malformed requests
    if total_cost > 0 {
        crate::fees::transfer_fee(oracle_queue_info, oracle_info, total_cost)?;
    }

    Ok(())
}
