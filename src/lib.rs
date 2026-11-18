mod pb;

use pb::solana::transfers::v1::{Transfer, Transfers};
use substreams::errors::Error;
use substreams_solana::pb::sf::solana::r#type::v1::Block;

const SYSTEM_PROGRAM_ID: [u8; 32] = [0u8; 32];

fn decode_system_transfer(
    program_id: &[u8],
    account_indices: &[u8],
    data: &[u8],
    resolved_accounts: &[Vec<u8>],
) -> Option<Transfer> {
    if program_id != SYSTEM_PROGRAM_ID || data.len() < 12 {
        return None;
    }
    let discriminant = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    match discriminant {
        2 if data.len() == 12 && account_indices.len() >= 2 => {
            let lamports = u64::from_le_bytes(data[4..12].try_into().ok()?);
            let from_bytes = resolved_accounts.get(account_indices[0] as usize)?;
            let to_bytes = resolved_accounts.get(account_indices[1] as usize)?;
            Some(Transfer {
                from: bs58::encode(from_bytes).into_string(),
                to: bs58::encode(to_bytes).into_string(),
                amount: lamports,
            })
        }
        11 if account_indices.len() >= 3 => {
            let lamports = u64::from_le_bytes(data[4..12].try_into().ok()?);
            let from_bytes = resolved_accounts.get(account_indices[0] as usize)?;
            let to_bytes = resolved_accounts.get(account_indices[2] as usize)?;
            Some(Transfer {
                from: bs58::encode(from_bytes).into_string(),
                to: bs58::encode(to_bytes).into_string(),
                amount: lamports,
            })
        }
        _ => None,
    }
}

#[substreams::handlers::map]
fn map_transfers(block: Block) -> Result<Transfers, Error> {
    let mut transfers = Vec::new();

    for confirmed_tx in &block.transactions {
        let meta = match &confirmed_tx.meta {
            Some(m) if m.err.is_none() => m,
            _ => continue,
        };
        let tx = match &confirmed_tx.transaction {
            Some(t) => t,
            None => continue,
        };
        let msg = match &tx.message {
            Some(m) => m,
            None => continue,
        };

        let mut resolved_accounts: Vec<Vec<u8>> = msg.account_keys.clone();
        resolved_accounts.extend(meta.loaded_writable_addresses.iter().cloned());
        resolved_accounts.extend(meta.loaded_readonly_addresses.iter().cloned());

        for ix in &msg.instructions {
            if let Some(program_id) = resolved_accounts.get(ix.program_id_index as usize) {
                if let Some(transfer) =
                    decode_system_transfer(program_id, &ix.accounts, &ix.data, &resolved_accounts)
                {
                    transfers.push(transfer);
                }
            }
        }

        for inner_group in &meta.inner_instructions {
            for ix in &inner_group.instructions {
                if let Some(program_id) = resolved_accounts.get(ix.program_id_index as usize) {
                    if let Some(transfer) = decode_system_transfer(
                        program_id,
                        &ix.accounts,
                        &ix.data,
                        &resolved_accounts,
                    ) {
                        transfers.push(transfer);
                    }
                }
            }
        }
    }

    Ok(Transfers { transfers })
}
