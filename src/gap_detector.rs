/// Sequence number gap detection
///
/// Tracks incoming sequence numbers and detects gaps indicating lost messages.

#[derive(Debug, Clone)]
pub struct GapDetector {
    last_sequence: Option<u32>,
    gaps: Vec<(u32, u32)>, // Vec of (start, end) ranges
    total_gap_count: u32,
}

impl GapDetector {
    pub fn new() -> Self {
        GapDetector {
            last_sequence: None,
            gaps: Vec::new(),
            total_gap_count: 0,
        }
    }

    /// Process a sequence number; detects and records any gaps
    pub fn process(&mut self, seq_num: u32) {
        match self.last_sequence {
            None => {
                self.last_sequence = Some(seq_num);
            }
            Some(last) => {
                let expected_next = last.wrapping_add(1);
                if seq_num != expected_next {
                    // There's a gap
                    let gap_size = seq_num.wrapping_sub(expected_next);
                    self.gaps.push((expected_next, seq_num.wrapping_sub(1)));
                    self.total_gap_count = self.total_gap_count.wrapping_add(gap_size);
                }
                self.last_sequence = Some(seq_num);
            }
        }
    }

    /// Get all detected gaps as (start, end) tuples (inclusive)
    pub fn gaps(&self) -> &[(u32, u32)] {
        &self.gaps
    }

    /// Get total number of missing sequence numbers
    pub fn total_gaps(&self) -> u32 {
        self.total_gap_count
    }

    /// Get count of gap ranges detected
    pub fn gap_count(&self) -> usize {
        self.gaps.len()
    }

    /// Reset state
    pub fn reset(&mut self) {
        self.last_sequence = None;
        self.gaps.clear();
        self.total_gap_count = 0;
    }

    /// Check if a specific sequence number is in a gap
    pub fn is_in_gap(&self, seq_num: u32) -> bool {
        self.gaps.iter().any(|&(start, end)| seq_num >= start && seq_num <= end)
    }
}

impl Default for GapDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_gaps() {
        let mut detector = GapDetector::new();
        for i in 0..100 {
            detector.process(i);
        }
        assert_eq!(detector.total_gaps(), 0);
        assert!(detector.gaps().is_empty());
    }

    #[test]
    fn test_single_gap() {
        let mut detector = GapDetector::new();
        detector.process(1);
        detector.process(2);
        detector.process(5); // gap: 3, 4
        detector.process(6);

        assert_eq!(detector.total_gaps(), 2);
        assert_eq!(detector.gap_count(), 1);
        assert_eq!(detector.gaps()[0], (3, 4));
    }

    #[test]
    fn test_multiple_gaps() {
        let mut detector = GapDetector::new();
        detector.process(1);
        detector.process(5); // gap: 2-4 (3 messages)
        detector.process(10); // gap: 6-9 (4 messages)
        detector.process(11);

        assert_eq!(detector.total_gaps(), 7);
        assert_eq!(detector.gap_count(), 2);
        assert_eq!(detector.gaps()[0], (2, 4));
        assert_eq!(detector.gaps()[1], (6, 9));
    }

    #[test]
    fn test_is_in_gap() {
        let mut detector = GapDetector::new();
        detector.process(1);
        detector.process(5);
        detector.process(10);

        assert!(detector.is_in_gap(2));
        assert!(detector.is_in_gap(3));
        assert!(detector.is_in_gap(4));
        assert!(!detector.is_in_gap(1));
        assert!(!detector.is_in_gap(5));
        assert!(detector.is_in_gap(6));
        assert!(detector.is_in_gap(9));
        assert!(!detector.is_in_gap(10));
    }

    #[test]
    fn test_reset() {
        let mut detector = GapDetector::new();
        detector.process(1);
        detector.process(5);
        assert_eq!(detector.total_gaps(), 3);

        detector.reset();
        assert_eq!(detector.total_gaps(), 0);
        assert!(detector.gaps().is_empty());
    }
}
