use solana_program::pubkey::Pubkey;
use wormhole_anchor_sdk::wormhole::SEED_PREFIX_EMITTER;

use crate::WORMHOLE_PROGRAM_ID;

/// derives the message PDA, with the nonce being the sequence number
/// of the sequence used when publishing a message.
///
/// program_id is the addres of the program which will be signing an instruction with this address
pub fn derive_message_pda(program_id: Pubkey, nonce: u64) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"message", &nonce.to_le_bytes()], &program_id)
}

/// derives the address used as the core emitter sequence account
/// we must include the pda of the emitter that we derived (see: derive_emitter function)
/// because this is a pda used for verification, we use our program id as the seed
pub fn derive_sequence(emitter_pda: Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"Sequence", emitter_pda.as_ref()],
        &crate::WORMHOLE_PROGRAM_ID,
    )
}

/// derive the emitter pda, where executing_program_id is the program
/// that will be using the emitter to sign cpi instructions
pub fn derive_emitter(executing_program_id: Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[SEED_PREFIX_EMITTER], &executing_program_id)
}
/// derives the address of the core bridge config program
pub fn derive_core_bridge_config() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"Bridge"], &WORMHOLE_PROGRAM_ID)
}

/// derives the wormhole fee collector program
pub fn derive_core_fee_collector() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"fee_collector"], &WORMHOLE_PROGRAM_ID)
}

/// derives the guardian set pda
pub fn derive_guardian_set(guardian_set_index: u32) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"GuardianSet", &guardian_set_index.to_be_bytes()[..]],
        &WORMHOLE_PROGRAM_ID,
    )
}

/// derives the posted vaa account
pub fn derive_posted_vaa(payload_hash: &[u8]) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"PostedVAA", &payload_hash], &WORMHOLE_PROGRAM_ID)
}

#[cfg(test)]
mod test {
    use solana_program::system_program;

    use super::*;
    #[test]
    fn test_derive_emitter() {
        let (pda, nonce) = derive_emitter(system_program::id());
        assert_eq!(
            pda.to_string(),
            "6TsAgEkaXfrUMW3hcLgiZXUehUhcaRkaRY3fjhrfadye"
        );
        assert_eq!(nonce, 255);
    }
    #[test]
    fn test_derive_sequence() {
        let (pda, nonce) = derive_emitter(system_program::id());
        assert_eq!(
            pda.to_string(),
            "6TsAgEkaXfrUMW3hcLgiZXUehUhcaRkaRY3fjhrfadye"
        );
        assert_eq!(nonce, 255);
        let (pda, nonce) = derive_sequence(pda);
        assert_eq!(
            pda.to_string(),
            "3PqEpt2V26bEkCjcef9crtzLtkHYBMQMCBedneayXXPd"
        );
        assert_eq!(nonce, 254);
    }
    #[test]
    fn test_derive_message_pda() {
        let (pda, nonce) = derive_message_pda(system_program::id(), 69);
        assert_eq!(
            pda.to_string(),
            "7ivBfWmf54DHwNp437fZtfXDd5TtXfWo5Q4YGnk7xrRB"
        );
        assert_eq!(nonce, 254);
    }
}
