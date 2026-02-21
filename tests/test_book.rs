/// Order book correctness tests

use feed_handler::{OrderBook, Decoder};
use byteorder::{LittleEndian, ByteOrder};

// Helper to create add order messages
fn create_add_order_msg(order_id: u64, price: u64, qty: u32, side: u8, seq: u32) -> Vec<u8> {
    let mut msg = vec![0u8; 46];
    msg[0] = 1; // AddOrder type
    LittleEndian::write_u16(&mut msg[1..3], 46);
    LittleEndian::write_u32(&mut msg[3..7], seq);

    // order_id at offset 8
    LittleEndian::write_u64(&mut msg[8..16], order_id);
    // price at offset 16
    LittleEndian::write_u64(&mut msg[16..24], price);
    // quantity at offset 24
    LittleEndian::write_u32(&mut msg[24..28], qty);
    // side at offset 28
    msg[28] = side;

    msg
}

#[test]
fn test_empty_book() {
    let book = OrderBook::new();
    assert_eq!(book.best_bid(), None);
    assert_eq!(book.best_ask(), None);
    assert_eq!(book.spread(), None);
    assert_eq!(book.order_count(), 0);
}

#[test]
fn test_add_single_bid() {
    let mut book = OrderBook::new();
    let price = 100_00000000u64; // 100.00
    let qty = 100u32;

    let msg_bytes = create_add_order_msg(1, price, qty, 0, 1);
    let (msg, _) = Decoder::decode(&msg_bytes).unwrap();
    book.apply_message(&msg).unwrap();

    assert_eq!(book.best_bid(), Some((price, qty)));
    assert_eq!(book.best_ask(), None);
    assert_eq!(book.order_count(), 1);
}

#[test]
fn test_add_bid_and_ask() {
    let mut book = OrderBook::new();
    let bid_price = 100_00000000u64;
    let ask_price = 101_00000000u64;
    let qty = 100u32;

    // Add bid
    let bid_msg_bytes = create_add_order_msg(1, bid_price, qty, 0, 1);
    let (bid_msg, _) = Decoder::decode(&bid_msg_bytes).unwrap();
    book.apply_message(&bid_msg).unwrap();

    // Add ask
    let ask_msg_bytes = create_add_order_msg(2, ask_price, qty, 1, 2);
    let (ask_msg, _) = Decoder::decode(&ask_msg_bytes).unwrap();
    book.apply_message(&ask_msg).unwrap();

    assert_eq!(book.best_bid(), Some((bid_price, qty)));
    assert_eq!(book.best_ask(), Some((ask_price, qty)));
    assert_eq!(book.spread(), Some(ask_price - bid_price));
    assert_eq!(book.order_count(), 2);
}

#[test]
fn test_multiple_bid_levels() {
    let mut book = OrderBook::new();

    // Add bids at different prices
    for i in 0..5 {
        let price = 100_00000000u64 - (i as u64 * 10_00000000); // 100, 99, 98, 97, 96
        let qty = 100 + i as u32;
        let msg_bytes = create_add_order_msg(i as u64, price, qty, 0, (i + 1) as u32);
        let (msg, _) = Decoder::decode(&msg_bytes).unwrap();
        book.apply_message(&msg).unwrap();
    }

    let best = book.best_bid();
    assert!(best.is_some());
    assert_eq!(best.unwrap().0, 100_00000000u64); // Best bid is highest
}

#[test]
fn test_multiple_ask_levels() {
    let mut book = OrderBook::new();

    // Add asks at different prices
    for i in 0..5 {
        let price = 100_00000000u64 + (i as u64 * 10_00000000); // 100, 101, 102, 103, 104
        let qty = 100 + i as u32;
        let msg_bytes = create_add_order_msg(i as u64 + 100, price, qty, 1, (i + 1) as u32);
        let (msg, _) = Decoder::decode(&msg_bytes).unwrap();
        book.apply_message(&msg).unwrap();
    }

    let best = book.best_ask();
    assert!(best.is_some());
    assert_eq!(best.unwrap().0, 100_00000000u64); // Best ask is lowest
}

#[test]
fn test_spread() {
    let mut book = OrderBook::new();
    let bid = 100_00000000u64;
    let ask = 101_50000000u64;

    let bid_msg_bytes = create_add_order_msg(1, bid, 100, 0, 1);
    let (bid_msg, _) = Decoder::decode(&bid_msg_bytes).unwrap();
    book.apply_message(&bid_msg).unwrap();

    let ask_msg_bytes = create_add_order_msg(2, ask, 100, 1, 2);
    let (ask_msg, _) = Decoder::decode(&ask_msg_bytes).unwrap();
    book.apply_message(&ask_msg).unwrap();

    let spread = book.spread().unwrap();
    assert_eq!(spread, ask - bid);
}

#[test]
fn test_depth() {
    let mut book = OrderBook::new();

    // Add 10 bid levels
    for i in 0..10 {
        let price = 100_00000000u64 - (i as u64 * 1_00000000);
        let msg_bytes = create_add_order_msg(i as u64 + 1000, price, 100 + i as u32, 0, (i + 1) as u32);
        let (msg, _) = Decoder::decode(&msg_bytes).unwrap();
        book.apply_message(&msg).unwrap();
    }

    // Add 10 ask levels
    for i in 0..10 {
        let price = 100_00000000u64 + (i as u64 * 1_00000000);
        let msg_bytes = create_add_order_msg(i as u64 + 2000, price, 100 + i as u32, 1, (i + 11) as u32);
        let (msg, _) = Decoder::decode(&msg_bytes).unwrap();
        book.apply_message(&msg).unwrap();
    }

    let depth = book.depth(5);
    assert_eq!(depth.bids.len(), 5);
    assert_eq!(depth.asks.len(), 5);

    // Check bid levels are in descending order
    for i in 1..depth.bids.len() {
        assert!(depth.bids[i - 1].0 > depth.bids[i].0);
    }

    // Check ask levels are in ascending order
    for i in 1..depth.asks.len() {
        assert!(depth.asks[i - 1].0 < depth.asks[i].0);
    }
}

#[test]
fn test_depth_request_more_than_available() {
    let mut book = OrderBook::new();

    // Add only 3 bid levels
    for i in 0..3 {
        let price = 100_00000000u64 - (i as u64 * 1_00000000);
        let msg_bytes = create_add_order_msg(i as u64, price, 100, 0, (i + 1) as u32);
        let (msg, _) = Decoder::decode(&msg_bytes).unwrap();
        book.apply_message(&msg).unwrap();
    }

    // Request 10 levels but only have 3
    let depth = book.depth(10);
    assert_eq!(depth.bids.len(), 3);
    assert_eq!(depth.asks.len(), 0);
}

#[test]
fn test_bid_levels_count() {
    let mut book = OrderBook::new();

    for i in 0..5 {
        let price = 100_00000000u64 - (i as u64 * 1_00000000);
        let msg_bytes = create_add_order_msg(i as u64, price, 100, 0, (i + 1) as u32);
        let (msg, _) = Decoder::decode(&msg_bytes).unwrap();
        book.apply_message(&msg).unwrap();
    }

    assert_eq!(book.bid_levels(), 5);
}

#[test]
fn test_ask_levels_count() {
    let mut book = OrderBook::new();

    for i in 0..7 {
        let price = 100_00000000u64 + (i as u64 * 1_00000000);
        let msg_bytes = create_add_order_msg(i as u64, price, 100, 1, (i + 1) as u32);
        let (msg, _) = Decoder::decode(&msg_bytes).unwrap();
        book.apply_message(&msg).unwrap();
    }

    assert_eq!(book.ask_levels(), 7);
}

#[test]
fn test_same_price_multiple_orders() {
    let mut book = OrderBook::new();
    let price = 100_00000000u64;

    // Add multiple orders at same price
    for i in 0..5 {
        let msg_bytes = create_add_order_msg(i as u64, price, 100, 0, (i + 1) as u32);
        let (msg, _) = Decoder::decode(&msg_bytes).unwrap();
        book.apply_message(&msg).unwrap();
    }

    assert_eq!(book.best_bid(), Some((price, 500)));
    assert_eq!(book.bid_levels(), 1);
    assert_eq!(book.order_count(), 5);
}

#[test]
fn test_inverted_spread() {
    let mut book = OrderBook::new();

    // Inverted spread (ask < bid) - market crossed
    let bid = 101_00000000u64;
    let ask = 100_00000000u64;

    let bid_msg_bytes = create_add_order_msg(1, bid, 100, 0, 1);
    let (bid_msg, _) = Decoder::decode(&bid_msg_bytes).unwrap();
    book.apply_message(&bid_msg).unwrap();

    let ask_msg_bytes = create_add_order_msg(2, ask, 100, 1, 2);
    let (ask_msg, _) = Decoder::decode(&ask_msg_bytes).unwrap();
    book.apply_message(&ask_msg).unwrap();

    // When crossed, spread should be None
    assert_eq!(book.spread(), None);
}
