/// Binary message format inspired by SBE (Simple Binary Encoding)
///
/// Fixed header: 8 bytes
///   - msg_type: u8 (1 byte)
///   - length: u16 (2 bytes) - total message length including header
///   - sequence: u32 (4 bytes) - monotonically increasing sequence number
///   - padding: u8 (1 byte)

use std::mem;

pub const HEADER_SIZE: usize = 8;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    AddOrder = 1,
    ModifyOrder = 2,
    DeleteOrder = 3,
    Trade = 4,
    Snapshot = 5,
}

impl MessageType {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            1 => Some(MessageType::AddOrder),
            2 => Some(MessageType::ModifyOrder),
            3 => Some(MessageType::DeleteOrder),
            4 => Some(MessageType::Trade),
            5 => Some(MessageType::Snapshot),
            _ => None,
        }
    }
}

/// Message header: 8 bytes total
/// Laid out as: [msg_type(1)][length(2)][sequence(4)][padding(1)]
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct MessageHeader {
    pub msg_type: u8,
    pub length: u16,    // little-endian
    pub sequence: u32,  // little-endian
    pub padding: u8,
}

/// Add a new order to the book
/// Total: 8 (header) + 38 = 46 bytes
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct AddOrder {
    pub header: MessageHeader,
    pub order_id: u64,        // 8 bytes
    pub price: u64,           // fixed-point: price * 10^8
    pub quantity: u32,        // 4 bytes
    pub side: u8,             // 0 = bid, 1 = ask (1 byte)
    pub _padding: [u8; 13],   // 13 bytes padding to align
}

/// Modify an existing order
/// Total: 8 (header) + 18 = 26 bytes
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct ModifyOrder {
    pub header: MessageHeader,
    pub order_id: u64,        // 8 bytes
    pub new_quantity: u32,    // 4 bytes
    pub _padding: [u8; 2],    // 2 bytes padding
}

/// Delete an existing order
/// Total: 8 (header) + 8 = 16 bytes
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct DeleteOrder {
    pub header: MessageHeader,
    pub order_id: u64,        // 8 bytes
}

/// Trade execution
/// Total: 8 (header) + 22 = 30 bytes
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Trade {
    pub header: MessageHeader,
    pub buyer_order_id: u64,  // 8 bytes
    pub seller_order_id: u64, // 8 bytes
    pub price: u64,           // fixed-point: price * 10^8 (8 bytes)
    pub quantity: u32,        // 4 bytes
    pub _padding: [u8; 2],    // 2 bytes padding
}

/// Full order book snapshot (variable length)
/// Total: 8 (header) + 4 + (bid_count + ask_count) * 16
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct SnapshotHeader {
    pub header: MessageHeader,
    pub num_bids: u32,        // 4 bytes
    pub num_asks: u32,        // 4 bytes
}

/// Single level in snapshot: price, quantity pair
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct SnapshotLevel {
    pub price: u64,           // fixed-point: price * 10^8
    pub quantity: u32,        // 4 bytes
    pub _padding: [u8; 4],    // 4 bytes padding
}

// Compile-time assertions for memory layout
const _: () = {
    const fn assert_size<const N: usize>() {}
    const fn check() {
        assert!(mem::size_of::<MessageHeader>() == 8);
        assert!(mem::size_of::<AddOrder>() == 46);
        assert!(mem::size_of::<ModifyOrder>() == 26);
        assert!(mem::size_of::<DeleteOrder>() == 16);
        assert!(mem::size_of::<Trade>() == 30);
        assert!(mem::size_of::<SnapshotHeader>() == 16);
        assert!(mem::size_of::<SnapshotLevel>() == 16);
    }
};

/// Convert price from fixed-point to float
pub fn price_from_fixed(fixed: u64) -> f64 {
    fixed as f64 / 1e8
}

/// Convert price to fixed-point
pub fn price_to_fixed(price: f64) -> u64 {
    (price * 1e8) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_type_conversion() {
        assert_eq!(MessageType::from_u8(1), Some(MessageType::AddOrder));
        assert_eq!(MessageType::from_u8(5), Some(MessageType::Snapshot));
        assert_eq!(MessageType::from_u8(99), None);
    }

    #[test]
    fn test_price_conversions() {
        let price = 123.456;
        let fixed = price_to_fixed(price);
        let back = price_from_fixed(fixed);
        assert!((back - price).abs() < 1e-6);
    }
}
