/// Feed Handler - Market Data Feed Processor
///
/// High-performance, zero-copy market data feed processor designed for quantitative
/// trading applications. Features include:
/// - Binary protocol parsing (SBE-inspired)
/// - Zero-copy message decoding
/// - Order book state management
/// - Sequence number gap detection
/// - Snapshot-based recovery
/// - Comprehensive performance statistics

pub mod protocol;
pub mod decoder;
pub mod book_builder;
pub mod gap_detector;
pub mod recovery;
pub mod stats;

pub use protocol::{MessageType, AddOrder, ModifyOrder, DeleteOrder, Trade, SnapshotHeader, SnapshotLevel};
pub use decoder::{Decoder, DecodeError, MessageRef, SnapshotRef};
pub use book_builder::{OrderBook, Order, Side, BookDepth};
pub use gap_detector::GapDetector;
pub use recovery::RecoveryManager;
pub use stats::{FeedStats, LatencyStats};
