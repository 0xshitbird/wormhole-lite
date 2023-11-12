use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    sysvar,
};
use wormhole_anchor_sdk::wormhole::Instruction as WormholeIx;

use crate::WORMHOLE_PROGRAM_ID;

/// the maximum amount of guardian keys in a single instruction
pub const MAX_LEN_GUARDIAN_KEYS: usize = 19;

#[derive(Clone, Copy, PartialEq, Debug, BorshSerialize, BorshDeserialize)]
pub struct VerifySignaturesData {
    /// instruction indices of signers (-1 for missing)
    pub signers: [i8; MAX_LEN_GUARDIAN_KEYS],
}

/// represents a guardian which participated in signing some data, whereby `index` is
/// the guardian's element index from the overall guardian set
#[derive(Clone, Copy, PartialEq, Debug, BorshSerialize, BorshDeserialize)]
pub struct GuardianSignatureMember {
    pub index: usize,
}

impl VerifySignaturesData {
    /// converts a slice of `guardianSignatures` as from https://wormholescan.io/#/tx/<TX_HASH>?view=rawdata
    /// and converts it into the VerifySignaturesData format
    pub fn parse_signature_set(members: &[GuardianSignatureMember]) -> Option<Self> {
        let mut verify_signatures = VerifySignaturesData::default();

        for member in members {
            // if the member index is greater than 18, abort
            if member.index > MAX_LEN_GUARDIAN_KEYS - 1 {
                solana_program::log::sol_log("member index greater than max");
                return None;
            }
            verify_signatures.signers[member.index] = 0;
        }
        Some(verify_signatures)
    }
}

/// initializes a default signatures data set defaulting to -1 for all members
impl Default for VerifySignaturesData {
    fn default() -> Self {
        Self {
            signers: [-1_i8; MAX_LEN_GUARDIAN_KEYS],
        }
    }
}

impl GuardianSignatureMember {
    pub fn new(index: usize) -> Self {
        Self { index }
    }
}

/// creates a new instruction for verifying guardian signature data
pub fn create_verify_signature_ix(
    payer: Pubkey,
    guardian_set_index: u32,
    signature_set: Pubkey,
    data: VerifySignaturesData,
) -> Option<Instruction> {
    let (guardian_set, _) = crate::utils::derivations::derive_guardian_set(guardian_set_index);

    Some(Instruction {
        program_id: WORMHOLE_PROGRAM_ID,

        accounts: vec![
            AccountMeta::new(payer, true),
            AccountMeta::new_readonly(guardian_set, false),
            AccountMeta::new(signature_set, true),
            AccountMeta::new_readonly(sysvar::instructions::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
            AccountMeta::new_readonly(solana_program::system_program::id(), false),
        ],

        data: WormholeIx::VerifySignatures { signers: data.signers }.try_to_vec().ok()?
    })
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_parse_signature_set() {
        let members = vec![
            GuardianSignatureMember::new(0),
            GuardianSignatureMember::new(1),
            GuardianSignatureMember::new(3),
            GuardianSignatureMember::new(4),
            GuardianSignatureMember::new(6),
            GuardianSignatureMember::new(7),
            GuardianSignatureMember::new(9),
            GuardianSignatureMember::new(11),
            GuardianSignatureMember::new(12),
            GuardianSignatureMember::new(13),
            GuardianSignatureMember::new(14),
            GuardianSignatureMember::new(16),
            GuardianSignatureMember::new(17),
        ];
        println!("{}", members.len());
        let want_members = vec![0, 1, 3, 4, 6, 7, 9, 11, 12, 13, 14, 16, 17];
        let verify_sig_data = VerifySignaturesData::parse_signature_set(&members[..]).unwrap();
        for want in want_members {
            //println!("guardian index {}, signed {}", want, verify_sig_data.signers[want as usize]);
            assert_eq!(verify_sig_data.signers[want as usize], 0_i8);
        }
    }
}
