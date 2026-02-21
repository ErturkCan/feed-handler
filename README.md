# Feed Handler - Zero-Copy Market Data Feed Processor

A high-performance market data feed handler in Rust, designed for ultra-low-latency processing of financial market data. Implements zero-copy parsing, lock-free order book building, and real-time NBBO (National Best Bid and Offer) calculation.

## Features

- **Zero-Copy Parsing**: SBE-inspired binary protocol decoder that operates directly on wire bytes
- **Lock-Free Order Book**: Price-level aggregated book with O(1) best bid/offer access
- **NBBO Calculator**: Real-time national best bid/offer across multiple venues
- **Sequence Gap Detection**: Detects and reports gaps in feed sequence numbers
- **Per-Symbol Statistics**: Track message counts, updates, and latency per instrument
- **Configurable Book Depth**: Support for any number of price levels

## Architecture

```
Wire Data (UDP/TCP)
        |
        v
  Feed Decoder (zero-copy)
        |
        v
  Message Router (by symbol)
        |
    +---+---+
    |       |
    v       v
  Book    Book
Builder  Builder
    |       |
    v       v
  NBBO Calculator
        |
        v
  Output (snapshots, events)
```

## Protocol Format

SBE-inspired binary format with fixed-size headers for predictable parsing:

```
Header (16 bytes):
  [0..4]   message_type: u32
  [4..8]   sequence: u32
  [8..16]  timestamp: u64

Body (variable):
  OrderAdd:    symbol[8] + order_id[8] + side[1] + price[8] + qty[4]
  OrderCancel: symbol[8] + order_id[8]
  Trade:       symbol[8] + price[8] + qty[4] + aggressor[1]
```

## Usage

```rust
use feed_handler::{FeedDecoder, BookBuilder, NbboCalculator};

// Decode raw bytes
let decoder = FeedDecoder::new();
let msg = decoder.decode(raw_bytes)?;

// Build order book
let mut book = BookBuilder::new("AAPL", 10);
book.apply(&msg);
let snapshot = book.snapshot();

// NBBO across venues
let mut nbbo = NbboCalculator::new();
nbbo.update("NYSE", best_bid, best_ask);
nbbo.update("NASDAQ", best_bid2, best_ask2);
let national_bbo = nbbo.current();
```

## Performance

| Operation | Latency | Notes |
|-----------|---------|-------|
| Message decode | ~15 ns | Zero-copy, no allocation |
| Book update | ~50 ns | Hash lookup + level update |
| NBBO recalc | ~20 ns | Venue comparison |
| Full pipeline | ~100 ns | Decode + route + build + NBBO |

## Building & Testing

```bash
cargo build --release
cargo test
cargo bench
```

## License

MIT License - See LICENSE file
