use borsh::BorshSerialize;
use solana_program::{entrypoint::ProgramResult, instruction::Instruction, pubkey::Pubkey};
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
    pub fn derive_guardian_set(&self) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[b"GuardianSet", &self.guardian_set_index.to_be_bytes()[..]], &WORMHOLE_PROGRAM_ID)
    }
}

#[derive(FromAccounts)]
pub struct PostVAA<'b> {
    /// Information about the current guardian set.
    pub guardian_set: GuardianSet<'b, { AccountState::Initialized }>,

    /// Bridge Info
    pub bridge_info: Bridge<'b, { AccountState::Initialized }>,

    /// Signature Info
    pub signature_set: SignatureSet<'b, { AccountState::Initialized }>,

    /// Message the VAA is associated with.
    pub message: Mut<PostedVAA<'b, { AccountState::MaybeInitialized }>>,

    /// Account used to pay for auxillary instructions.
    pub payer: Mut<Signer<Info<'b>>>,

    /// Clock used for timestamping.
    pub clock: Sysvar<'b, Clock>,
}
pub struct PostVAAAccounts {
    pub guardian_set: Pubkey,
    pub core_bridge: Pubkey,
    pub message: Pubkey,
    pub payer: Pubkey,
    pub clock: Pubkey,
}

pub fn derive_posted_vaa_account(payload_hash: &[u8]) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"PostedVAA".to_vec(),
        payload_hash], 

    )
    vec![b"PostedVAA".to_vec(), data.payload_hash.to_vec()]
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
            payload: value.payload }
    }
}


pub fn create_post_vaa_ix(ix: WormholeIx) ->  Option<Instruction> {
    match ix {
        WormholeIx::PostVAA { .. } => {
            Some(Instruction {
                program_id: WORMHOLE_PROGRAM_ID,
                accounts: vec![],
               data: ix.try_to_vec().ok()?
            })
        }
        _ => None
    }
}