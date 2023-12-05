use std::fmt::Write;

use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Serialize, Deserialize};
use solana_program::pubkey::Pubkey;

#[repr(transparent)]
#[derive(Default)]
pub struct PostedMessageData {
    pub message: MessageData,
}


#[derive(Debug, Default, BorshSerialize, BorshDeserialize, Clone, Serialize, Deserialize)]
pub struct MessageData {
    /// Header of the posted VAA
    pub vaa_version: u8,

    /// Level of consistency requested by the emitter
    pub consistency_level: u8,

    /// Time the vaa was submitted
    pub vaa_time: u32,

    /// Account where signatures are stored
    pub vaa_signature_account: Pubkey,

    /// Time the posted message was created
    pub submission_time: u32,

    /// Unique nonce for this message
    pub nonce: u32,

    /// Sequence number of this message
    pub sequence: u64,

    /// Emitter of the message
    pub emitter_chain: u16,

    /// Emitter of the message
    pub emitter_address: [u8; 32],

    /// Message payload
    pub payload: Vec<u8>,
}

#[repr(transparent)]
#[derive(Default)]
pub struct PostedVAAData {
    pub message: MessageData,
}

impl BorshSerialize for PostedVAAData {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        writer.write_all(b"vaa")?;
        BorshSerialize::serialize(&self.message, writer)
    }
}

impl BorshDeserialize for PostedVAAData {
    fn deserialize_reader<R: std::io::Read>(reader: &mut R)  -> std::io::Result<Self> {
        let mut buf = Vec::with_capacity(128);
        reader.read_to_end(&mut buf)?;

        if buf.len() < 3 {
            return Err(std::io::ErrorKind::Other.into());
        }

        // We accept "vaa", "msg", or "msu" because it's convenient to read all of these as PostedVAAData
        let expected: [&[u8]; 3] = [b"vaa", b"msg", b"msu"];
        let magic: &[u8] = &buf[0..3];
        if !expected.contains(&magic) {
            println!("magic mismatch");
            return Err(std::io::ErrorKind::InvalidData.into());
        };
        Ok(PostedVAAData {
            message: <MessageData as BorshDeserialize>::deserialize(&mut &buf[3..])?,
        })
    }
}

impl std::ops::Deref for PostedVAAData {
    type Target = MessageData;

    fn deref(&self) -> &Self::Target {
        &self.message
    }
}

impl std::ops::DerefMut for PostedVAAData {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.message
    }
}

impl Clone for PostedVAAData {
    fn clone(&self) -> Self {
        PostedVAAData {
            message: self.message.clone(),
        }
    }
}

impl BorshSerialize for PostedMessageData {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        writer.write_all(b"msg")?;
        BorshSerialize::serialize(&self.message, writer)
    }
}

impl BorshDeserialize for PostedMessageData {
    fn deserialize_reader<R: std::io::Read>(reader: &mut R)  -> std::io::Result<Self> {
        let mut buf = Vec::with_capacity(128);
        reader.read_to_end(&mut buf)?;

        if buf.len() < 3 {
            return Err(std::io::ErrorKind::Other.into());
        }

        let expected = b"msg";
        let magic: &[u8] = &buf[0..3];
        if magic != expected {
            println!(
                "Magic mismatch. Expected {:?} but got {:?}",
                expected, magic
            );
            return Err(std::io::ErrorKind::InvalidData.into());
        };
        Ok(PostedMessageData {
            message: <MessageData as BorshDeserialize>::deserialize(&mut &buf[3..])?,
        })
    }
}



impl std::ops::Deref for PostedMessageData {
    type Target = MessageData;

    fn deref(&self) -> &Self::Target {
        &self.message
    }
}
