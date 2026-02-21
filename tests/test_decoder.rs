/// Protocol conformance and decoder tests

use feed_handler::{Decoder, MessageType, DecodeError};
use byteorder::{LittleEndian, ByteOrder};

fn create_message(msg_type: MessageType, seq: u32, payload_size: usize) -> Vec<u8> {
    let total_size = 8 + payload_size; // 8 byte header
    let mut msg = vec![0u8; total_size];
    msg[0] = msg_type as u8;
    LittleEndian::write_u16(&mut msg[1..3], total_size as u16);
    LittleEndian::write_u32(&mut msg[3..7], seq);
    msg
}

#[test]
fn test_decode_add_order() {
    let msg = create_message(MessageType::AddOrder, 42, 38);
    let (decoded, consumed) = Decoder::decode(&msg).unwrap();

    assert_eq!(consumed, 46);
    assert_eq!(decoded.sequence(), 42);
    assert_eq!(decoded.message_type(), MessageType::AddOrder);
}

#[test]
fn test_decode_modify_order() {
    let msg = create_message(MessageType::ModifyOrder, 10, 18);
    let (decoded, consumed) = Decoder::decode(&msg).unwrap();

    assert_eq!(consumed, 26);
    assert_eq!(decoded.sequence(), 10);
    assert_eq!(decoded.message_type(), MessageType::ModifyOrder);
}

#[test]
fn test_decode_delete_order() {
    let msg = create_message(MessageType::DeleteOrder, 20, 8);
    let (decoded, consumed) = Decoder::decode(&msg).unwrap();

    assert_eq!(consumed, 16);
    assert_eq!(decoded.sequence(), 20);
    assert_eq!(decoded.message_type(), MessageType::DeleteOrder);
}

#[test]
fn test_decode_trade() {
    let msg = create_message(MessageType::Trade, 30, 30);
    let (decoded, consumed) = Decoder::decode(&msg).unwrap();

    assert_eq!(consumed, 38);
    assert_eq!(decoded.sequence(), 30);
    assert_eq!(decoded.message_type(), MessageType::Trade);
}

#[test]
fn test_buffer_too_small_header() {
    let small = vec![0u8; 4];
    let result = Decoder::decode(&small);
    assert!(matches!(result, Err(DecodeError::BufferTooSmall { .. })));
}

#[test]
fn test_buffer_too_small_payload() {
    let mut msg = vec![0u8; 8];
    msg[0] = MessageType::AddOrder as u8;
    LittleEndian::write_u16(&mut msg[1..3], 50); // claims 50 bytes
    let result = Decoder::decode(&msg);
    assert!(matches!(result, Err(DecodeError::TruncatedMessage { .. })));
}

#[test]
fn test_invalid_message_type() {
    let mut msg = vec![0u8; 46];
    msg[0] = 99; // invalid type
    LittleEndian::write_u16(&mut msg[1..3], 46);
    let result = Decoder::decode(&msg);
    assert!(matches!(result, Err(DecodeError::InvalidMessageType(99))));
}

#[test]
fn test_zero_length() {
    let mut msg = vec![0u8; 8];
    msg[0] = MessageType::AddOrder as u8;
    LittleEndian::write_u16(&mut msg[1..3], 4); // less than header size
    let result = Decoder::decode(&msg);
    assert!(result.is_err());
}

#[test]
fn test_decode_snapshot_empty() {
    let mut msg = vec![0u8; 16]; // header + 4 + 4 for num_bids/asks
    msg[0] = MessageType::Snapshot as u8;
    LittleEndian::write_u16(&mut msg[1..3], 16);
    LittleEndian::write_u32(&mut msg[3..7], 100);
    LittleEndian::write_u32(&mut msg[8..12], 0); // num_bids
    LittleEndian::write_u32(&mut msg[12..16], 0); // num_asks

    let (decoded, consumed) = Decoder::decode(&msg).unwrap();
    assert_eq!(consumed, 16);
    assert_eq!(decoded.sequence(), 100);
    assert_eq!(decoded.message_type(), MessageType::Snapshot);
}

#[test]
fn test_decode_snapshot_with_levels() {
    // Header (8) + num_bids/asks (8) + 2 bid levels (32) + 2 ask levels (32) = 80 bytes
    let num_bids = 2u32;
    let num_asks = 2u32;
    let total_size = 8 + 8 + (num_bids + num_asks) as usize * 16;

    let mut msg = vec![0u8; total_size];
    msg[0] = MessageType::Snapshot as u8;
    LittleEndian::write_u16(&mut msg[1..3], total_size as u16);
    LittleEndian::write_u32(&mut msg[3..7], 200);
    LittleEndian::write_u32(&mut msg[8..12], num_bids);
    LittleEndian::write_u32(&mut msg[12..16], num_asks);

    // Add bid levels
    for i in 0..num_bids as usize {
        let offset = 16 + i * 16;
        LittleEndian::write_u64(&mut msg[offset..offset + 8], 1000000000 - (i as u64) * 100000000); // prices
        LittleEndian::write_u32(&mut msg[offset + 8..offset + 12], 100 + i as u32); // quantities
    }

    // Add ask levels
    for i in 0..num_asks as usize {
        let offset = 16 + (num_bids as usize) * 16 + i * 16;
        LittleEndian::write_u64(&mut msg[offset..offset + 8], 1000000000 + (i as u64) * 100000000); // prices
        LittleEndian::write_u32(&mut msg[offset + 8..offset + 12], 100 + i as u32); // quantities
    }

    let (decoded, consumed) = Decoder::decode(&msg).unwrap();
    assert_eq!(consumed, total_size);
    assert_eq!(decoded.sequence(), 200);
    assert_eq!(decoded.message_type(), MessageType::Snapshot);

    if let feed_handler::MessageRef::Snapshot(snap) = decoded {
        assert_eq!(snap.bid_levels.len(), 2);
        assert_eq!(snap.ask_levels.len(), 2);
    } else {
        panic!("Expected snapshot");
    }
}

#[test]
fn test_decode_snapshot_truncated() {
    let mut msg = vec![0u8; 16];
    msg[0] = MessageType::Snapshot as u8;
    LittleEndian::write_u16(&mut msg[1..3], 80); // claims 80 bytes
    LittleEndian::write_u32(&mut msg[8..12], 2); // num_bids
    LittleEndian::write_u32(&mut msg[12..16], 2); // num_asks
    // but only have 16 bytes total

    let result = Decoder::decode(&msg);
    assert!(matches!(result, Err(DecodeError::TruncatedMessage { .. })));
}

#[test]
fn test_decode_stream() {
    let msg1 = create_message(MessageType::AddOrder, 1, 38);
    let msg2 = create_message(MessageType::ModifyOrder, 2, 18);

    let mut buffer = msg1.clone();
    buffer.extend_from_slice(&msg2);

    let mut count = 0;
    let mut last_seq = 0;

    let result = Decoder::decode_stream(&buffer, |msg| {
        count += 1;
        last_seq = msg.sequence();
        true
    });

    assert!(result.is_ok());
    assert_eq!(count, 2);
    assert_eq!(last_seq, 2);
}

#[test]
fn test_decode_stream_stops_on_callback_false() {
    let msg1 = create_message(MessageType::AddOrder, 1, 38);
    let msg2 = create_message(MessageType::ModifyOrder, 2, 18);
    let msg3 = create_message(MessageType::DeleteOrder, 3, 8);

    let mut buffer = msg1.clone();
    buffer.extend_from_slice(&msg2);
    buffer.extend_from_slice(&msg3);

    let mut count = 0;

    let result = Decoder::decode_stream(&buffer, |_msg| {
        count += 1;
        count < 2
    });

    assert!(result.is_ok());
    assert_eq!(count, 2);
}

#[test]
fn test_message_type_boundary() {
    for msg_type in 1..=5 {
        let mut msg = create_message(MessageType::AddOrder, 0, 38);
        msg[0] = msg_type;
        let result = Decoder::decode(&msg);
        assert!(result.is_ok(), "Message type {} should be valid", msg_type);
    }
}

#[test]
fn test_alignment_boundaries() {
    // Test with various payload sizes to ensure alignment is handled
    for size in 0..100 {
        let msg = create_message(MessageType::AddOrder, 0, size);
        if msg.len() >= 46 {
            // Only test valid sizes
            let result = Decoder::decode(&msg);
            match result {
                Ok((decoded, _)) => {
                    assert_eq!(decoded.message_type(), MessageType::AddOrder);
                }
                Err(e) => {
                    // Expected for too-small messages
                    assert!(matches!(e, DecodeError::BufferTooSmall { .. }));
                }
            }
        }
    }
}
