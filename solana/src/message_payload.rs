use borsh::{BorshDeserialize, BorshSerialize};

/// an object representing an arbitrary payload to relay through wormhole, whereby the
/// `payload_id` is used to identify the specific instruction/function to execute and
/// `data` is the actual data of the instruction or function call
#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub struct Payload {
    /// payload_id is used to identify the type of payload being sent, and is application specific
    pub payload_id: u8,
    /// the actual data contained by the payload, limited to 1024 bytes due to solana based constraints
    pub data: Vec<u8>,
}

impl BorshSerialize for Payload {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        self.payload_id.serialize(writer)?;
        // serialize the length of the data first
        (self.data.len() as u16).to_be_bytes().serialize(writer)?;
        // serialize the actual data
        for item in &self.data {
            (*item).serialize(writer)?;
        }
        Ok(())
    }
}

impl BorshDeserialize for Payload {
    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        let mut data = Vec::with_capacity(1024);
        reader.read_to_end(&mut data)?;
        let payload_id = data[0];
        let length = {
            let mut out = [0u8; 2];
            out.copy_from_slice(&data[1..3]);
            u16::from_be_bytes(out) as usize
        };
        let data = data[3..(3 + length)].to_vec();
        Ok(Self { payload_id, data })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_wormhole_example() {
        let payload = Payload {
            payload_id: 1,
            data: b"Hello World".to_vec(),
        };
        let ser_p = payload.try_to_vec().unwrap();
        println!("{}", hex::encode(&ser_p));
        let payload2 = Payload::try_from_slice(&ser_p[..]).unwrap();
        assert_eq!(payload.data, payload2.data);
    }
}
