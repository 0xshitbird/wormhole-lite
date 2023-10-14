use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use solana_program::{
    program_pack::{self, IsInitialized, Sealed},
    pubkey::Pubkey,
};
use wormhole_anchor_sdk::wormhole::SEED_PREFIX_EMITTER;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
/// account used for signing and publishing messages to wormhole
pub struct Emitter {
    /// program which owns the emitter account
    pub owner: Pubkey,
    /// nonce used in the derivation process
    pub nonce: u8,
    /// the nonce to use when the emitter next publishes a message
    ///
    /// this must be incremented after successfully publishing a message
    pub next_publishable_nonce: u64,
    /// padding reserved for future use
    pub padding: [u8; 32],
}

impl Emitter {
    /// returns the common seed used for wormhole emitters
    pub fn seed() -> &'static [u8] {
        SEED_PREFIX_EMITTER
    }
    /// derive the sequence account which uses the emitter account as a seed
    pub fn derive_sequence(&self) -> (Pubkey, u8) {
        let (emitter_pda, _) = self.derive();
        crate::utils::derivations::derive_sequence(emitter_pda)
    }
    /// derives the pda of the emitter, where program_id is the address
    /// of the program that will own this account
    pub fn derive(&self) -> (Pubkey, u8) {
        crate::utils::derivations::derive_emitter(self.owner)
    }
    /// given a slice of bytes, extract the last published nonce for "zero copy access"
    ///
    /// VALIDATE THE SLICE OF BYTES BEFORE CALLING
    pub fn slice_next_publishable_nonce(input: &[u8]) -> u64 {
        let mut data: [u8; 8] = [0_u8; 8];
        data.copy_from_slice(&input[33..41]);
        u64::from_le_bytes(data)
    }
    pub fn increment_publishable_nonce(&mut self) {
        self.next_publishable_nonce = self.next_publishable_nonce.checked_add(1).unwrap();
    }
}

impl Sealed for Emitter {}
impl IsInitialized for Emitter {
    fn is_initialized(&self) -> bool {
        self.owner.ne(&Pubkey::default())
    }
}

impl program_pack::Pack for Emitter {
    const LEN: usize = 73;
    fn unpack_from_slice(src: &[u8]) -> Result<Self, solana_program::program_error::ProgramError> {
        let src = array_ref![src, 0, 73];
        let (owner, pda_nonce, next_publishable_nonce, padding) = array_refs![src, 32, 1, 8, 32];
        Ok(Self {
            owner: Pubkey::new_from_array(*owner),
            next_publishable_nonce: u64::from_le_bytes(*next_publishable_nonce),
            nonce: pda_nonce[0],
            padding: *padding,
        })
    }
    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, 73];
        let (_owner, _pda_nonce, _next_publishable_nonce, _padding) =
            mut_array_refs![dst, 32, 1, 8, 32];
        let Emitter {
            ref owner,
            ref nonce,
            ref next_publishable_nonce,
            ref padding,
        } = self;
        _owner.copy_from_slice(owner.as_ref());
        _pda_nonce[0] = *nonce;
        _next_publishable_nonce.copy_from_slice(&next_publishable_nonce.to_le_bytes());
        _padding.copy_from_slice(padding);
    }
}

#[cfg(test)]
mod test {
    use solana_program::{program_pack::Pack, system_program};

    use crate::WORMHOLE_PROGRAM_ID;

    use super::*;
    #[test]
    fn test_dex_emitter_unpack_pack() {
        let (pda, nonce) = crate::utils::derivations::derive_emitter(WORMHOLE_PROGRAM_ID);
        let et = Emitter {
            owner: WORMHOLE_PROGRAM_ID,
            nonce: nonce,
            next_publishable_nonce: 69,
            padding: [1_u8; 32],
        };
        let mut buffer: [u8; 73] = [0_u8; 73];
        Emitter::pack(et, &mut buffer).unwrap();
        let mut et2 = Emitter::unpack(&buffer[..]).unwrap();
        assert_eq!(et, et2);

        let nonce = Emitter::slice_next_publishable_nonce(&buffer[..]);
        assert_eq!(nonce, et2.next_publishable_nonce);

        et2.increment_publishable_nonce();
        assert_eq!(et2.next_publishable_nonce, 70);

        Emitter::pack(et2, &mut buffer).unwrap();

        let et3 = Emitter::unpack(&buffer[..]).unwrap();
        assert_eq!(et3, et2);
        assert_eq!(et3.padding, et.padding);
        let nonce2 = Emitter::slice_next_publishable_nonce(&buffer[..]);
        assert_eq!(nonce2, et2.next_publishable_nonce);
        assert_eq!(nonce, et.next_publishable_nonce);
        let got_pda = et3.derive().0;
        let got_seq = et3.derive_sequence().0;
        assert_eq!(
            got_pda.to_string(),
            "BJnzPydW9tofuktPJzDVWhTgsmtuZBvwxi9rzJBwPH52"
        );
        assert_eq!(pda, got_pda);
        assert_eq!(
            got_seq.to_string(),
            "4C33zbgcszH7DqsxQh8Jw3BN3WWfMLAG5nDPENBTZaWX"
        );
    }
}
