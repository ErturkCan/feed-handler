/// Zero-copy message decoder
///
/// This decoder takes a byte buffer and returns typed references (MessageRef)
/// that point directly into the original buffer. No allocation or copying occurs
/// during decode.

use crate::protocol::*;
use byteorder::{LittleEndian, ByteOrder};
use std::mem;
use thiserror::Error;

#[derive(Error, Debug, Clone, Copy)]
pub enum DecodeError {
    #[error("buffer too small: need {need} bytes, have {have}")]
    BufferTooSmall { need: usize, have: usize },

    #[error("invalid message type: {0}")]
    InvalidMessageType(u8),

    #[error("truncated message: declared length {declared} exceeds buffer {actual}")]
    TruncatedMessage { declared: u16, actual: usize },

    #[error("invalid header")]
    InvalidHeader,

    #[error("misaligned snapshot: invalid number of levels")]
    MisalignedSnapshot,
}

pub type DecodeResult<T> = Result<T, DecodeError>;

/// Message reference types - all contain references into original buffer
pub enum MessageRef<'a> {
    AddOrder(&'a AddOrder),
    ModifyOrder(&'a ModifyOrder),
    DeleteOrder(&'a DeleteOrder),
    Trade(&'a Trade),
    Snapshot(SnapshotRef<'a>),
}

/// Reference to snapshot with dynamic level data
pub struct SnapshotRef<'a> {
    pub header: &'a SnapshotHeader,
    pub bid_levels: &'a [SnapshotLevel],
    pub ask_levels: &'a [SnapshotLevel],
}

impl<'a> SnapshotRef<'a> {
    pub fn sequence(&self) -> u32 {
        self.header.header.sequence
    }

    pub fn num_bids(&self) -> u32 {
        self.header.num_bids
    }

    pub fn num_asks(&self) -> u32 {
        self.header.num_asks
    }
}

impl<'a> MessageRef<'a> {
    /// Extract sequence number from any message
    pub fn sequence(&self) -> u32 {
        match self {
            MessageRef::AddOrder(m) => m.header.sequence,
            MessageRef::ModifyOrder(m) => m.header.sequence,
            MessageRef::DeleteOrder(m) => m.header.sequence,
            MessageRef::Trade(m) => m.header.sequence,
            MessageRef::Snapshot(s) => s.sequence(),
        }
    }

    /// Extract message type
    pub fn message_type(&self) -> MessageType {
        match self {
            MessageRef::AddOrder(_) => MessageType::AddOrder,
            MessageRef::ModifyOrder(_) => MessageType::ModifyOrder,
            MessageRef::DeleteOrder(_) => MessageType::DeleteOrder,
            MessageRef::Trade(_) => MessageType::Trade,
            MessageRef::Snapshot(_) => MessageType::Snapshot,
        }
    }
}

/// Zero-copy decoder
pub struct Decoder;

impl Decoder {
    /// Parse a single message from buffer at given offset
    /// Returns the message and the size consumed
    pub fn decode(buffer: &[u8]) -> DecodeResult<(MessageRef, usize)> {
        if buffer.len() < HEADER_SIZE {
            return Err(DecodeError::BufferTooSmall {
                need: HEADER_SIZE,
                have: buffer.len(),
            });
        }

        // Read header (8 bytes)
        let msg_type = buffer[0];
        let length = LittleEndian::read_u16(&buffer[1..3]);
        let _sequence = LittleEndian::read_u32(&buffer[3..7]);

        // Validate message type
        let msg_type_enum = MessageType::from_u8(msg_type)
            .ok_or(DecodeError::InvalidMessageType(msg_type))?;

        // Validate length
        let length = length as usize;
        if length < HEADER_SIZE || length > buffer.len() {
            return Err(DecodeError::TruncatedMessage {
                declared: length as u16,
                actual: buffer.len(),
            });
        }

        // Ensure we have the full message
        if buffer.len() < length {
            return Err(DecodeError::TruncatedMessage {
                declared: length as u16,
                actual: buffer.len(),
            });
        }

        let msg_slice = &buffer[..length];
        let consumed = length;

        let msg_ref = match msg_type_enum {
            MessageType::AddOrder => {
                if msg_slice.len() < mem::size_of::<AddOrder>() {
                    return Err(DecodeError::BufferTooSmall {
                        need: mem::size_of::<AddOrder>(),
                        have: msg_slice.len(),
                    });
                }
                let ptr = msg_slice.as_ptr() as *const AddOrder;
                let msg = unsafe { &*ptr };
                MessageRef::AddOrder(msg)
            }
            MessageType::ModifyOrder => {
                if msg_slice.len() < mem::size_of::<ModifyOrder>() {
                    return Err(DecodeError::BufferTooSmall {
                        need: mem::size_of::<ModifyOrder>(),
                        have: msg_slice.len(),
                    });
                }
                let ptr = msg_slice.as_ptr() as *const ModifyOrder;
                let msg = unsafe { &*ptr };
                MessageRef::ModifyOrder(msg)
            }
            MessageType::DeleteOrder => {
                if msg_slice.len() < mem::size_of::<DeleteOrder>() {
                    return Err(DecodeError::BufferTooSmall {
                        need: mem::size_of::<DeleteOrder>(),
                        have: msg_slice.len(),
                    });
                }
                let ptr = msg_slice.as_ptr() as *const DeleteOrder;
                let msg = unsafe { &*ptr };
                MessageRef::DeleteOrder(msg)
            }
            MessageType::Trade => {
                if msg_slice.len() < mem::size_of::<Trade>() {
                    return Err(DecodeError::BufferTooSmall {
                        need: mem::size_of::<Trade>(),
                        have: msg_slice.len(),
                    });
                }
                let ptr = msg_slice.as_ptr() as *const Trade;
                let msg = unsafe { &*ptr };
                MessageRef::Trade(msg)
            }
            MessageType::Snapshot => {
                if msg_slice.len() < mem::size_of::<SnapshotHeader>() {
                    return Err(DecodeError::BufferTooSmall {
                        need: mem::size_of::<SnapshotHeader>(),
                        have: msg_slice.len(),
                    });
                }

                let hdr_ptr = msg_slice.as_ptr() as *const SnapshotHeader;
                let hdr = unsafe { &*hdr_ptr };

                let num_bids = LittleEndian::read_u32(&hdr.num_bids.to_le_bytes()) as usize;
                let num_asks = LittleEndian::read_u32(&hdr.num_asks.to_le_bytes()) as usize;
                let expected_size =
                    mem::size_of::<SnapshotHeader>() + (num_bids + num_asks) * mem::size_of::<SnapshotLevel>();

                if msg_slice.len() < expected_size {
                    return Err(DecodeError::TruncatedMessage {
                        declared: length as u16,
                        actual: msg_slice.len(),
                    });
                }

                let levels_ptr =
                    unsafe { msg_slice.as_ptr().add(mem::size_of::<SnapshotHeader>()) } as *const SnapshotLevel;
                let bid_levels = unsafe { std::slice::from_raw_parts(levels_ptr, num_bids) };
                let ask_levels = unsafe { std::slice::from_raw_parts(levels_ptr.add(num_bids), num_asks) };

                MessageRef::Snapshot(SnapshotRef {
                    header: hdr,
                    bid_levels,
                    ask_levels,
                })
            }
        };

        Ok((msg_ref, consumed))
    }

    /// Decode a stream of messages from buffer
    /// Calls callback for each message; stops on error or if callback returns false
    pub fn decode_stream<F>(buffer: &[u8], mut callback: F) -> DecodeResult<usize>
    where
        F: FnMut(&MessageRef) -> bool,
    {
        let mut offset = 0;
        let mut count = 0;

        while offset < buffer.len() {
            match Self::decode(&buffer[offset..]) {
                Ok((msg, consumed)) => {
                    if !callback(&msg) {
                        break;
                    }
                    offset += consumed;
                    count += 1;
                }
                Err(DecodeError::BufferTooSmall { .. }) => break, // normal end
                Err(e) => return Err(e),
            }
        }

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_add_order_msg(seq: u32) -> Vec<u8> {
        let mut msg = vec![0u8; 46];
        msg[0] = MessageType::AddOrder as u8;
        LittleEndian::write_u16(&mut msg[1..3], 46);
        LittleEndian::write_u32(&mut msg[3..7], seq);
        msg
    }

    #[test]
    fn test_decode_add_order() {
        let msg = create_add_order_msg(42);
        let (decoded, consumed) = Decoder::decode(&msg).unwrap();
        assert_eq!(consumed, 46);
        assert_eq!(decoded.sequence(), 42);
        assert_eq!(decoded.message_type(), MessageType::AddOrder);
    }

    #[test]
    fn test_buffer_too_small() {
        let small_buf = vec![0u8; 4];
        let result = Decoder::decode(&small_buf);
        assert!(matches!(result, Err(DecodeError::BufferTooSmall { .. })));
    }

    #[test]
    fn test_invalid_message_type() {
        let mut msg = vec![0u8; 8];
        msg[0] = 99; // invalid type
        LittleEndian::write_u16(&mut msg[1..3], 8);
        let result = Decoder::decode(&msg);
        assert!(matches!(result, Err(DecodeError::InvalidMessageType(99))));
    }

    #[test]
    fn test_truncated_message() {
        let mut msg = vec![0u8; 8];
        msg[0] = MessageType::AddOrder as u8;
        LittleEndian::write_u16(&mut msg[1..3], 100); // claims 100 bytes
        let result = Decoder::decode(&msg);
        assert!(matches!(result, Err(DecodeError::TruncatedMessage { .. })));
    }
}
