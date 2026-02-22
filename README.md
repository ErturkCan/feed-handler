Feed Handler - Market Data Feed Processor
===========================================

A high-performance, zero-copy market data feed processor designed for quantitative trading applications. Written in Rust with reliability and performance.

## Features

- **Binary Protocol**: SBE-inspired fixed-size message format with zero-copy decoding
- **Zero-Copy Parsing**: Decoder returns references directly into input buffer - no allocations during decode
- **Order Book Management**: Efficient bid/ask price level tracking using BTreeMap
- **Sequence Tracking**: Built-in gap detection for lost message recovery
- **Snapshot Recovery**: Full book snapshots for recovery after communication gaps
- **Performance Metrics**: Comprehensive statistics including latency percentiles
- **Type-Safe**: Full type safety with compile-time assertions for memory layout

## Binary Protocol

All messages use a fixed 8-byte header followed by message-specific payloads:

```
Header (8 bytes):
  [msg_type:u8][length:u16][sequence:u32][padding:u8]

Message Types:
  1 = AddOrder    (46 bytes total)
  2 = ModifyOrder (26 bytes total)
  3 = DeleteOrder (16 bytes total)
  4 = Trade       (38 bytes total)
  5 = Snapshot    (variable length)
```

### AddOrder
```
Offset  Field         Type    Notes
0       msg_type      u8      1
1-2     length        u16     46
3-6     sequence      u32     Monotonically increasing
7       padding       u8
8-15    order_id      u64
16-23   price         u64     Fixed-point: price * 10^8
24-27   quantity      u32
28      side          u8      0=bid, 1=ask
29-45   padding       u8[17]
```

### ModifyOrder
```
Offset  Field         Type
0       msg_type      u8      2
1-2     length        u16     26
3-6     sequence      u32
7       padding       u8
8-15    order_id      u64
16-19   new_quantity  u32
20-25   padding       u8[2]
```

### DeleteOrder
```
Offset  Field         Type
0       msg_type      u8      3
1-2     length        u16     16
3-6     sequence      u32
7       padding       u8
8-15    order_id      u64
```

### Trade
```
Offset  Field         Type
0       msg_type      u8      4
1-2     length        u16     38
3-6     sequence      u32
7       padding       u8
8-15    buyer_order_id    u64
16-23   seller_order_id   u64
24-31   price         u64     Fixed-point: price * 10^8
32-35   quantity      u32
36-37   padding       u8[2]
```

### Snapshot
```
Header  (8 bytes)
num_bids  (4 bytes)
num_asks  (4 bytes)
bid_levels    (num_bids * 16 bytes)
ask_levels    (num_asks * 16 bytes)

Each level: [price:u64][quantity:u32][padding:u8[4]]
```

## Zero-Copy Design

The decoder achieves zero allocations by using unsafe pointer casting:

```rust
// Instead of copying the message, cast the buffer pointer
let ptr = buffer.as_ptr() as *const AddOrder;
let msg_ref = unsafe { &*ptr };
```

This enables microsecond-latency message parsing. Message references are tied to the lifetime of the input buffer.

## Architecture

```
┌─────────────────────────────────────┐
│   Binary Feed (bytes)               │
└────────────┬────────────────────────┘
             │
             v
┌─────────────────────────────────────┐
│   Decoder (zero-copy parsing)       │
│   → MessageRef<'a>                  │
└────────────┬────────────────────────┘
             │
      ┌──────┴──────┐
      │             │
      v             v
  GapDetector   OrderBook
 - Sequence   - Bid levels
    tracking   - Ask levels
 - Gap ranges - Order tracking
      │             │
      └──────┬──────┘
             v
   RecoveryManager
  - Snapshot state
  - Recovery logic
             │
             v
        FeedStats
       - Throughput
       - Latencies
       - Gap counts
```

## Usage Examples

### Basic Decoding

```rust
use feed_handler::Decoder;

let buffer = /* binary data */;
match Decoder::decode(&buffer) {
    Ok((msg, consumed)) => {
        println!("Decoded message: {:?}", msg.message_type());
        println!("Sequence: {}", msg.sequence());
    }
    Err(e) => eprintln!("Decode error: {}", e),
}
```

### Order Book Management

```rust
use feed_handler::{OrderBook, Decoder};

let mut book = OrderBook::new();
let buffer = /* feed data */;

let mut offset = 0;
while offset < buffer.len() {
    if let Ok((msg, consumed)) = Decoder::decode(&buffer[offset..]) {
        // Apply message to book
        let _ = book.apply_message(&msg);

        // Get market data
        if let (Some((bid, bid_qty)), Some((ask, ask_qty))) =
            (book.best_bid(), book.best_ask()) {
            println!("Spread: {} - {}", bid, ask);
        }

        offset += consumed;
    } else {
        break;
    }
}
```

### Gap Detection

```rust
use feed_handler::{Decoder, GapDetector};

let mut detector = GapDetector::new();

Decoder::decode_stream(&buffer, |msg| {
    detector.process(msg.sequence());
    true
})?;

println!("Gaps: {:?}", detector.gaps());
println!("Total missing: {}", detector.total_gaps());
```

### Recovery from Snapshots

```rust
use feed_handler::{Decoder, RecoveryManager};

let mut recovery = RecoveryManager::new();

Decoder::decode_stream(&buffer, |msg| {
    match msg.message_type() {
        MessageType::Snapshot => {
            let _ = recovery.apply_snapshot(msg);
        }
        _ if !recovery.needs_recovery() => {
            let _ = recovery.apply_update(msg);
        }
        _ => {
            // Waiting for snapshot
        }
    }
    true
})?;

let book = recovery.book();
println!("Best bid: {:?}", book.best_bid());
```

## Performance Targets

- **Decode throughput**: > 1M messages/sec
- **Decode latency p50**: < 1 µs
- **Decode latency p99**: < 10 µs
- **Book update latency**: < 5 µs
- **Zero allocations during decode**

## Running Benchmarks

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark
cargo bench decode_throughput

# Generate HTML reports (in target/criterion/)
cargo bench --bench decode -- --output-format bencher
```

## Running Tests

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_decode_add_order
```

## Feed Generator

Generate synthetic market data for testing:

```bash
# Generate 100,000 messages to file
cargo run --example feed_generator -- /tmp/feed.bin 100000

# Generate to stdout (for piping)
cargo run --example feed_generator -- stdout 10000 | head -c 1000 | hexdump -C
```

## Design Principles

1. **Zero-Copy**: No allocations during message decoding
2. **Type Safety**: Compile-time assertions for struct layout
3. **Bounds Checking**: Defensive validation of all inputs
4. **Performance**: Microsecond-scale latencies
5. **Simplicity**: No external dependencies beyond byteorder and thiserror
6. **Testability**: Comprehensive test coverage and benchmarks

## Memory Layout

Compile-time assertions verify struct sizes and alignment:

```rust
assert!(mem::size_of::<MessageHeader>() == 8);
assert!(mem::size_of::<AddOrder>() == 46);
assert!(mem::size_of::<ModifyOrder>() == 26);
assert!(mem::size_of::<DeleteOrder>() == 16);
assert!(mem::size_of::<Trade>() == 38);
```

All message types use `repr(C, packed)` to guarantee exact binary layout.

## Safety Considerations

- All unsafe code is used only for pointer casting from validated buffers
- Buffer bounds checking happens before any pointer operations
- Message sequence numbers are validated before processing
- Decode errors are comprehensive and actionable

## Project Structure

```
feed-handler/
├── Cargo.toml
├── src/
│   ├── lib.rs           # Library re-exports
│   ├── protocol.rs      # Message format definitions
│   ├── decoder.rs       # Zero-copy parser
│   ├── book_builder.rs  # Order book state
│   ├── gap_detector.rs  # Sequence tracking
│   ├── recovery.rs      # Snapshot recovery
│   └── stats.rs         # Performance metrics
├── tests/
│   ├── test_decoder.rs  # Protocol conformance
│   └── test_book.rs     # Book correctness
├── benches/
│   ├── decode.rs        # Decode benchmarks
│   └── book_update.rs   # Book update benchmarks
├── examples/
│   └── feed_generator.rs # Synthetic data generator
└── README.md
```

## License

This project is provided as a portfolio example for quantitative trading roles.
