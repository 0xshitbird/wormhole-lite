use solana_sdk::secp256k1_instruction::{
    SecpSignatureOffsets, HASHED_PUBKEY_SERIALIZED_SIZE, SIGNATURE_OFFSETS_SERIALIZED_SIZE,
    SIGNATURE_SERIALIZED_SIZE,
};

#[derive(Clone, Copy)]
/// A struct to hold the values specified in the `SecpSignatureOffsets` struct.
pub struct SecpSignature {
    pub signature: [u8; SIGNATURE_SERIALIZED_SIZE],
    pub recovery_id: u8,
    pub eth_address: [u8; HASHED_PUBKEY_SERIALIZED_SIZE],
    /// this is the hash of the payload in the VAA
    pub message: [u8; 32],
}

impl Default for SecpSignature {
    fn default() -> Self {
        Self {
            signature: [0_u8; SIGNATURE_SERIALIZED_SIZE],
            recovery_id: 0,
            eth_address: [0_u8; HASHED_PUBKEY_SERIALIZED_SIZE],
            message: [0_u8; 32],
        }
    }
}

/// Create the instruction data for a secp256k1 instruction.
///
/// `instruction_index` is the index the secp256k1 instruction will appear
/// within the transaction. For simplicity, this function only supports packing
/// the signatures into the secp256k1 instruction data, and not into any other
/// instructions within the transaction.
pub fn make_secp256k1_instruction_data(
    signatures: &[SecpSignature],
    instruction_index: u8,
) -> anyhow::Result<Vec<u8>> {
    assert!(signatures.len() <= u8::max_value().into());

    // We're going to pack all the signatures into the secp256k1 instruction data.
    // Before our signatures though is the signature offset structures
    // the secp256k1 program parses to find those signatures.
    // This value represents the byte offset where the signatures begin.
    let data_start = 1 + signatures.len() * SIGNATURE_OFFSETS_SERIALIZED_SIZE;

    let mut signature_offsets = vec![];
    let mut signature_buffer = vec![];

    for signature_bundle in signatures {
        let data_start = data_start
            .checked_add(signature_buffer.len())
            .expect("overflow");

        let signature_offset = data_start;
        let eth_address_offset = data_start
            .checked_add(SIGNATURE_SERIALIZED_SIZE + 1)
            .expect("overflow");
        let message_data_offset = eth_address_offset
            .checked_add(HASHED_PUBKEY_SERIALIZED_SIZE)
            .expect("overflow");
        let message_data_size = signature_bundle.message.len();

        let signature_offset = u16::try_from(signature_offset)?;
        let eth_address_offset = u16::try_from(eth_address_offset)?;
        let message_data_offset = u16::try_from(message_data_offset)?;
        let message_data_size = u16::try_from(message_data_size)?;

        signature_offsets.push(SecpSignatureOffsets {
            signature_offset,
            signature_instruction_index: instruction_index,
            eth_address_offset,
            eth_address_instruction_index: instruction_index,
            message_data_offset,
            message_data_size,
            message_instruction_index: instruction_index,
        });

        signature_buffer.extend(signature_bundle.signature);
        signature_buffer.push(signature_bundle.recovery_id);
        signature_buffer.extend(&signature_bundle.eth_address);
        signature_buffer.extend(&signature_bundle.message);
    }

    let mut instr_data = vec![];
    instr_data.push(signatures.len() as u8);

    for offsets in signature_offsets {
        let offsets = bincode::serialize(&offsets)?;
        instr_data.extend(offsets);
    }

    instr_data.extend(signature_buffer);

    Ok(instr_data)
}
