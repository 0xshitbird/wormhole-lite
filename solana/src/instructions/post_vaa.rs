use std::io::Cursor;

use borsh::BorshSerialize;
use solana_program::{
    entrypoint::ProgramResult,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    sysvar,
};
use wormhole_anchor_sdk::wormhole::Instruction as WormholeIx;

use crate::WORMHOLE_PROGRAM_ID;

/// The actual VAA which we are posting to the bridge and verifying
///
/// To view the VAA you can navigate to https://wormholescan.io/#/tx/<TX_HASH>.
/// From this we want the following values:
///  version
///  guardianSetIndex
///  timestamp
///  nonce
///  emitterChain
///  emitterAddress
///  sequence
///  consistencyLevel
///  payload
///
/// The above api call returns a `hash` parameter which is the payload hash
#[derive(Clone, PartialEq, Debug)]
pub struct PostVAADataIx {
    pub version: u8,
    pub guardian_set_index: u32,
    pub timestamp: u32,
    pub nonce: u32,
    pub emitter_chain: u16,
    pub emitter_address: [u8; 32],
    pub sequence: u64,
    pub consistency_level: u8,
    pub payload: Vec<u8>,
}

impl PostVAADataIx {
    /// derives the guardian set account which stores information about the 
    /// guardians who signed teh vaa
    pub fn derive_guardian_set(&self) -> (Pubkey, u8) {
        crate::utils::derivations::derive_guardian_set(self.guardian_set_index)
    }
    /// given the vaa paylaod hash, return the account used for storing its information
    pub fn derive_posted_vaa_account(&self) -> (Pubkey, u8) {
        let payload_hash = hash_vaa(self).to_vec();
        crate::utils::derivations::derive_posted_vaa(&payload_hash)
    }
    /// hashes the serialized vaa, which is the data signed by the guardian network
    pub fn hash_vaa(&self) -> [u8; 32] {
        hash_vaa(self)
    }
}

// Convert a full VAA structure into the serialization of its unique components, this structure is
// what is hashed and verified by Guardians.
pub fn serialize_vaa(vaa: &PostVAADataIx) -> Vec<u8> {
    use std::io::Write;
    let mut v = Cursor::new(Vec::new());
    v.write(&vaa.timestamp.to_be_bytes()).unwrap();
    v.write(&vaa.nonce.to_be_bytes()).unwrap();
    v.write(&vaa.emitter_chain.to_be_bytes()).unwrap();
    v.write(&vaa.emitter_address).unwrap();
    v.write(&vaa.sequence.to_be_bytes()).unwrap();
    v.write(&[vaa.consistency_level]).unwrap();
    v.write(&vaa.payload).unwrap();
    v.into_inner()
}

// Hash a VAA, this combines serialization and hashing.
pub fn hash_vaa(vaa: &PostVAADataIx) -> [u8; 32] {
    use sha3::Digest;
    use std::io::Write;
    let body = serialize_vaa(vaa);
    let mut h = sha3::Keccak256::default();
    h.write_all(body.as_slice()).unwrap();
    h.finalize().into()
}

impl From<PostVAADataIx> for WormholeIx {
    fn from(value: PostVAADataIx) -> Self {
        Self::PostVAA {
            version: value.version,
            guardian_set_index: value.guardian_set_index,
            timestamp: value.timestamp,
            nonce: value.nonce,
            emitter_chain: value.emitter_chain,
            emitter_address: value.emitter_address,
            sequence: value.sequence,
            consistency_level: value.consistency_level,
            payload: value.payload,
        }
    }
}

/// creates a post_vaa instruction which should be invoked after running
/// the verify_signature instruction
pub fn create_post_vaa_ix(
    vaa_data: PostVAADataIx,
    payer: Pubkey,
    signature_set: Pubkey,
) -> Option<Instruction> {
    let (posted_vaa, _) = vaa_data.derive_posted_vaa_account();
    let (guardian_set, _) = vaa_data.derive_guardian_set();
    let ix: WormholeIx = From::from(vaa_data);
    match ix {
        WormholeIx::PostVAA { .. } => Some(Instruction {
            program_id: WORMHOLE_PROGRAM_ID,
            accounts: vec![
                AccountMeta::new_readonly(guardian_set, false),
                AccountMeta::new_readonly(
                    crate::utils::derivations::derive_core_bridge_config().0,
                    false,
                ),
                AccountMeta::new_readonly(signature_set, false),
                AccountMeta::new(posted_vaa, false), // aka message
                AccountMeta::new(payer, true),
                AccountMeta::new_readonly(sysvar::clock::id(), false),
                AccountMeta::new_readonly(sysvar::rent::id(), false),
                AccountMeta::new_readonly(solana_program::system_program::id(), false),
            ],
            data: ix.try_to_vec().ok()?,
        }),
        _ => None,
    }
}
