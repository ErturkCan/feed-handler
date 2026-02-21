/// Feed statistics tracking
///
/// Tracks metrics like messages/sec, decode latency, book update latency, gaps.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

const WINDOW_SIZE: usize = 10000;

#[derive(Debug, Clone, Copy)]
pub struct LatencyStats {
    pub min_us: u64,
    pub max_us: u64,
    pub mean_us: f64,
    pub p50_us: u64,
    pub p99_us: u64,
}

#[derive(Debug, Clone)]
pub struct FeedStats {
    // Timing
    start_time: Option<Instant>,
    total_messages: u64,
    total_bytes: u64,

    // Decode latencies (in microseconds)
    decode_latencies: VecDeque<u64>,

    // Book update latencies
    book_update_latencies: VecDeque<u64>,

    // Gap tracking
    total_gaps: u32,
    gap_events: u32,
}

impl FeedStats {
    pub fn new() -> Self {
        FeedStats {
            start_time: None,
            total_messages: 0,
            total_bytes: 0,
            decode_latencies: VecDeque::with_capacity(WINDOW_SIZE),
            book_update_latencies: VecDeque::with_capacity(WINDOW_SIZE),
            total_gaps: 0,
            gap_events: 0,
        }
    }

    /// Record a message received
    pub fn record_message(&mut self, size: usize) {
        if self.start_time.is_none() {
            self.start_time = Some(Instant::now());
        }
        self.total_messages += 1;
        self.total_bytes += size as u64;
    }

    /// Record decode latency in microseconds
    pub fn record_decode_latency(&mut self, micros: u64) {
        if self.decode_latencies.len() >= WINDOW_SIZE {
            self.decode_latencies.pop_front();
        }
        self.decode_latencies.push_back(micros);
    }

    /// Record book update latency in microseconds
    pub fn record_book_update_latency(&mut self, micros: u64) {
        if self.book_update_latencies.len() >= WINDOW_SIZE {
            self.book_update_latencies.pop_front();
        }
        self.book_update_latencies.push_back(micros);
    }

    /// Record a gap event
    pub fn record_gap(&mut self, gap_size: u32) {
        self.total_gaps = self.total_gaps.wrapping_add(gap_size);
        self.gap_events += 1;
    }

    /// Get messages per second
    pub fn messages_per_sec(&self) -> f64 {
        match self.start_time {
            None => 0.0,
            Some(start) => {
                let elapsed = start.elapsed().as_secs_f64();
                if elapsed > 0.0 {
                    self.total_messages as f64 / elapsed
                } else {
                    0.0
                }
            }
        }
    }

    /// Get bytes per second
    pub fn bytes_per_sec(&self) -> f64 {
        match self.start_time {
            None => 0.0,
            Some(start) => {
                let elapsed = start.elapsed().as_secs_f64();
                if elapsed > 0.0 {
                    self.total_bytes as f64 / elapsed
                } else {
                    0.0
                }
            }
        }
    }

    /// Get decode latency statistics
    pub fn decode_latency_stats(&self) -> Option<LatencyStats> {
        if self.decode_latencies.is_empty() {
            return None;
        }

        let mut sorted: Vec<u64> = self.decode_latencies.iter().copied().collect();
        sorted.sort_unstable();

        let min = sorted[0];
        let max = sorted[sorted.len() - 1];
        let mean = sorted.iter().sum::<u64>() as f64 / sorted.len() as f64;
        let p50 = sorted[sorted.len() / 2];
        let p99 = sorted[(sorted.len() * 99) / 100];

        Some(LatencyStats {
            min_us: min,
            max_us: max,
            mean_us: mean,
            p50_us: p50,
            p99_us: p99,
        })
    }

    /// Get book update latency statistics
    pub fn book_update_latency_stats(&self) -> Option<LatencyStats> {
        if self.book_update_latencies.is_empty() {
            return None;
        }

        let mut sorted: Vec<u64> = self.book_update_latencies.iter().copied().collect();
        sorted.sort_unstable();

        let min = sorted[0];
        let max = sorted[sorted.len() - 1];
        let mean = sorted.iter().sum::<u64>() as f64 / sorted.len() as f64;
        let p50 = sorted[sorted.len() / 2];
        let p99 = sorted[(sorted.len() * 99) / 100];

        Some(LatencyStats {
            min_us: min,
            max_us: max,
            mean_us: mean,
            p50_us: p50,
            p99_us: p99,
        })
    }

    /// Get total elapsed time
    pub fn elapsed(&self) -> Option<Duration> {
        self.start_time.map(|st| st.elapsed())
    }

    /// Get total messages processed
    pub fn total_messages(&self) -> u64 {
        self.total_messages
    }

    /// Get total bytes processed
    pub fn total_bytes(&self) -> u64 {
        self.total_bytes
    }

    /// Get total gap count
    pub fn total_gaps(&self) -> u32 {
        self.total_gaps
    }

    /// Get number of gap events
    pub fn gap_events(&self) -> u32 {
        self.gap_events
    }

    /// Reset all statistics
    pub fn reset(&mut self) {
        self.start_time = None;
        self.total_messages = 0;
        self.total_bytes = 0;
        self.decode_latencies.clear();
        self.book_update_latencies.clear();
        self.total_gaps = 0;
        self.gap_events = 0;
    }

    /// Print statistics summary
    pub fn print_summary(&self) {
        println!("=== Feed Statistics ===");
        println!("Total Messages: {}", self.total_messages);
        println!("Total Bytes: {}", self.total_bytes);
        println!("Elapsed: {:?}", self.elapsed());
        println!("Messages/sec: {:.2}", self.messages_per_sec());
        println!("Bytes/sec: {:.2}", self.bytes_per_sec());

        if let Some(stats) = self.decode_latency_stats() {
            println!("\nDecode Latency (us):");
            println!("  Min: {}, Max: {}, Mean: {:.2}", stats.min_us, stats.max_us, stats.mean_us);
            println!("  P50: {}, P99: {}", stats.p50_us, stats.p99_us);
        }

        if let Some(stats) = self.book_update_latency_stats() {
            println!("\nBook Update Latency (us):");
            println!("  Min: {}, Max: {}, Mean: {:.2}", stats.min_us, stats.max_us, stats.mean_us);
            println!("  P50: {}, P99: {}", stats.p50_us, stats.p99_us);
        }

        println!("\nGaps: {} total, {} events", self.total_gaps, self.gap_events);
    }
}

impl Default for FeedStats {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_message() {
        let mut stats = FeedStats::new();
        stats.record_message(46);
        assert_eq!(stats.total_messages(), 1);
        assert_eq!(stats.total_bytes(), 46);
    }

    #[test]
    fn test_decode_latency_stats() {
        let mut stats = FeedStats::new();
        for i in 1..=100 {
            stats.record_decode_latency(i);
        }

        let latency_stats = stats.decode_latency_stats().unwrap();
        assert_eq!(latency_stats.min_us, 1);
        assert_eq!(latency_stats.max_us, 100);
    }

    #[test]
    fn test_gaps() {
        let mut stats = FeedStats::new();
        stats.record_gap(5);
        stats.record_gap(3);
        assert_eq!(stats.total_gaps(), 8);
        assert_eq!(stats.gap_events(), 2);
    }
}
