use solana_program::pubkey::Pubkey;

/// derives the message PDA, with the nonce being the sequence number
/// of the sequence used when publishing a message.
/// 
/// program_id is the addres of the program which will be signing an instruction with this address
pub fn derive_message_pda(program_id: Pubkey, nonce: u64) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            b"message",
            &nonce.to_le_bytes(),
        ],
        &program_id
    )
}