use crate::cli::Cli;
use crate::error::{BingrepError, Result};

/// Configuration constants and defaults for bingrep
#[derive(Debug, Clone)]
pub struct Config {
    pub buffer_size: usize,
    pub buffer_padding: usize,
    pub max_line_width: usize,
    pub min_line_width: usize,
    pub max_file_size: u64,      // Maximum file size to process
    pub max_memory_usage: usize, // Maximum memory usage in bytes
}

impl Default for Config {
    fn default() -> Self {
        Self {
            // Optimized for modern NVMe SSDs (1-4MB range) while maintaining compatibility
            buffer_size: 4 * 1024 * 1024, // 4MB for optimal NVMe/SSD read performance
            buffer_padding: 8192,         // 8KB padding for better pattern boundary handling
            max_line_width: 8192,         // Maximum bytes per line
            min_line_width: 1,            // Minimum bytes per line
            max_file_size: 100 * 1024 * 1024 * 1024u64, // 100GB maximum file size
            max_memory_usage: 1024 * 1024 * 1024, // 1GB maximum memory usage
        }
    }
}

impl Config {
    /// Validate all input parameters from CLI
    pub fn validate_cli(&self, cli: &Cli) -> Result<()> {
        // Validate line width
        if !self.validate_width(cli.line_width) {
            return Err(BingrepError::InvalidWidth(cli.line_width));
        }

        // Validate chunk size doesn't exceed memory limits
        if cli.chunk_size > self.max_memory_usage / 4 {
            return Err(BingrepError::InvalidPattern(format!(
                "Chunk size {} too large, maximum allowed: {}",
                cli.chunk_size,
                self.max_memory_usage / 4
            )));
        }

        // Validate limit (must be non-negative, but usize ensures this)
        // No additional validation needed for limit

        // Validate position (must be non-negative, but u64 ensures this)
        // No additional validation needed for position

        Ok(())
    }

    pub fn validate_width(&self, width: usize) -> bool {
        width >= self.min_line_width && width <= self.max_line_width
    }

    pub fn get_buffer_size(&self, width: usize) -> usize {
        // Use smaller buffer size for small line widths to avoid memory waste
        if width < 1024 {
            std::cmp::max(width * 4, 4096) // At least 4KB, or 4x the line width
        } else {
            self.buffer_size
        }
    }

    pub fn get_min_width(&self) -> usize {
        self.min_line_width
    }

    pub fn get_max_width(&self) -> usize {
        self.max_line_width
    }

    /// Validate file size doesn't exceed limits
    pub fn validate_file_size(&self, size: u64) -> Result<()> {
        if size > self.max_file_size {
            return Err(BingrepError::InvalidPattern(format!(
                "File size {} bytes exceeds maximum allowed: {} bytes",
                size, self.max_file_size
            )));
        }
        Ok(())
    }

    /// Get maximum file size limit
    pub fn get_max_file_size(&self) -> u64 {
        self.max_file_size
    }

    /// Get maximum memory usage limit
    pub fn get_max_memory_usage(&self) -> usize {
        self.max_memory_usage
    }
}
