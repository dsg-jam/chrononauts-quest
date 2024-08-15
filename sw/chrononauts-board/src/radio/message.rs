use super::MAX_PACKET_SIZE;

#[derive(Debug, thiserror::Error)]
pub enum MessageError {
    #[error("Missing opcode")]
    MissingOpcode,
    #[error("Invalid opcode")]
    InvalidOpcode,
    #[error("Invalid payload length")]
    InvalidPayloadLength,
}

pub trait ToBytes {
    fn to_bytes(&self) -> heapless::Vec<u8, MAX_PACKET_SIZE>;
}

pub trait FromBytes {
    fn from_bytes(value: &[u8]) -> Result<Self, MessageError>
    where
        Self: Sized;
}

/// ChrononautsPackage struct
/// Represents a Chrononauts message, which is composed of a header and a payload.
///
/// The package length is limited to 61 bytes, as the maximum packet size is 64 bytes,
/// with 3 bytes reserved for the length and RSSI/LQI.
///
/// The package is structured as follows:
/// Header (3 bytes) | Payload (max 58 bytes)
///
#[derive(Debug, Clone, Copy)]
pub struct ChrononautsPackage {
    pub header: ChrononautsHeader,
    pub payload: ChrononautsPayload,
}

impl ChrononautsPackage {
    pub fn new(header: ChrononautsHeader, payload: ChrononautsPayload) -> Self {
        ChrononautsPackage { header, payload }
    }
}

impl ToBytes for ChrononautsPackage {
    fn to_bytes(&self) -> heapless::Vec<u8, MAX_PACKET_SIZE> {
        let mut data = heapless::Vec::new();
        data.extend(self.header.to_bytes());
        data.extend(self.payload.to_bytes());
        data
    }
}

impl FromBytes for ChrononautsPackage {
    fn from_bytes(value: &[u8]) -> Result<Self, MessageError> {
        if value.len() < 3 {
            return Err(MessageError::InvalidPayloadLength);
        }
        let header = ChrononautsHeader::from_bytes(&value[..3])?;
        let payload = ChrononautsPayload::from_bytes(&value[3..])?;
        Ok(ChrononautsPackage { header, payload })
    }
}

/// ChrononautsHeader struct
/// Represents the header of a Chrononauts message.
///
/// The header is composed of 3 bytes:
/// - Source (1 byte)
/// - Destination (1 byte)
/// - Payload length (1 byte)
///
#[derive(Debug, Clone, Copy)]
pub struct ChrononautsHeader {
    pub source: u8,
    pub destination: u8,
    pub payload_length: u8,
}

impl ChrononautsHeader {
    pub fn new(source: u8, destination: u8, payload_length: u8) -> Self {
        ChrononautsHeader {
            source,
            destination,
            payload_length,
        }
    }
}

impl ToBytes for ChrononautsHeader {
    fn to_bytes(&self) -> heapless::Vec<u8, MAX_PACKET_SIZE> {
        let mut data = heapless::Vec::new();
        data.push(self.source).unwrap();
        data.push(self.destination).unwrap();
        data.push(self.payload_length).unwrap();
        data
    }
}

impl FromBytes for ChrononautsHeader {
    fn from_bytes(value: &[u8]) -> Result<Self, MessageError> {
        if value.len() < 3 {
            return Err(MessageError::InvalidPayloadLength);
        }
        Ok(ChrononautsHeader {
            source: value[0],
            destination: value[1],
            payload_length: value[2],
        })
    }
}

/// ChrononautsPayload enum
/// Represents the payload of a Chrononauts message.
///
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum ChrononautsPayload {
    SyncRequest = 0x00,
    SyncResponse = 0x01,
    Ack = 0x02,
    Nack = 0x03,
    StartGame = 0x04,
    EndGame = 0x05,
    SetLevel(u8) = 0x06,
}

impl ChrononautsPayload {}

impl From<ChrononautsPayload> for u8 {
    fn from(value: ChrononautsPayload) -> u8 {
        match value {
            ChrononautsPayload::SyncRequest => 0x00,
            ChrononautsPayload::SyncResponse => 0x01,
            ChrononautsPayload::Ack => 0x02,
            ChrononautsPayload::Nack => 0x03,
            ChrononautsPayload::StartGame => 0x04,
            ChrononautsPayload::EndGame => 0x05,
            ChrononautsPayload::SetLevel(_) => 0x06,
        }
    }
}

impl ToBytes for ChrononautsPayload {
    fn to_bytes(&self) -> heapless::Vec<u8, MAX_PACKET_SIZE> {
        let mut data = heapless::Vec::new();
        data.push((*self).into()).unwrap();
        if let ChrononautsPayload::SetLevel(level) = self {
            data.push(*level).unwrap();
        }
        data
    }
}

impl FromBytes for ChrononautsPayload {
    fn from_bytes(value: &[u8]) -> Result<Self, MessageError> {
        let opcode = value.first().ok_or(MessageError::MissingOpcode)?;
        Ok(match opcode {
            0x00 => ChrononautsPayload::SyncRequest,
            0x01 => ChrononautsPayload::SyncResponse,
            0x02 => ChrononautsPayload::Ack,
            0x03 => ChrononautsPayload::Nack,
            0x04 => ChrononautsPayload::StartGame,
            0x05 => ChrononautsPayload::EndGame,
            0x06 => {
                if value.len() < 2 {
                    return Err(MessageError::InvalidPayloadLength);
                }
                ChrononautsPayload::SetLevel(value[1])
            }
            _ => return Err(MessageError::InvalidOpcode),
        })
    }
}
