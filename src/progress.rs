use std::io::{self, Write};
use std::time::{Duration, Instant};

/// Progress indicator for file processing
pub struct ProgressIndicator {
    start_time: Instant,
    last_update: Instant,
    total_bytes: u64,
    processed_bytes: u64,
    enabled: bool,
    show_progress: bool,
}

impl ProgressIndicator {
    /// Create a new progress indicator
    ///
    /// # Arguments
    ///
    /// * `total_bytes` - Total number of bytes to process
    /// * `show_progress` - Whether to show progress updates
    pub fn new(total_bytes: u64, show_progress: bool) -> Self {
        let now = Instant::now();
        Self {
            start_time: now,
            last_update: now,
            total_bytes,
            processed_bytes: 0,
            enabled: show_progress && total_bytes > 0,
            show_progress,
        }
    }

    /// Update progress with the number of bytes processed
    ///
    /// # Arguments
    ///
    /// * `bytes_processed` - Additional bytes processed since last update
    pub fn update(&mut self, bytes_processed: u64) {
        self.processed_bytes = self.processed_bytes.saturating_add(bytes_processed);

        if !self.enabled {
            return;
        }

        let now = Instant::now();

        // Update progress every 100ms
        if now.duration_since(self.last_update) >= Duration::from_millis(100) {
            self.display_progress();
            self.last_update = now;
        }
    }

    /// Set the progress to completed
    pub fn finish(&mut self) {
        if !self.enabled {
            return;
        }

        self.processed_bytes = self.total_bytes;
        self.display_progress();
        eprintln!(); // New line after progress
    }

    /// Display current progress
    fn display_progress(&self) {
        if !self.show_progress {
            return;
        }

        let elapsed = self.start_time.elapsed();
        let bytes_per_sec = if elapsed.as_secs_f64() > 0.0 {
            self.processed_bytes as f64 / elapsed.as_secs_f64()
        } else {
            0.0
        };

        let (rate_value, rate_unit) = format_bytes_per_second(bytes_per_sec);
        let (processed_value, processed_unit) = format_bytes(self.processed_bytes);

        if self.total_bytes > 0 {
            // Known file size - show percentage progress bar
            let percentage = (self.processed_bytes as f64 / self.total_bytes as f64 * 100.0) as u32;
            let (total_value, total_unit) = format_bytes(self.total_bytes);

            // Progress bar
            let bar_width = 20;
            let filled = (percentage as usize * bar_width) / 100;
            let empty = bar_width - filled;

            eprint!(
                "\r[{}{}] {}% ({:.1} {}/{:.1} {}) {:.1} {}/s",
                "=".repeat(filled),
                " ".repeat(empty),
                percentage,
                processed_value,
                processed_unit,
                total_value,
                total_unit,
                rate_value,
                rate_unit
            );
        } else {
            // Unknown file size - show spinner style
            let spinner_chars = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧'];
            let spinner_idx = (elapsed.as_millis() / 100) % spinner_chars.len() as u128;
            let spinner = spinner_chars[spinner_idx as usize];

            eprint!(
                "\r{} Processing... ({:.1} {}) {:.1} {}/s",
                spinner,
                processed_value,
                processed_unit,
                rate_value,
                rate_unit
            );
        }

        let _ = io::stderr().flush();
    }

    /// Create a progress indicator that's always disabled
    pub fn disabled() -> Self {
        Self {
            start_time: Instant::now(),
            last_update: Instant::now(),
            total_bytes: 0,
            processed_bytes: 0,
            enabled: false,
            show_progress: false,
        }
    }

    /// Create a progress indicator for unknown file sizes (e.g., forensic images)
    /// This enables silent mode even when total_bytes is unknown
    pub fn new_silent_only(show_progress: bool) -> Self {
        let now = Instant::now();
        Self {
            start_time: now,
            last_update: now,
            total_bytes: 0,
            processed_bytes: 0,
            enabled: show_progress, // Enable for silent mode even with unknown size
            show_progress,
        }
    }

    /// Check if progress should be shown based on output destination
    pub fn should_show_progress() -> bool {
        use std::io::IsTerminal;
        io::stderr().is_terminal()
    }

    /// Check if output should be silenced (when progress is enabled)
    pub fn is_silent(&self) -> bool {
        self.enabled
    }
}

/// Format bytes with appropriate unit
fn format_bytes(bytes: u64) -> (f64, &'static str) {
    const UNITS: &[(&str, u64)] = &[
        ("TB", 1024_u64.pow(4)),
        ("GB", 1024_u64.pow(3)),
        ("MB", 1024_u64.pow(2)),
        ("KB", 1024),
        ("B", 1),
    ];

    for &(unit, divisor) in UNITS {
        if bytes >= divisor {
            return (bytes as f64 / divisor as f64, unit);
        }
    }

    (0.0, "B")
}

/// Format bytes per second with appropriate unit
fn format_bytes_per_second(bytes_per_sec: f64) -> (f64, &'static str) {
    const UNITS: &[(&str, f64)] = &[
        ("TB/s", 1024.0 * 1024.0 * 1024.0 * 1024.0), // 1024^4
        ("GB/s", 1024.0 * 1024.0 * 1024.0),          // 1024^3
        ("MB/s", 1024.0 * 1024.0),                   // 1024^2
        ("KB/s", 1024.0),
        ("B/s", 1.0),
    ];

    for &(unit, divisor) in UNITS {
        if bytes_per_sec >= divisor {
            return (bytes_per_sec / divisor, unit);
        }
    }

    (0.0, "B/s")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), (0.0, "B"));
        assert_eq!(format_bytes(512), (512.0, "B"));
        assert_eq!(format_bytes(1024), (1.0, "KB"));
        assert_eq!(format_bytes(1536), (1.5, "KB"));
        assert_eq!(format_bytes(1024 * 1024), (1.0, "MB"));
        assert_eq!(format_bytes(1024 * 1024 * 1024), (1.0, "GB"));
    }

    #[test]
    fn test_format_bytes_per_second() {
        assert_eq!(format_bytes_per_second(0.0), (0.0, "B/s"));
        assert_eq!(format_bytes_per_second(512.0), (512.0, "B/s"));
        assert_eq!(format_bytes_per_second(1024.0), (1.0, "KB/s"));
        assert_eq!(format_bytes_per_second(1536.0), (1.5, "KB/s"));
        assert_eq!(format_bytes_per_second(1024.0 * 1024.0), (1.0, "MB/s"));
    }

    #[test]
    fn test_progress_indicator_creation() {
        let progress = ProgressIndicator::new(1000, true);
        assert_eq!(progress.total_bytes, 1000);
        assert_eq!(progress.processed_bytes, 0);
        assert!(progress.enabled);

        let disabled_progress = ProgressIndicator::disabled();
        assert!(!disabled_progress.enabled);
    }

    #[test]
    fn test_progress_update() {
        let mut progress = ProgressIndicator::new(1000, false); // Don't show to avoid stderr output in tests
        progress.update(250);
        assert_eq!(progress.processed_bytes, 250);

        progress.update(750);
        assert_eq!(progress.processed_bytes, 1000);
    }

    #[test]
    fn test_progress_overflow() {
        let mut progress = ProgressIndicator::new(100, false);
        progress.update(150); // More than total
        assert_eq!(progress.processed_bytes, 150); // Should not clamp, but saturate on add
    }
}
