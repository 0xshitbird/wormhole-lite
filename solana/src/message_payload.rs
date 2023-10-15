use borsh::{BorshSerialize, BorshDeserialize};


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
        self.data.serialize(writer)
    }
}


impl BorshDeserialize for Payload {
    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        let payload_id = u8::deserialize_reader(reader)?;
        let data = Vec::<u8>::deserialize_reader(reader)?;

        Ok(Self { payload_id, data })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_payload_serialize_deserialize() {
        let payload1 = Payload {
            payload_id: 69,
            data: vec![4,2,0]
        };
        let payload2 = Payload {
            payload_id: 254,
            data: vec![6, 6, 6, 6, 6, 6, 6],
        };

        let p1_ser = payload1.try_to_vec().unwrap();
        // Serialize into [payload_id, len, data...]
        assert_eq!(p1_ser, [69, 3, 0, 0, 0, 4, 2, 0]);
        let p1_der = Payload::try_from_slice(&p1_ser).unwrap();
        assert!(payload1.eq(&p1_der));

        let p2_ser = payload2.try_to_vec().unwrap();
        assert_eq!(p2_ser, [254, 7, 0, 0, 0, 6, 6, 6, 6, 6, 6, 6]);
        let p2_der = Payload::try_from_slice(&p2_ser).unwrap();
        assert!(payload2.eq(&p2_der));
    }
}