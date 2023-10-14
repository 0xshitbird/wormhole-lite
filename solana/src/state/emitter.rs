use arrayref::{array_ref, array_refs, array_mut_ref, mut_array_refs};
use solana_program::{pubkey::Pubkey, program_pack::{Sealed, IsInitialized, self}};
use wormhole_anchor_sdk::wormhole::SEED_PREFIX_EMITTER;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
/// account used for signing and publishing messages to wormhole
pub struct Emitter {
    /// program which owns the emitter account
    pub owner: Pubkey,
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
    /// derives the pda of the emitter, where program_id is the address
    /// of the program that will own this account
    pub fn derive(program_id: Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[Self::seed()],
            &program_id
        )
    }
    /// given a slice of bytes, extract the last published nonce for "zero copy access"
    /// 
    /// VALIDATE THE SLICE OF BYTES BEFORE CALLING
    pub fn slice_next_publishable_nonce(input: &[u8]) -> u64 {
        let mut data: [u8; 8] = [0_u8; 8];
        data.copy_from_slice(&input[32..40]);
        u64::from_le_bytes(data)
    }
}

impl Sealed for Emitter {}
impl IsInitialized for Emitter {
    fn is_initialized(&self) -> bool {
        self.owner.ne(&Pubkey::default())
    }
}

impl program_pack::Pack for Emitter {
    const LEN: usize = 72;
    fn unpack_from_slice(src: &[u8]) -> Result<Self, solana_program::program_error::ProgramError> {
        let src = array_ref![src, 0, 72];
        let (owner, next_publishable_nonce, padding) = array_refs![src, 32, 8, 32];
        Ok(Self {
            owner: Pubkey::new_from_array(*owner),
            next_publishable_nonce: u64::from_le_bytes(*next_publishable_nonce),
            padding: *padding
        })
    }
    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, 72];
        let (
            _owner,
            _next_publishable_nonce,
            _padding
        ) = mut_array_refs![dst, 32, 8, 32];
        let Emitter {
            ref owner,
            ref next_publishable_nonce,
            ref padding
        } = self;
        _owner.copy_from_slice(owner.as_ref());
        _next_publishable_nonce.copy_from_slice(&next_publishable_nonce.to_le_bytes());
        _padding.copy_from_slice(padding);
    }
}


#[cfg(test)]
mod test {
    use solana_program::program_pack::Pack;

    use super::*;
    #[test]
    fn test_dex_emitter_unpack_pack() {
        let et = Emitter {
            owner: Pubkey::new_unique(),
            next_publishable_nonce: 69,
            padding: [1_u8; 32]
        };
        let mut buffer: [u8; 72] = [0_u8; 72];
        Emitter::pack(et, &mut buffer).unwrap();
        let et2 = Emitter::unpack(&buffer[..]).unwrap();
        assert_eq!(et, et2);
    }
}