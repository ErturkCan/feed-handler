/// Snapshot-based order book recovery
///
/// Handles full book snapshots to reset state and recover from communication gaps.

use crate::book_builder::OrderBook;
use crate::decoder::MessageRef;

#[derive(Debug, Clone)]
pub struct RecoveryManager {
    last_snapshot_seq: Option<u32>,
    book: OrderBook,
}

impl RecoveryManager {
    pub fn new() -> Self {
        RecoveryManager {
            last_snapshot_seq: None,
            book: OrderBook::new(),
        }
    }

    /// Apply a snapshot to reset the order book
    /// Returns the snapshot sequence number
    pub fn apply_snapshot(&mut self, msg: &MessageRef) -> Result<u32, String> {
        match msg {
            MessageRef::Snapshot(snap) => {
                let seq = snap.sequence();
                self.book.apply_message(msg)?;
                self.last_snapshot_seq = Some(seq);
                Ok(seq)
            }
            _ => Err("Expected snapshot message".to_string()),
        }
    }

    /// Apply an incremental update
    /// Returns error if message sequence is before last snapshot
    pub fn apply_update(&mut self, msg: &MessageRef) -> Result<(), String> {
        let seq = msg.sequence();

        if let Some(last_snap) = self.last_snapshot_seq {
            if seq <= last_snap {
                return Err(format!(
                    "Message sequence {} is before last snapshot {}",
                    seq, last_snap
                ));
            }
        }

        self.book.apply_message(msg)
    }

    /// Get the last snapshot sequence number
    pub fn last_snapshot_sequence(&self) -> Option<u32> {
        self.last_snapshot_seq
    }

    /// Get reference to current order book
    pub fn book(&self) -> &OrderBook {
        &self.book
    }

    /// Get mutable reference to current order book
    pub fn book_mut(&mut self) -> &mut OrderBook {
        &mut self.book
    }

    /// Reset the recovery manager
    pub fn reset(&mut self) {
        self.last_snapshot_seq = None;
        self.book = OrderBook::new();
    }

    /// Check if recovery is needed (no snapshot received yet)
    pub fn needs_recovery(&self) -> bool {
        self.last_snapshot_seq.is_none()
    }
}

impl Default for RecoveryManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_needs_recovery() {
        let recovery = RecoveryManager::new();
        assert!(recovery.needs_recovery());
    }

    #[test]
    fn test_recovery_reset() {
        let mut recovery = RecoveryManager::new();
        recovery.last_snapshot_seq = Some(42);
        assert!(!recovery.needs_recovery());

        recovery.reset();
        assert!(recovery.needs_recovery());
        assert_eq!(recovery.last_snapshot_seq, None);
    }
}
