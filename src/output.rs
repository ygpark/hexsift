use crate::cli::ColorChoice;
/// Utilities for formatting binary data as hexadecimal output
use colored::*;
use std::io::IsTerminal;

pub struct OutputFormatter;

impl OutputFormatter {
    /// Format bytes as hexadecimal string with given separator
    pub fn format_bytes_as_hex(bytes: &[u8], separator: &str) -> String {
        bytes
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(separator)
    }

    /// Format offset with proper padding based on file size
    pub fn format_offset(offset: u64, hex_offset_length: usize) -> String {
        format!("{:0width$X}h", offset, width = hex_offset_length)
    }

    /// Calculate the number of digits needed for hex offset display
    pub fn calculate_hex_offset_length(file_size: u64) -> usize {
        format!("{:X}", file_size).len()
    }

    /// Print a line with optional offset
    pub fn print_line(offset: u64, hex_data: &str, show_offset: bool, hex_offset_length: usize) {
        Self::print_line_with_silent(offset, hex_data, show_offset, hex_offset_length, false);
    }

    /// Print a line with optional offset and silent mode
    pub fn print_line_with_silent(
        offset: u64,
        hex_data: &str,
        show_offset: bool,
        hex_offset_length: usize,
        silent: bool,
    ) {
        if silent {
            return; // Skip output when in silent mode
        }

        Self::print_line_with_color(
            offset,
            hex_data,
            show_offset,
            hex_offset_length,
            crate::color_context::get_color_choice(),
        )
    }

    /// Print a line with optional offset and color support, with match highlighting
    pub fn print_line_with_match_highlight(
        offset: u64,
        hex_data: &str,
        show_offset: bool,
        hex_offset_length: usize,
        color_choice: &ColorChoice,
        match_start: Option<usize>,
        match_length: Option<usize>,
    ) {
        Self::print_line_with_match_highlight_silent(
            offset,
            hex_data,
            show_offset,
            hex_offset_length,
            color_choice,
            match_start,
            match_length,
            false,
        );
    }

    /// Print a line with optional offset and color support, with match highlighting and silent mode
    pub fn print_line_with_match_highlight_silent(
        offset: u64,
        hex_data: &str,
        show_offset: bool,
        hex_offset_length: usize,
        color_choice: &ColorChoice,
        match_start: Option<usize>,
        match_length: Option<usize>,
        silent: bool,
    ) {
        if silent {
            return; // Skip output when in silent mode
        }

        let should_use_color = match color_choice {
            ColorChoice::Always => true,
            ColorChoice::Never => false,
            ColorChoice::Auto => std::io::stdout().is_terminal(),
        };

        if show_offset {
            let offset_str = Self::format_offset(offset, hex_offset_length);

            if should_use_color {
                println!(
                    "{} : {}",
                    offset_str.cyan().bold(),
                    Self::colorize_hex_data_with_match(hex_data, match_start, match_length)
                );
            } else {
                println!("{} : {}", offset_str, hex_data);
            }
        } else {
            if should_use_color {
                println!(
                    "{}",
                    Self::colorize_hex_data_with_match(hex_data, match_start, match_length)
                );
            } else {
                println!("{}", hex_data);
            }
        }
    }

    /// Print a line with optional offset and color support
    pub fn print_line_with_color(
        offset: u64,
        hex_data: &str,
        show_offset: bool,
        hex_offset_length: usize,
        color_choice: &ColorChoice,
    ) {
        Self::print_line_with_match_highlight(
            offset,
            hex_data,
            show_offset,
            hex_offset_length,
            color_choice,
            None,
            None,
        );
    }

    /// Apply colors to hex data with match highlighting
    fn colorize_hex_data_with_match(
        hex_data: &str,
        match_start: Option<usize>,
        match_length: Option<usize>,
    ) -> String {
        let bytes: Vec<&str> = hex_data.split_whitespace().collect();

        bytes
            .iter()
            .enumerate()
            .map(|(i, byte)| {
                // Check if this byte is part of a match
                let is_match = if let (Some(start), Some(len)) = (match_start, match_length) {
                    i >= start && i < start + len
                } else {
                    false
                };

                if is_match {
                    // Highlight matches with dark red color
                    byte.red().bold().to_string()
                } else {
                    // No color for non-matched bytes
                    byte.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Apply colors to hex data
    #[allow(dead_code)]
    fn colorize_hex_data(hex_data: &str) -> String {
        hex_data
            .split_whitespace()
            .map(|byte| {
                match u8::from_str_radix(byte, 16) {
                    Ok(b) => match b {
                        0x00 => byte.bright_black().to_string(), // NULL bytes - dark gray
                        0x20..=0x7E => byte.green().to_string(), // Printable ASCII - green
                        0xFF => byte.bright_red().bold().to_string(), // 0xFF - bright red
                        0x01..=0x1F | 0x7F..=0x9F => byte.yellow().to_string(), // Control characters - yellow
                        _ => byte.blue().to_string(), // Other bytes - blue
                    },
                    Err(_) => byte.to_string(), // Fallback for non-hex data
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Format a line with offset (returns a string instead of printing)
    pub fn format_line_with_offset(
        offset: u64,
        hex_data: &str,
        hex_offset_length: usize,
    ) -> String {
        format!(
            "{} : {}",
            Self::format_offset(offset, hex_offset_length),
            hex_data
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes_as_hex() {
        let bytes = vec![0x48, 0x65, 0x6C, 0x6C, 0x6F]; // "Hello"
        let result = OutputFormatter::format_bytes_as_hex(&bytes, " ");
        assert_eq!(result, "48 65 6C 6C 6F");
    }

    #[test]
    fn test_format_bytes_with_different_separators() {
        let bytes = vec![0x00, 0xFF, 0x42];

        let with_space = OutputFormatter::format_bytes_as_hex(&bytes, " ");
        assert_eq!(with_space, "00 FF 42");

        let with_dash = OutputFormatter::format_bytes_as_hex(&bytes, "-");
        assert_eq!(with_dash, "00-FF-42");

        let no_separator = OutputFormatter::format_bytes_as_hex(&bytes, "");
        assert_eq!(no_separator, "00FF42");
    }

    #[test]
    fn test_format_offset() {
        let result = OutputFormatter::format_offset(0x1234, 6);
        assert_eq!(result, "001234h");
    }

    #[test]
    fn test_calculate_hex_offset_length() {
        assert_eq!(OutputFormatter::calculate_hex_offset_length(0xFF), 2);
        assert_eq!(OutputFormatter::calculate_hex_offset_length(0x1000), 4);
        assert_eq!(OutputFormatter::calculate_hex_offset_length(0x100000), 6);
    }
}
